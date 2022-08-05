#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use anyhow::{Context, Result};
use eframe::egui;
use futures::task::LocalSpawnExt;
use futures::task::Poll;
use futures::StreamExt;
use roomba::{api, Client};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;

const ROOMBA_CFG: &str = "roomba.toml";

fn main() -> Result<()> {
    let config_file = dirs::config_dir().unwrap_or_default().join(ROOMBA_CFG);
    let config = if config_file.exists() {
        std::fs::read_to_string(&config_file)
            .map_err(|err| err.to_string())
            .and_then(|ref content| toml::from_str(content).map_err(|err| err.to_string()))
            .unwrap_or_else(|err| {
                eprintln!(
                    "Could not read configuration file `{}`: {}",
                    config_file.display(),
                    err
                );
                Config::default()
            })
    } else {
        Config::default()
    };
    let pmap_id = config.pmap_id.clone().unwrap();
    let user_pmapv_id = config.user_pmapv_id.clone().unwrap();
    let client = futures::executor::block_on(Client::new(
        config
            .hostname
            .as_ref()
            .context("Missing hostname in the configuration")?,
        config
            .username
            .as_ref()
            .context("Missing username in the configuration")?,
        config
            .password
            .as_ref()
            .context("Missing password in the configuration")?,
        0,
    ))?;

    let options = eframe::NativeOptions {
        maximized: true,
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(move |cc| {
            // Force dark mode
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            // Forcer PPP to 2.0
            cc.egui_ctx.set_pixels_per_point(2.0);
            Box::new(MyApp::new(
                client,
                config.rooms.as_slice(),
                pmap_id,
                user_pmapv_id,
            ))
        }),
    );
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    hostname: Option<String>,
    username: Option<String>,
    password: Option<String>,
    pmap_id: Option<String>,
    user_pmapv_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    rooms: Vec<Room>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
    name: String,
    #[serde(flatten)]
    region: api::Region,
}

struct MyApp {
    client: Client,
    pool: futures::executor::LocalPool,
    items: Vec<(Room, bool)>,
    pmap_id: String,
    user_pmapv_id: String,
    log: Rc<RefCell<String>>,
}

impl MyApp {
    pub fn new(mut client: Client, rooms: &[Room], pmap_id: String, user_pmapv_id: String) -> Self {
        let pool = futures::executor::LocalPool::default();
        let mut rooms = rooms.to_vec();
        let log = Rc::new(RefCell::new(String::new()));

        rooms.sort_by(|a, b| a.name.cmp(&b.name));

        /*
        {
            let log = log.clone();
            let spawner = pool.spawner();
            let task = client.events.next();
            pool.spawner().spawn_local(async move {
                loop {
                    //println!("loop");
                    if let Some(ev) = client.events.next().await {
                        println!("{:?}", ev);
                        todo!();
                    } else {
                        futures_timer::Delay::new(std::time::Duration::from_secs(0)).await;
                        //futures::future::pending().await
                    }
                }
            });
        }
        */

        Self {
            client,
            pool,
            items: rooms.into_iter().map(|x| (x, false)).collect(),
            pmap_id,
            user_pmapv_id,
            log,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let MyApp {
            client,
            items,
            log,
            pool,
            pmap_id,
            user_pmapv_id,
            ..
        } = self;
        items.sort_unstable_by_key(|(_, active)| !*active);
        egui::TopBottomPanel::bottom("http_bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    let rooms: Vec<_> = items
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
                    let extra = api::Extra::StartRegions {
                        ordered: 1,
                        pmap_id: pmap_id.to_string(),
                        user_pmapv_id: user_pmapv_id.to_string(),
                        regions: rooms.iter().map(|x| x.region.clone()).collect(),
                    };
                    let message = api::Message::new_command(api::Command::Start, Some(extra));
                    futures::executor::block_on(client.send_message(&message));
                }
                if ui.button("Pause").clicked() {
                    let message = api::Message::new_command(api::Command::Pause, None);
                    futures::executor::block_on(client.send_message(&message));
                }
                if ui.button("Resume").clicked() {
                    let message = api::Message::new_command(api::Command::Resume, None);
                    futures::executor::block_on(client.send_message(&message));
                }
                if ui.button("Stop").clicked() {
                    let message = api::Message::new_command(api::Command::Stop, None);
                    futures::executor::block_on(client.send_message(&message));
                }
                if ui.button("Dock").clicked() {
                    let message = api::Message::new_command(api::Command::Dock, None);
                    futures::executor::block_on(client.send_message(&message));
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    frame.quit();
                }
            });
        });
        // Force the app to refresh so the mqtt feed can be refreshed
        //ctx.request_repaint();
        //pool.try_run_one();
        egui::SidePanel::left("my_left_panel")
            .frame(
                egui::Frame::menu(&Default::default()).inner_margin(egui::style::Margin::same(8.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("my_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .min_col_width(60.0)
                        .striped(true)
                        .show(ui, |ui| {
                            let mut move_up = None;
                            let mut move_down = None;
                            for (i, (room, active)) in items.iter_mut().enumerate() {
                                ui.add(|ui: &mut egui::Ui| ui.label((i + 1).to_string()));
                                ui.toggle_value(active, &room.name);
                                if *active {
                                    ui.horizontal(|ui| {
                                        if ui.button("+").clicked() {
                                            move_up.replace(i);
                                        }
                                        ui.label("/");
                                        if ui.button("-").clicked() {
                                            move_down.replace(i);
                                        }
                                    });
                                } else {
                                    ui.label("");
                                }
                                ui.end_row();
                            }
                            if let Some(i) = move_up {
                                if i > 0 {
                                    items.swap(i, i - 1);
                                }
                            }
                            if let Some(i) = move_down {
                                if i < items.len() - 1 {
                                    items.swap(i, i + 1);
                                }
                            }
                        });
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_enabled_ui(true, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom()
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut *log.borrow_mut())
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(1)
                                    .frame(false)
                                    .font(egui::TextStyle::Monospace),
                            );
                        })
                });
            });
        });
    }
}
