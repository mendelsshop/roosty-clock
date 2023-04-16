#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(clippy::use_self, rust_2018_idioms)]

use core::fmt;
use std::path::PathBuf;

use eframe::{
    egui::{self, CentralPanel, Layout, ScrollArea, Separator, TopBottomPanel, Visuals, Window},
    epaint::vec2,
};

/// represnts an alarm
/// contains the time that the alarm should go of at.
/// as well as an optinal sound and name
struct Alarm {
    time: chrono::NaiveTime,
    name: Option<String>,
    /// there is a default sound
    sound: AlarmSound,
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

#[derive(Default, Clone, PartialEq, Eq)]
enum AlarmSound {
    // TODO: better names/more of them
    #[default]
    Ring,
    BingBong,
    TickTock,
    Rain,
    Custom(PathBuf, String),
}

impl fmt::Display for AlarmSound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                // for custom shows the name and file path, otherwise shows name of alarm
                AlarmSound::Custom(file, name) => format!("{name} ({})", file.to_string_lossy()),
                AlarmSound::Ring => stringify!(Ring).to_string(),
                AlarmSound::BingBong => stringify!(BingBong).to_string(),
                AlarmSound::TickTock => stringify!(TickTock).to_string(),
                AlarmSound::Rain => stringify!(Rain).to_string(),
            }
        )
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
    alarm_sound: AlarmSound,
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
            alarm_sound: AlarmSound::Ring,
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
        // TODO: remove staticly set vec2s
        if self.adding_alarm {
            Window::new("adding alarm")
                .fixed_size(vec2(200.0, 50.0))
                .show(ctx, |ui| {
                    let mut text_input_name = String::new();
                    ui.horizontal(|ui| {
                        ui.label("name: ");
                        ui.text_edit_singleline(&mut text_input_name);
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        // time selector
                        ui.vertical(|ui| {
                            ui.set_max_size(vec2(95.0, 115.0));
                            ui.horizontal(|ui| {
                                // hour selector
                                ui.vertical(|ui| {
                                    ui.label("Hour");
                                    ScrollArea::vertical().id_source("hours").show(ui, |ui| {
                                        (1..=12).for_each(|i| {
                                            ui.selectable_value(
                                                &mut self.alarm_time_input_hour,
                                                if i != 12 { i } else { 0 },
                                                i.to_string(),
                                            );
                                        });
                                    });
                                });

                                ui.add(Separator::default().vertical());
                                // minute selector
                                ui.vertical(|ui| {
                                    ui.label("Minute");
                                    ScrollArea::vertical().id_source("minutes").show(ui, |ui| {
                                        (0..=59).for_each(|i| {
                                            ui.selectable_value(
                                                &mut self.alarm_time_input_mins,
                                                i,
                                                i.to_string(),
                                            );
                                        });
                                    });
                                });
                            });
                            ui.separator();
                            // am or pm
                            ui.horizontal(|ui| {
                                ui.add_space(15.0);
                                ui.selectable_value(
                                    &mut self.alarm_time_time_of_day,
                                    TimeOfDay::AM,
                                    "AM",
                                );
                                ui.selectable_value(
                                    &mut self.alarm_time_time_of_day,
                                    TimeOfDay::PM,
                                    "PM",
                                );
                            });
                        });

                        ui.add(Separator::default().vertical());
                        ui.vertical(|ui| {
                            ui.scope(|ui| {
                                // set size of alarm selector so it doesnt make alarm creation to big when using cutom alarm
                                if matches!(self.alarm_sound, AlarmSound::Custom(..)) {
                                    ui.set_max_size(vec2(75.0, 80.0));
                                }

                                // pick an alarm sound
                                // TODO: make something that automates this
                                ScrollArea::vertical().id_source("alarm").show(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.alarm_sound,
                                        AlarmSound::Ring,
                                        "Ring",
                                    );
                                    ui.selectable_value(
                                        &mut self.alarm_sound,
                                        AlarmSound::BingBong,
                                        "BingBong",
                                    );
                                    ui.selectable_value(
                                        &mut self.alarm_sound,
                                        AlarmSound::TickTock,
                                        "TickTock",
                                    );
                                    ui.selectable_value(
                                        &mut self.alarm_sound,
                                        AlarmSound::Rain,
                                        "Rain",
                                    );
                                    ui.selectable_value(
                                        &mut self.alarm_sound,
                                        AlarmSound::Custom(PathBuf::new(), String::new()),
                                        "custom",
                                    );
                                });
                            });
                            // set custom alarm sound stuff
                            if let AlarmSound::Custom(path, name) = &mut self.alarm_sound {
                                ui.text_edit_singleline(name);
                                ui.horizontal(|ui| {
                                    if ui.button("file").clicked() {
                                        // TODO: validate is a sound file
                                        if let Some(path_name) = rfd::FileDialog::new().pick_file()
                                        {
                                            *path = path_name;
                                        }
                                    }
                                    ui.label(path.to_string_lossy());
                                });
                            }
                        });
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
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
                                sound: self.alarm_sound.clone(),
                                snooze_time: (),
                                enabled_days: (),
                            });
                            self.adding_alarm = false;
                        }
                        if ui.button("cancel").clicked() {
                            self.adding_alarm = false;
                        }
                    });
                });
        }
        // header
        TopBottomPanel::top("time_and_ctrl").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.dark_theme, "Dark Theme");
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
                ui.label(format!("alarm sound: {}", alarm.sound));
            }
        });
    }
}
