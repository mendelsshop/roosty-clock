use std::collections::HashMap;

use chrono::NaiveTime;
use eframe::egui::{self, ScrollArea};

use crate::{
    config::{self, Sound},
    AlarmBuilder, TimeOfDay,
};

impl AlarmBuilder {
    #[must_use]
    pub fn build(self) -> config::Alarm {
        config::Alarm {
            name: if self.name.is_empty() {
                None
            } else {
                Some(self.name)
            },
            time: NaiveTime::from_hms_opt(
                u32::from(if self.time_of_day == TimeOfDay::AM {
                    self.hour
                } else {
                    self.hour + 12
                }),
                u32::from(self.minute),
                0,
            )
            .unwrap(),
            sound: self.sound,
            volume: self.volume,
            enabled: true,
        }
    }

    pub(crate) fn edit_alarm(&mut self, ui: &mut egui::Ui, sounds: &mut HashMap<String, Sound>) {
        ui.text_edit_singleline(&mut self.name);
        ui.horizontal(|ui| {
            self.render_time_editor(ui);
            // // sound editor
            // // ui.separator();
            self.render_sound_editor(ui, sounds);
        });
    }

    pub(crate) fn set_hour(&mut self, hour: u8) {
        self.hour = hour;
    }

    pub(crate) fn set_minute(&mut self, minute: u8) {
        self.minute = minute;
    }

    pub(crate) fn set_ampm(&mut self, ampm: TimeOfDay) {
        self.time_of_day = ampm;
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

    pub(crate) fn render_sound_editor(
        &mut self,
        ui: &mut egui::Ui,
        sounds: &mut HashMap<String, Sound>,
    ) {
        ui.vertical(|ui| {
            // alarm sound
            self.render_alarm_sound_selector(ui, sounds);
            // set custom alarm sound stuff
            self.render_custom_alarm_sound_editor(ui);
        });
    }

    pub(crate) fn render_custom_alarm_sound_editor(&mut self, _ui: &mut egui::Ui) {
        // if let AlarmSound::Custom(path, name) = &mut self.sound {
        //     ui.horizontal(|ui| {
        //         ui.text_edit_singleline(name);
        //         if ui.button("file").clicked() {
        //             // TODO: validate is a sound file
        //             if let Some(path_name) = rfd::FileDialog::new().pick_file() {
        //                 *path = path_name;
        //             }
        //         }
        //     });
        // }
    }

    pub(crate) fn render_alarm_sound_selector(
        &mut self,
        ui: &mut egui::Ui,
        sounds: &mut HashMap<String, Sound>,
    ) {
        ui.vertical(|ui| {
            // set size of alarm selector so it doesnt make alarm creation to big when using cutom alarm
            // pick an alarm sound
            // TODO: make something that automates this
            ScrollArea::vertical().id_source("alarm").show(ui, |ui| {
                // ui.selectable_value(&mut self.sound, AlarmSound::Ring, "Ring");
                // ui.selectable_value(&mut self.sound, AlarmSound::BingBong, "BingBong");
                // ui.selectable_value(&mut self.sound, AlarmSound::TickTock, "TickTock");
                // ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                // ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                // ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                // ui.selectable_value(&mut self.sound, AlarmSound::Rain, "Rain");
                // ui.selectable_value(
                //     &mut self.sound,
                //     AlarmSound::Custom(PathBuf::new(), String::new()),
                //     "custom",
                // );
                for (name, _) in &mut *sounds {
                    ui.selectable_value(&mut self.sound, name.to_string(), name);
                }
            });
        });
    }
}
