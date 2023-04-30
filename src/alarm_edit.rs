use std::path::PathBuf;

use chrono::Timelike;
use eframe::egui::{self, ScrollArea};

use crate::{
    alarm::{Alarm, AlarmSound},
    TimeOfDay,
};

impl Alarm {
    // whenever alarm time is changed, update next alarm if changing the alarm time makes next alarm in the past
    pub (crate) fn set_hour(&mut self, hour: u32) {
        self.time = self.time.with_hour(hour).unwrap();
        if self.next_alarm.with_hour(hour).unwrap() < chrono::Local::now().naive_local() {
            self.next_alarm = self.next_alarm.with_hour(hour).unwrap() + chrono::Duration::days(1);
        } else {
            self.next_alarm = self.next_alarm.with_hour(hour).unwrap();
        }
    }

    pub (crate) fn set_minute(&mut self, minute: u32) {
        self.time = self.time.with_minute(minute).unwrap();
        if self.next_alarm.with_minute(minute).unwrap() < chrono::Local::now().naive_local() {
            self.next_alarm =
                self.next_alarm.with_minute(minute).unwrap() + chrono::Duration::days(1);
        } else {
            self.next_alarm = self.next_alarm.with_minute(minute).unwrap();
        }
    }

    pub (crate) fn set_ampm(&mut self, ampm: TimeOfDay) {
        let hour = self.time.hour12();
        match (ampm, hour.0) {
            // am, bool false = am, bool true = pm
            (TimeOfDay::AM, false) | (TimeOfDay::PM, true) => {}
            (TimeOfDay::AM, true) => {
                self.set_hour(hour.1 - 12);
            }
            (TimeOfDay::PM, false) => {
                self.set_hour(hour.1 + 12);
            }
        }
    }

    pub(crate) fn edit_alarm(&mut self, ui: &mut egui::Ui) {
        ui.text_edit_singleline(&mut self.name);
        ui.horizontal(|ui| {
            self.render_time_editor(ui);
            // sound editor
            // ui.separator();
            self.render_sound_editor(ui);
        });
    }

    pub(crate) fn render_sound_editor(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // alarm sound
            self.render_alarm_sound_selector(ui);
            // set custom alarm sound stuff
            self.render_custom_alarm_sound_editor(ui);
        });
    }

    pub(crate) fn render_time_editor(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                // hour selector
                self.render_hour_selector(ui);
                // ui.add(Separator::default().spacing(10f32));
                // minute selector
                self.render_minute_selector(ui);
            });
            // ui.add(Separator::default().spacing(10f32));
            // am or pm
            self.render_am_pm_selector(ui);
        });
    }

    pub(crate) fn render_custom_alarm_sound_editor(&mut self, ui: &mut egui::Ui) {
        if let AlarmSound::Custom(path, name) = &mut self.sound {
            ui.horizontal(|ui| {
                ui.text_edit_singleline(name);
                if ui.button("file").clicked() {
                    // TODO: validate is a sound file
                    if let Some(path_name) = rfd::FileDialog::new().pick_file() {
                        *path = path_name;
                    }
                }
            });
        }
    }

    pub(crate) fn render_alarm_sound_selector(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // set size of alarm selector so it doesnt make alarm creation to big when using cutom alarm
            // pick an alarm sound
            // TODO: make something that automates this
            ScrollArea::vertical().id_source("alarm").show(ui, |ui| {
                ui.selectable_value(&mut self.sound, AlarmSound::Ring, "Ring");
                ui.selectable_value(&mut self.sound, AlarmSound::BingBong, "BingBong");
                ui.selectable_value(&mut self.sound, AlarmSound::TickTock, "TickTock");
                ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                ui.selectable_value(
                    &mut self.sound,
                    AlarmSound::Custom(PathBuf::new(), String::new()),
                    "custom",
                );
            });
        });
    }

    pub(crate) fn render_am_pm_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(15.0);
            if ui
                .selectable_value(&mut self.time_of_day, TimeOfDay::AM, "AM")
                .clicked()
            {
                self.set_ampm(TimeOfDay::AM);
            }
            if ui
                .selectable_value(&mut self.time_of_day, TimeOfDay::PM, "PM")
                .clicked()
            {
                self.set_ampm(TimeOfDay::PM);
            }
        });
    }

    pub(crate) fn render_minute_selector(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Minute");
            ScrollArea::vertical().id_source("minutes").show(ui, |ui| {
                (0..=59).for_each(|i| {
                    if ui
                        .selectable_value(&mut self.minute, i, i.to_string())
                        .clicked()
                    {
                        self.set_minute(i);
                    }
                });
            });
        });
    }

    pub(crate) fn render_hour_selector(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Hour");
            ScrollArea::vertical().id_source("hours").show(ui, |ui| {
                (1..=12).for_each(|i| {
                    if ui
                        .selectable_value(
                            &mut self.hour,
                            if i == 12 { 0 } else { i },
                            i.to_string(),
                        )
                        .clicked()
                    {
                        self.set_hour(if i == 12 { 0 } else { i });
                    }
                });
            });
        });
    }
}
