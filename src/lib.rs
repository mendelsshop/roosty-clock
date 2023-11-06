#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(clippy::use_self, rust_2018_idioms)]
#![allow(clippy::multiple_crate_versions, clippy::module_name_repetitions)]

use alarm_edit::EditingState;
use chrono::Timelike;
use config::{Config, Sound, Theme};
use eframe::egui::{
    self, Button, CentralPanel, Context, Grid, Layout, ScrollArea, TopBottomPanel, Window,
};

pub mod config;

/// implementation of alarm editing for egui
pub mod alarm_edit;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlarmBuilder {
    name: String,
    hour: u8,
    minute: u8,
    time_of_day: TimeOfDay,
    sound: String,
    volume: f32,
}

impl Default for AlarmBuilder {
    fn default() -> Self {
        let time = chrono::Local::now().naive_local().time();
        let (ampm, hour) = time.hour12();
        Self {
            name: String::default(),
            hour: if hour == 12 { 0 } else { hour } as u8,
            minute: time.minute() as u8,
            time_of_day: if ampm { TimeOfDay::PM } else { TimeOfDay::AM },
            sound: Sound::get_default_name(),
            volume: 100.0,
        }
    }
}

impl Clock {
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: Config::load(Config::config_path()),
            in_config: false,
            adding_alarm: None,
        }
    }

    fn render_settings(&mut self, ctx: &egui::Context) {
        Window::new("settings ⚙").show(ctx, |ui| {
            if ui.button("x").clicked() {
                self.in_config = false;
            }
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
        for (i, alarm) in self.config.alarms.iter_mut().enumerate().skip(skip) {
            if ui.button("x").on_hover_text("delete alarm").clicked() {
                self.config.alarms.remove(i);
                self.list_alarms(ui, i, ctx);
                break;
            }
            alarm.render_alarm(&self.config.time_format, ui, ctx);
            ui.end_row();
        }
    }
}

impl eframe::App for Clock {
    // TODO: extract into different functions
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // an alarm need to keep state of its been rang today

        ctx.set_visuals(self.config.theme.into());
        // config window
        if self.in_config {
            self.render_settings(ctx);
        }
        // alarm creation
        if let Some(editing) = &mut self.adding_alarm {
            match editing.render_alarm_editor(ctx, &mut self.config.sounds) {
                EditingState::Done(new_alarm) => {
                    self.adding_alarm = None;
                    self.config.alarms.push(new_alarm);
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
                self.adding_alarm = Some(AlarmBuilder::default());
            }

            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("alarms").show(ui, |ui| {
                    self.list_alarms(ui, 0, ctx);
                });
            });
        });
    }
}
