#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(clippy::use_self, rust_2018_idioms)]
#![allow(clippy::multiple_crate_versions, clippy::module_name_repetitions)]

use std::io::BufRead;
use std::{
    collections::HashMap,
    io::{BufReader, Write},
};

use alarm_edit::EditingState;
use base64::Engine;
use chrono::Timelike;
use config::{Config, Sound, Theme};
use eframe::egui::{
    self, Button, CentralPanel, Context, Grid, Layout, ScrollArea, TopBottomPanel, Window,
};
use interprocess::local_socket::Stream;

pub mod config;
use roosty_clockd::{config as roosty_clockd_config, ServerMessage};

/// implementation of alarm editing for egui
pub mod alarm_edit;
pub mod communication;
pub mod widgets;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeOfDay {
    #[default]
    AM,
    PM,
}
pub struct Clock {
    config: Config,
    in_config: bool,
    adding_alarm: Option<AlarmBuilder>,
    alarms: HashMap<u64, roosty_clockd_config::Alarm>,
    sounds: HashMap<String, roosty_clockd_config::Sound>,
    conn: BufReader<Stream>,
}

pub fn send_to_server(
    conn: &mut BufReader<Stream>,
    message: roosty_clockd::ClientMessage,
) -> Result<(), ()> {
    writeln!(
        conn.get_mut(),
        "{}",
        base64::prelude::BASE64_STANDARD
            .encode(toml::to_string(&message).map_err(|_| ())?.as_bytes())
    )
    .map_err(|_| ())
}
pub fn recieve_from_server(
    conn: &mut BufReader<Stream>,
) -> Result<roosty_clockd::ServerMessage, ()> {
    let mut bytes = String::new();
    conn.read_line(&mut bytes).map_err(|_| ())?;
    let s = &base64::prelude::BASE64_STANDARD
        .decode(&bytes[..bytes.len() - 1])
        .unwrap();
    println!("`{}`", str::from_utf8(s).unwrap());
    toml::from_slice::<'_, roosty_clockd::ServerMessage>(s).map_err(|e| {
        print!("{e}");
        ();
    })
}
#[derive(Debug, Clone, PartialEq)]
pub struct AlarmBuilder {
    name: String,
    hour: u8,
    minute: u8,
    time_of_day: TimeOfDay,
    sound: String,
    volume: f32,
    id: u64,
}

impl Default for AlarmBuilder {
    fn default() -> Self {
        let time = chrono::Local::now().naive_local().time();
        let (ampm, hour) = time.hour12();
        let minute = time.minute();
        Self {
            name: String::default(),
            hour: hour as u8,
            minute: minute as u8,
            time_of_day: if ampm { TimeOfDay::PM } else { TimeOfDay::AM },
            sound: Sound::get_default_name(),
            volume: 100.0,
            id: 0,
        }
    }
}

impl Clock {
    #[must_use]
    pub fn new(
        conn: BufReader<Stream>,
        sounds: HashMap<String, roosty_clockd_config::Sound>,
        alarms: HashMap<u64, roosty_clockd_config::Alarm>,
    ) -> Self {
        Self {
            config: Config::load(Config::config_path()),
            sounds,
            alarms,
            conn,
            in_config: false,
            adding_alarm: None,
        }
    }

    fn render_settings(&mut self, ctx: &egui::Context) {
        Window::new("settings ⚙")
            .open(&mut self.in_config)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label("Default Sound");
                AlarmBuilder::render_sound_selector_editor(
                    &mut self.config.default_sound,
                    ui,
                    &self.sounds,
                );
                self.config.save(Config::config_path());
            });
    }

    fn render_header(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("time_and_ctrl").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let theme_btn = ui.add(Button::new({
                    if self.config.theme == Theme::Dark {
                        "🌞"
                    } else {
                        "🌙"
                    }
                }));
                if theme_btn.clicked() {
                    self.config.theme = !self.config.theme;
                }
                // TODO: fix allignment
                ui.centered_and_justified(|ui| {
                    ui.label(format!(
                        "Time: {}",
                        chrono::Local::now()
                            .naive_local()
                            .format(&self.config.time_format)
                    ));
                });
                ui.with_layout(Layout::right_to_left(eframe::emath::Align::Min), |ui| {
                    if ui.button("⚙").on_hover_text("settings").clicked() {
                        self.in_config = true;
                    }
                });
            });
        });
    }

    fn list_alarms(&mut self, ui: &mut egui::Ui, skip: usize, ctx: &Context) {
        for (i, (id, _alarm)) in self.alarms.iter().enumerate().skip(skip) {
            if ui.button("x").on_hover_text("delete alarm").clicked() {
                // handle if alarm is currently active
                send_to_server(
                    &mut self.conn,
                    roosty_clockd::ClientMessage::RemoveAlarm(*id),
                );

                // write changes to disk
                self.save();
                self.list_alarms(ui, i, ctx);
                break;
            }

            // let alarm_changed = self.render_alarm(alarm, ui, ctx);
            // if alarm_changed {
            //     // even if alarm.enabled is false or alarm.rang_today is false
            //     // it may have been rang today or enabled but the user changed the alarm
            //     self.save();
            //     self.list_alarms(ui, i, ctx);
            //     break;
            // }
            ui.end_row();
        }
    }

    fn save(&self) {
        self.config.save(Config::config_path());
    }
}

impl eframe::App for Clock {
    // TODO: extract into different functions
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ctx.request_repaint();
        // an alarm need to keep state of its been rang today

        ctx.set_visuals(self.config.theme.into());
        // config window
        if self.in_config {
            self.render_settings(ctx);
        }
        // alarm creation
        if let Some(editing) = &mut self.adding_alarm {
            match editing.render_alarm_editor(ctx, &self.sounds) {
                EditingState::Done(new_alarm) => {
                    self.adding_alarm = None;
                    self.alarms.insert(new_alarm.id, new_alarm.clone());
                    send_to_server(
                        &mut self.conn,
                        roosty_clockd::ClientMessage::AddAlarm(roosty_clockd::Alarm {
                            name: new_alarm.name,
                            time: new_alarm.time,
                            volume: new_alarm.volume,
                            sound: new_alarm.sound,
                            id: new_alarm.id,
                        }),
                    );
                }
                EditingState::Cancelled => {
                    self.adding_alarm = None;
                }
                _ => {}
            }
        }
        // header
        self.render_header(ctx);
        // // show all alarms
        CentralPanel::default().show(ctx, |ui| {
            send_to_server(&mut self.conn, roosty_clockd::ClientMessage::GetNewUID);
            if let Ok(ServerMessage::UID(id)) = recieve_from_server(&mut self.conn) {
                if ui.button("+").on_hover_text("add alarm").clicked() {
                    self.adding_alarm = Some(AlarmBuilder {
                        sound: self.config.default_sound.clone(),
                        id,
                        ..Default::default()
                    });
                }
            }

            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("alarms").show(ui, |ui| {
                    self.list_alarms(ui, 0, ctx);
                });
            });
        });
    }
}
