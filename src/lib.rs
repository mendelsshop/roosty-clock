#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(clippy::use_self, rust_2018_idioms)]
#![allow(clippy::multiple_crate_versions, clippy::module_name_repetitions)]

use alarm::Alarm;
use eframe::{
    egui::{self, CentralPanel, Grid, Layout, ScrollArea, TopBottomPanel, Visuals, Window},
    epaint::vec2,
};

/// structures for alarms
pub mod alarm;

/// implementation of alarm editing for egui
pub mod alarm_edit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeOfDay {
    #[default]
    AM,
    PM,
}

#[doc(hidden)]
#[derive(Default)]
pub struct App {
    /// if true, the app will use the dark theme (default)
    dark_theme: bool,
    alarms: Vec<Alarm>,
    time_format: String,
    in_config: bool,
    adding_alarm: bool,
    alarm: Alarm,
}

impl App {
    #[must_use] pub fn new(time_format: String) -> Self {
        Self {
            dark_theme: true,
            alarms: Vec::new(),
            time_format,
            in_config: false,
            adding_alarm: false,
            alarm: Alarm::default(),
        }
    }
    // TODO: remove staticly set vec2s
    pub(crate) fn render_alarm_creation(&mut self, ctx: &egui::Context) {
        Window::new("adding alarm")
            // .fixed_size(vec2(190.0, 80.0))
            .resize(|resize| resize.resizable(false).max_size(vec2(190.0, 50.0)))
            .show(ctx, |ui| {
                self.alarm.edit_alarm(ui);
                ui.horizontal(|ui| {
                    if ui.button("done").clicked() {
                        self.alarms.push(std::mem::take(&mut self.alarm));
                        self.adding_alarm = false;
                    }
                    if ui.button("cancel").clicked() {
                        self.adding_alarm = false;
                    }
                });
            });
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
                ui.checkbox(&mut self.dark_theme, "Dark Theme");
                // TODO: fix allignment
                ui.centered_and_justified(|ui| {
                    ui.label(format!("Time: {}", chrono::Local::now().naive_local()));
                });
                ui.with_layout(Layout::right_to_left(eframe::emath::Align::Min), |ui| {
                    if ui.button("⚙").clicked() {
                        self.in_config = true;
                    }
                });
            });
        });
    }
}

impl eframe::App for App {
    // TODO: extract into different functions
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // an alarm need to keep state of its been rang today
        self.alarms
            .iter_mut()
            .filter(|alarm| alarm.enabled && alarm.next_alarm <= chrono::Local::now().naive_local())
            .for_each(Alarm::ring);
        ctx.request_repaint();
        if self.dark_theme {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }
        // config window
        if self.in_config {
            self.render_settings(ctx);
        }
        // alarm creation
        if self.adding_alarm {
            self.render_alarm_creation(ctx);
        }
        // header
        self.render_header(ctx);
        // show all alarms
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("+").clicked() {
                self.adding_alarm = true;
            }
            // scrollable grid - dynamic each cell is an alarm that get rendered with Alarm::render_alarm
            // needs to be fixed it messes with Alarm::edit_alarm via Alarm::render_alarm
            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("alarms").show(ui, |ui| {
                    self.alarms.iter_mut().for_each(|alarm| {
                        alarm.render_alarm(&self.time_format, ui);
                    });
                    //  check if were at end of a row
                    if ui.available_size_before_wrap().x < ui.available_size().x {
                        ui.end_row();
                    }
                });
            });
        });
    }
}
