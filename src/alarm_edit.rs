use std::{collections::HashMap, ffi::OsStr, path::Path};

use chrono::NaiveTime;
use eframe::egui::{self, ScrollArea, TextEdit, Widget, Window};

use crate::{
    config::{self, get_uid, Alarm, Sound, Sounds},
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
            editing: None,
            rang_today: false,
            ringing: false,
            id: get_uid(),
        }
    }

    pub(crate) fn edit_alarm(&mut self, ui: &mut egui::Ui, sounds: &mut Sounds) {
        ui.text_edit_singleline(&mut self.name);
        ui.horizontal(|ui| {
            self.render_time_editor(ui);
            // // sound editor
            // // ui.separator();
            self.render_sound_editor(ui, sounds);
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
    pub(crate) fn render_am_pm_selector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(15.0);
            ui.selectable_value(&mut self.time_of_day, TimeOfDay::AM, "AM");
            ui.selectable_value(&mut self.time_of_day, TimeOfDay::PM, "PM");
        });
    }

    pub(crate) fn render_minute_selector(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Minute");
            if ui.button("Up").clicked() && self.minute < 60 {
                self.minute += 1;
                self.minute_string = self.minute.to_string();
            }
            if {
                TextEdit::singleline(&mut self.minute_string)
                    .desired_width(20.0)
                    .char_limit(2)
                    .ui(&mut *ui)
            }
            .lost_focus()
            {
                // if the input value is vaild, update the value
                if let Ok(parsed_value) = self.minute_string.parse::<u8>() {
                    self.minute = parsed_value.clamp(0, 60);
                }
                // sync the input value and the value regardless
                self.minute_string = self.minute.to_string();
            }

            if ui.button("Down").clicked() && self.minute > 0 {
                self.minute -= 1;
                self.minute_string = self.minute.to_string();
            }
        });
    }

    pub(crate) fn render_hour_selector(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.label("Hour");
            if ui.button("Up").clicked() && self.hour < 12 {
                self.hour += 1;
                self.hour_string = self.hour.to_string();
            }
            if {
                TextEdit::singleline(&mut self.hour_string)
                    .desired_width(20.0)
                    .char_limit(2)
                    .ui(&mut *ui)
            }
            .lost_focus()
            {
                // if the input value is vaild, update the value
                if let Ok(parsed_value) = self.hour_string.parse::<u8>() {
                    self.hour = parsed_value.clamp(0, 12);
                }
                // sync the input value and the value regardless
                self.hour_string = self.hour.to_string();
            }

            if ui.button("Down").clicked() && self.hour > 0 {
                self.hour -= 1;
                self.hour_string = self.hour.to_string();
            }
        });
    }

    pub(crate) fn render_sound_editor(&mut self, ui: &mut egui::Ui, sounds: &mut Sounds) {
        Self::render_sound_selector_editor(&mut self.sound, ui, &mut sounds.sounds);
        self.render_volume_slider(ui);
    }

    pub(crate) fn render_sound_selector_editor(
        sound: &mut String,
        ui: &mut egui::Ui,
        sounds: &mut HashMap<String, Sound>,
    ) {
        ui.vertical(|ui| {
            // alarm sound
            Self::render_alarm_sound_selector(sound, ui, sounds);
            // set custom alarm sound stuff
            Self::render_custom_alarm_sound_editor(sounds, ui);
        });
    }

    fn render_custom_alarm_sound_editor(sounds: &mut HashMap<String, Sound>, ui: &mut egui::Ui) {
        if ui.button("Custom").clicked() {
            // TODO: rfd with gnome opens Recents not audio folder https://github.com/PolyMeilex/rfd/issues/237
            let file_dialog = rfd::FileDialog::new().set_title("Pick alarm sound");
            let file_dialog = match directories::UserDirs::new()
                .and_then(|u| u.audio_dir().map(Path::to_path_buf))
            {
                Some(audio_path) => file_dialog.set_directory(audio_path),
                None => file_dialog,
            };

            // TODO: maybe copy sound to sound directory

            // when done in alarm editor which one do we pick if we have multiple alarms
            if let Some(paths) = { file_dialog }.pick_files() {
                paths.iter().for_each(|path_name| {
                    if let Some(name) = path_name.file_prefix().and_then(OsStr::to_str) {
                        sounds.insert(
                            name.to_string(),
                            Sound {
                                name: name.to_string(),
                                path: path_name.clone(),
                            },
                        );
                    }
                });
            }
        }
    }

    pub(crate) fn render_alarm_sound_selector(
        sound: &mut String,
        ui: &mut egui::Ui,
        sounds: &mut HashMap<String, Sound>,
    ) {
        ui.vertical(|ui| {
            // set size of alarm selector so it doesnt make alarm creation to big when using cutom alarm
            // pick an alarm sound
            // TODO: make something that automates this
            ScrollArea::vertical().id_salt("alarm").show(ui, |ui| {
                for name in sounds.keys() {
                    ui.selectable_value(sound, name.clone(), name);
                }
            });
        });
    }

    pub fn render_volume_slider(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::Slider::new(&mut self.volume, 0.0..=100.0)
                .vertical()
                .integer()
                .suffix("%")
                .text("volume"),
        );
    }

    pub fn render_alarm_editor(
        &mut self,
        ctx: &egui::Context,
        sounds: &mut Sounds,
    ) -> EditingState {
        let mut ret = EditingState::Editing;
        // if no alarm name set we need way to differentiate between different alarms
        Window::new(format!("editing alarm {}", self.name)).show(ctx, |ui| {
            self.edit_alarm(ui, sounds);
            ui.horizontal(|ui| {
                if ui.button("done").clicked() {
                    ret = EditingState::Done(self.clone().build());
                } else if ui.button("cancel").clicked() {
                    ret = EditingState::Cancelled;
                } else {
                    ret = EditingState::Editing;
                }
            });
        });
        ret
    }
}

pub enum EditingState {
    Cancelled,
    Editing,
    Done(Alarm),
}
