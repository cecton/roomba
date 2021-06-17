use super::Room;
use futures::channel::mpsc;
use futures::select;
use futures::stream::StreamExt;
use roomba::{api, Client};
use std::thread;
use std::{error::Error, io};
use termion::input::TermRead;
use termion::{event::Key, raw::IntoRawMode, screen::AlternateScreen};
use tui::widgets::ListState;
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem},
    Terminal,
};

macro_rules! safe_json_traversal {
    ($var:expr => $key:expr => $($tokens:tt)=>+) => {{
        let object = safe_json_traversal!($var => $key);

        safe_json_traversal!(with_result object => $($tokens)=>+)
    }};
    ($var:expr => [$index:expr]) => {{
        $var.as_array()
            .ok_or_else(|| format!("Not an object: {}", stringify!($var)))
            .and_then(|x| x.get($index)
                .ok_or_else(|| format!("Key {:?} not found.", stringify!($index))))
    }};
    ($var:expr => $key:expr) => {{
        $var.as_object()
            .ok_or_else(|| format!("Not an object: {}", stringify!($var)))
            .and_then(|x| x.get(stringify!($key))
                .ok_or_else(|| format!("Key {:?} not found.", stringify!($key))))
    }};
    (with_result $var:expr => $key:expr => $($tokens:tt)=>+) => {{
        let object = safe_json_traversal!(with_result $var => $key);

        safe_json_traversal!(with_result object => $($tokens)=>+)
    }};
    (with_result $var:expr => $key:expr) => {{
        $var.and_then(|object| safe_json_traversal!(object => $key))
    }};
}

type Map = (String, String);

pub struct App<'a> {
    client: &'a mut Client,
    events: Vec<(Vec<String>, String)>,
    items: Vec<(Room, bool)>,
    map: Option<Map>,
    running: bool,
    state: ListState,
}

impl<'a> App<'a> {
    pub fn new(client: &'a mut Client, rooms: &[Room], map: Option<Map>) -> Self {
        let mut rooms = rooms.to_vec();
        rooms.sort_by(|a, b| a.name.cmp(&b.name));

        let mut state = ListState::default();
        state.select(Some(0));

        Self {
            client,
            events: vec![],
            items: rooms.into_iter().map(|x| (x, false)).collect(),
            map,
            running: true,
            state,
        }
    }

    fn log(&mut self, message: Vec<String>, subject: &str) {
        self.events.insert(0, (message, subject.to_string()));
    }

    fn log_event(&mut self, event: paho_mqtt::message::Message) -> serde_json::Result<()> {
        let payload = serde_json::from_str::<serde_json::Value>(&event.payload_str())?;

        if let Ok(battery) = safe_json_traversal!(payload => state => reported => batPct) {
            self.log(vec![format!("{}%", battery)], "BATTERY");
        }

        if let Ok(last_command) = safe_json_traversal!(payload => state => reported => lastCommand)
        {
            let pretty = serde_json::to_string_pretty(last_command)?;
            self.log(pretty.lines().map(Into::into).collect(), "LAST_COMMAND");
        }

        if let Ok(pmaps) = safe_json_traversal!(payload => state => reported => pmaps) {
            let pretty = serde_json::to_string_pretty(pmaps)?;
            self.log(pretty.lines().map(Into::into).collect(), "PMAPS");

            if self.map.is_none() {
                if let Some(array) = pmaps.as_array() {
                    if let Some(object) = array.get(0).and_then(|x| x.as_object()) {
                        if let Some((pmap_id, user_pmapv_id)) = object.iter().next() {
                            if let Some(user_pmapv_id) = user_pmapv_id.as_str() {
                                self.map = Some((pmap_id.to_string(), user_pmapv_id.to_string()));
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn command(&mut self, command: Vec<String>) {
        self.events.insert(0, (command, "command".to_string()));
    }

    pub async fn main_loop(mut self) -> Result<Option<Map>, Box<dyn Error>> {
        // Terminal initialization
        let stdout = io::stdout().into_raw_mode()?;
        let stdout = AlternateScreen::from(stdout);
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let mut events = events();

        while self.running {
            self.render(&mut terminal)?;

            select! {
                ev = events.next() => {
                    if let Some(event) = ev {
                        self.handle_event(event).await;
                    } else {
                        break
                    }
                },
                ev = self.client.events.next() => {
                    if let Some(ev) = ev.flatten() {
                        // TODO: log error in logger, not in user interface
                        let _ = self.log_event(ev);
                    } else {
                        break
                    }
                },
                complete => break,
            }
        }

        Ok(self.map)
    }

    fn render<B: tui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> std::io::Result<()> {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(25), Constraint::Percentage(75)].as_ref())
                .split(f.size());

            let items: Vec<ListItem> = self
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
            f.render_stateful_widget(items, chunks[0], &mut self.state);

            let events: Vec<ListItem> = self
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

        Ok(())
    }

    async fn handle_event(&mut self, event: termion::event::Key) {
        match event {
            Key::Char('q') => {
                self.running = false;
            }
            Key::Down => self.next(),
            Key::Up => self.previous(),
            Key::Char(' ') => self.toggle(),
            Key::Char('+') => self.move_up(),
            Key::Char('-') => self.move_down(),
            Key::Char('\n') => self.start_job().await,
            _ => {}
        }
    }

    async fn start_job(&mut self) {
        if let Some((pmap_id, user_pmapv_id)) = self.map.clone() {
            let rooms: Vec<_> = self
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
            self.command(
                rooms
                    .iter()
                    .enumerate()
                    .map(|(i, room)| format!("{:>2}. {}", i + 1, room))
                    .collect(),
            );
            let command = api::Command::Start;
            let extra = api::Extra::StartRegions {
                ordered: 1,
                pmap_id,
                user_pmapv_id,
                regions: rooms.iter().map(|x| x.region.clone()).collect(),
            };
            let message = api::Message::new_command(command, Some(extra));
            self.client
                .send_message(&message)
                .await
                .unwrap_or_else(|err| {
                    self.log(
                        vec![
                            "Could not send message to the device:".to_string(),
                            err.to_string(),
                        ],
                        "ERROR",
                    );
                });
        } else {
            self.log(
                vec!["pmap_id and user_pmapv_id not set!".to_string()],
                "ERROR",
            );
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.is_empty() || i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.is_empty() {
                    0
                } else if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn toggle(&mut self) {
        if let Some(i) = self.state.selected() {
            if self.items.len() > i {
                self.items[i].1 ^= true;
                self.items.sort_by_key(|x| !x.1);
            }
        }
    }

    fn move_up(&mut self) {
        if let Some(i) = self.state.selected() {
            if i > 0 && self.items[i].1 && self.items[i - 1].1 {
                let elem = self.items[i].clone();
                self.items[i] = self.items[i - 1].clone();
                self.items[i - 1] = elem;
                self.state.select(Some(i - 1));
            }
        }
    }

    fn move_down(&mut self) {
        if let Some(i) = self.state.selected() {
            if !self.items.is_empty()
                && i < self.items.len() - 1
                && self.items[i].1
                && self.items[i + 1].1
            {
                let elem = self.items[i].clone();
                self.items[i] = self.items[i + 1].clone();
                self.items[i + 1] = elem;
                self.state.select(Some(i + 1));
            }
        }
    }
}

fn events() -> mpsc::UnboundedReceiver<Key> {
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
