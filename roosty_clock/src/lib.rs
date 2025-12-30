#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(clippy::use_self, rust_2018_idioms)]
#![allow(clippy::multiple_crate_versions, clippy::module_name_repetitions)]

use std::{collections::HashMap, io::BufReader, mem};

use alarm_edit::EditingState;
use chrono::Timelike;
use config::{Config, Sound, Theme};
use eframe::egui::{
    self, Button, CentralPanel, Context, Grid, Layout, ScrollArea, TopBottomPanel, Window,
};
use interprocess::local_socket::{RecvHalf, SendHalf};

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
    recv: BufReader<RecvHalf>,
    alarm_edits: HashMap<u64, AlarmBuilder>,
    send: SendHalf,
}

pub fn send_to_server(w: &mut SendHalf, message: roosty_clockd::ClientMessage) -> Result<(), ()> {
    let bytes = bitcode::serialize(&message).map_err(|_| ())?;
    // bytes.push(b'\n');

    roosty_clockd::write(w, &bytes).map_err(|_| ()).map(|_| ())
}
pub fn recieve_from_server(
    conn: &mut BufReader<RecvHalf>,
) -> Result<roosty_clockd::ServerMessage, ()> {
    let mut bytes = Vec::new();
    roosty_clockd::read(conn, &mut bytes).map_err(|_e| {
        // println!("e: {e}");
        ();
    })?;
    // println!("got {bytes:?}");
    // bytes.pop();
    // println!("got {bytes:?}");

    bitcode::deserialize(&bytes).map_err(|_e| {
        // println!("e1: {e}");
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
        send: SendHalf,
        recv: BufReader<RecvHalf>,
        sounds: HashMap<String, roosty_clockd_config::Sound>,
        alarms: HashMap<u64, roosty_clockd_config::Alarm>,
    ) -> Self {
        Self {
            alarm_edits: HashMap::new(),
            config: Config::load(Config::config_path()),
            sounds,
            alarms,
            send,
            recv,
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
        let collect = self
            .alarms
            .keys()
            .copied()
            .enumerate()
            .skip(skip)
            .collect::<Vec<_>>();
        for (_i, id) in collect {
            if ui.button("x").on_hover_text("delete alarm").clicked() {
                // handle if alarm is currently active
                send_to_server(
                    &mut self.send,
                    roosty_clockd::ClientMessage::RemoveAlarm(id),
                );

                self.alarms.remove(&id);
                self.list_alarms(ui, 0, ctx);
                break;
            }

            let _alarm_changed = self.render_alarm(id, ui, ctx);
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
                        &mut self.send,
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
            if ui.button("+").on_hover_text("add alarm").clicked() {
                send_to_server(&mut self.send, roosty_clockd::ClientMessage::GetNewUID);
                if let Ok(ServerMessage::UID(id)) = recieve_from_server(&mut self.recv) {
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

            let mut old_alarm_edits = HashMap::new();
            mem::swap(&mut old_alarm_edits, &mut self.alarm_edits);
            self.alarm_edits =
                HashMap::from_iter(old_alarm_edits.into_iter().filter_map(|(id, mut alarm)| {
                    match alarm.render_alarm_editor(ctx, &self.sounds) {
                        EditingState::Cancelled => None,
                        EditingState::Editing => Some((id, alarm)),
                        EditingState::Done(alarm) => {
                            self.alarms.insert(id, alarm.clone());
                            send_to_server(
                                &mut self.send,
                                roosty_clockd::ClientMessage::AddAlarm(roosty_clockd::Alarm {
                                    name: alarm.name,
                                    time: alarm.time,
                                    volume: alarm.volume,
                                    sound: alarm.sound,
                                    id,
                                }),
                            );
                            None
                        }
                    }
                }));
        });
    }
}
