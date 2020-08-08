use futures::channel::mpsc;
use std::thread;

use futures::select;
use futures::stream::StreamExt;
use roomba::{api, Client};
use serde::Deserialize;
use std::{error::Error, io};
use termion::input::TermRead;
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

#[derive(Deserialize)]
struct Config {
    hostname: String,
    username: String,
    password: String,
    pmap_id: String,
    user_pmapv_id: String,
    rooms: Vec<Room>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Room {
    name: String,
    #[serde(flatten)]
    region: api::Region,
}

impl std::fmt::Display for Room {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

struct App {
    items: StatefulList,
    events: Vec<(Vec<String>, String)>,
}

impl App {
    fn new(rooms: Vec<Room>) -> App {
        App {
            items: StatefulList::with_items(rooms.into_iter().map(|x| (x, false)).collect()),
            events: vec![],
        }
    }

    fn update(&mut self, event: paho_mqtt::message::Message) {
        let parser = |input| -> Result<Vec<String>, Box<dyn Error>> {
            let payload = serde_json::from_str::<serde_json::Value>(input)?;
            let status = payload
                .as_object()
                .ok_or("not an object")?
                .get("state")
                .ok_or("missing state")?
                .as_object()
                .ok_or("not an object")?;
            let reported = status
                .get("reported")
                .ok_or("missing reported")?
                .as_object()
                .ok_or("not an object")?;
            let battery = reported.get("batPct").ok_or("missing batPct")?;
            let last_command = reported.get("lastCommand").ok_or("missing lastCommand")?;
            let pmaps = reported.get("pmaps").ok_or("missing pmaps")?;

            Ok(vec![
                format!("battery: {}%", battery),
                format!("last command: {}", last_command),
                format!("pmaps: {}", pmaps),
            ])
        };
        let message = parser(&event.payload_str()).unwrap_or_else(|err| vec![err.to_string()]);
        self.events.insert(0, (message, event.topic().to_string()));
    }

    fn command(&mut self, command: Vec<String>) {
        self.events.insert(0, (command, "command".to_string()));
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let toml_str = std::fs::read_to_string(format!(
        "{}/.config/roomba.toml",
        std::env::var("HOME").unwrap()
    ))
    .unwrap();
    let config: Config = toml::from_str(toml_str.as_str()).unwrap();
    let mut client = Client::new(&config.hostname, &config.username, &config.password, 0).await?;

    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut events = events();

    // App
    let mut app = App::new(config.rooms.clone());

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
                .split(f.size());

            let items: Vec<ListItem> = app
                .items
                .items
                .iter()
                .enumerate()
                .map(|(i, room)| {
                    let lines = vec![Spans::from(format!(
                        "[{:>2}] {}",
                        if room.1 {
                            (i + 1).to_string()
                        } else {
                            String::new()
                        },
                        room.0
                    ))];
                    ListItem::new(lines)
                })
                .collect();
            let items = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Rooms"))
                .highlight_style(Style::default().fg(Color::LightYellow));
            f.render_stateful_widget(items, chunks[0], &mut app.items.state);

            let events: Vec<ListItem> = app
                .events
                .iter()
                .map(|(evt, level)| {
                    let s = match level.as_str() {
                        "CRITICAL" => Style::default().fg(Color::Red),
                        "ERROR" => Style::default().fg(Color::Magenta),
                        "WARNING" => Style::default().fg(Color::Yellow),
                        "INFO" => Style::default().fg(Color::Blue),
                        _ => Style::default(),
                    };
                    let header = Spans::from(vec![Span::styled(level, s)]);
                    let mut lines = vec![
                        Spans::from("-".repeat(chunks[1].width as usize)),
                        header,
                        Spans::from(""),
                    ];
                    lines.extend(evt.iter().map(|x| Spans::from(x.as_str())));
                    ListItem::new(lines)
                })
                .collect();
            let events_list = List::new(events)
                .block(Block::default().borders(Borders::ALL).title("Events"))
                .start_corner(Corner::BottomLeft);
            f.render_widget(events_list, chunks[1]);
        })?;

        let mut handle_ev = |ev| {
            if let Some(key) = ev {
                match key {
                    Key::Char('q') => {
                        return (true, None);
                    }
                    Key::Left => {
                        app.items.unselect();
                    }
                    Key::Down => {
                        app.items.next();
                    }
                    Key::Up => {
                        app.items.previous();
                    }
                    Key::Char(' ') => {
                        app.items.select();
                    }
                    Key::Char('+') => {
                        app.items.move_up();
                    }
                    Key::Char('-') => {
                        app.items.move_down();
                    }
                    Key::Char('\n') => {
                        let rooms: Vec<_> = app
                            .items
                            .items
                            .iter()
                            .filter_map(
                                |(room, selected)| {
                                    if *selected {
                                        Some(room.clone())
                                    } else {
                                        None
                                    }
                                },
                            )
                            .collect();
                        app.command(
                            rooms
                                .iter()
                                .enumerate()
                                .map(|(i, room)| format!("{:>2}. {}", i + 1, room))
                                .collect(),
                        );
                        let command = api::Command::Start;
                        let extra = api::Extra::StartRegions {
                            ordered: 1,
                            pmap_id: config.pmap_id.clone(),
                            user_pmapv_id: config.user_pmapv_id.clone(),
                            regions: rooms.iter().map(|x| x.region.clone()).collect(),
                        };
                        let message = api::Message::new_command(command, Some(extra));
                        return (false, Some(message));
                    }
                    _ => {}
                }
            } else {
                return (true, None);
            }

            (false, None)
        };

        select! {
            ev = events.next() => {
                let (res, message) = handle_ev(ev);
                if res {
                    break;
                }
                if let Some(message) = message {
                    client.send_message(&message).await.unwrap();
                }
            },
            ev = client.events.next() => {
                if let Some(ev) = ev.flatten() {
                    app.update(ev);
                }
            },
            complete => break,
        }
    }

    Ok(())
}

pub fn events() -> mpsc::UnboundedReceiver<Key> {
    let (tx, rx) = mpsc::unbounded();
    thread::spawn(move || {
        let stdin = io::stdin();
        for evt in stdin.keys() {
            if let Ok(key) = evt {
                match tx.unbounded_send(key) {
                    Err(e) if e.is_disconnected() => break,
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                    Ok(()) => {}
                }
            }
        }
    });

    rx
}

use tui::widgets::ListState;

pub struct StatefulList {
    pub state: ListState,
    pub items: Vec<(Room, bool)>,
}

impl StatefulList {
    pub fn with_items(items: Vec<(Room, bool)>) -> StatefulList {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn select(&mut self) {
        if let Some(i) = self.state.selected() {
            self.items[i].1 ^= true;
            self.items.sort_by_key(|x| !x.1);
        }
    }

    pub fn move_up(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 && self.items[i].1 && self.items[i - 1].1 {
                let elem = self.items[i].clone();
                self.items[i] = self.items[i - 1].clone();
                self.items[i - 1] = elem;
                self.state.select(Some(i - 1));
            }
        }
    }

    pub fn move_down(&mut self) {
        if let Some(i) = self.state.selected() {
            if i < self.items.len() - 1 && self.items[i].1 && self.items[i + 1].1 {
                let elem = self.items[i].clone();
                self.items[i] = self.items[i + 1].clone();
                self.items[i + 1] = elem;
                self.state.select(Some(i + 1));
            }
        }
    }
}
