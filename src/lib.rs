#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(clippy::use_self, rust_2018_idioms)]

use eframe::egui::{self, CentralPanel, Layout, TopBottomPanel, Visuals, Window};

/// represnts an alarm
/// contains the time that the alarm should go of at.
/// as well as an optinal sound and name
struct Alarm {
    time: chrono::NaiveTime,
    name: Option<String>,
    /// there is a default sound
    sound: Option<()>,
    snooze_time: (),
    enabled_days: (),
    // time_of_day: TimeOfDay,
    // possibly volume
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimeOfDay {
    AM,
    PM,
}

impl Default for TimeOfDay {
    fn default() -> Self {
        TimeOfDay::AM
    }
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
    alarm_time_input_mins: u8,
    alarm_time_input_hour: u8,
    alarm_time_time_of_day: TimeOfDay,
}

impl App {
    pub fn new(time_format: String) -> Self {
        Self {
            dark_theme: true,
            alarms: Vec::new(),
            time_format,
            in_config: false,
            adding_alarm: false,
            alarm_time_input_mins: 0,
            alarm_time_input_hour: 0,
            alarm_time_time_of_day: TimeOfDay::AM,
        }
    }
}

impl eframe::App for App {
    // TODO: extract into different functions
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        if self.dark_theme {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }
        // config window
        if self.in_config {
            Window::new("settings ⚙").show(ctx, |ui| {
                if ui.button("x").clicked() {
                    self.in_config = false;
                }
            });
        }
        // alarm creation
        if self.adding_alarm {
            Window::new("adding alarm").show(ctx, |ui| {
                // TODO: make look nices
                if ui.button("x").clicked() {
                    self.adding_alarm = false;
                }

                (1..=12).for_each(|i| {
                    ui.selectable_value(
                        &mut self.alarm_time_input_hour,
                        if i != 12 { i } else { 0 },
                        i.to_string(),
                    );
                });
                ui.separator();
                (0..=59).for_each(|i| {
                    ui.selectable_value(&mut self.alarm_time_input_mins, i, i.to_string());
                });
                ui.selectable_value(&mut self.alarm_time_time_of_day, TimeOfDay::AM, "AM");
                ui.selectable_value(&mut self.alarm_time_time_of_day, TimeOfDay::PM, "PM");
                if ui.button("done").clicked() {
                    self.alarms.push(Alarm {
                        // we can use unwrap b/c we are validating time before
                        time: chrono::NaiveTime::from_hms_opt(
                            match self.alarm_time_time_of_day {
                                TimeOfDay::AM => self.alarm_time_input_hour as u32,
                                TimeOfDay::PM => (self.alarm_time_input_hour + 12) as u32,
                            },
                            self.alarm_time_input_mins.into(),
                            0,
                        )
                        .unwrap(),
                        name: None,
                        sound: None,
                        snooze_time: (),
                        enabled_days: (),
                        // time_of_day: self.alarm_time_time_of_day,
                    });
                    self.adding_alarm = false;
                }
            });
        }
        // header
        TopBottomPanel::top("time_and_ctrl").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.dark_theme, "Dark Theme");
                // ui.label();
                // TODO: fix allignment
                ui.centered_and_justified(|ui| {
                    ui.label(format!(
                        "Time: {}",
                        chrono::Local::now().format(&self.time_format)
                    ));
                });
                ui.with_layout(Layout::right_to_left(eframe::emath::Align::Min), |ui| {
                    if ui.button("⚙").clicked() {
                        self.in_config = true;
                    }
                });
            });
        });
        CentralPanel::default().show(ctx, |ui| {
            if ui.button("+").clicked() {
                self.adding_alarm = true;
            }
            // TOPO use grid allignment
            for alarm in &self.alarms {
                if let Some(name) = &alarm.name {
                    ui.label(name);
                }
                ui.label(alarm.time.format(&self.time_format).to_string());
            }
        });
    }
}
