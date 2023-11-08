use std::{
    collections::HashMap,
    fmt,
    ops::{AddAssign, Not},
    path::PathBuf,
};

use chrono::{NaiveTime, Timelike};
use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::{alarm_edit::EditingState, AlarmBuilder, TimeOfDay};

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Not for Theme {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }
}

impl From<Theme> for egui::Visuals {
    fn from(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::dark(),
            Theme::Light => Self::light(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub(crate) time_format: String,
    #[serde(default)]
    pub(crate) theme: Theme,
    pub(crate) alarms: Vec<Alarm>,
    pub(crate) sounds: HashMap<String, Sound>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            time_format: "%l:%M %p".to_string(),
            theme: Theme::Dark,
            alarms: vec![],
            // Ring,
            // BingBong,
            // TickTock,
            // Rain,
            sounds: vec![
                ("ring".to_string(), Sound::ring()),
                ("bing bong".to_string(), Sound::bing_bong()),
                ("tick tock".to_string(), Sound::tick_tock()),
                ("beep beep".to_string(), Sound::beep_beep()),
                ("rain".to_string(), Sound::rain()),
            ]
            .into_iter()
            .collect(),
        }
    }
}

impl Config {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn load(path: PathBuf) -> Self {
        let config = std::fs::read_to_string(path).expect("couldn't read config file");
        toml::from_str(&config).expect("couldn't parse config file")
    }

    pub fn save(&self, path: PathBuf) {
        let config = toml::to_string(self).expect("couldn't serialize config");
        std::fs::create_dir_all(path.parent().unwrap()).expect("couldn't create config dir");
        std::fs::write(path, config).expect("couldn't write config file");
    }

    #[must_use]
    pub fn config_path() -> PathBuf {
        let mut path = directories::ProjectDirs::from("", "", "roosty_clock")
            .expect("couldn't get config path")
            .config_dir()
            .to_path_buf();
        path.push("config.toml");
        path
    }

    #[must_use]
    pub fn sounds_path() -> PathBuf {
        let mut path = directories::ProjectDirs::from("", "", "roosty_clock")
            .expect("couldn't get sounds directory path")
            .data_dir()
            .to_path_buf();
        path.push("sounds");
        path
    }

    #[must_use]
    pub fn is_config_present() -> bool {
        Self::config_path().exists()
    }
}

#[inline]
#[must_use]
pub const fn always_true() -> bool {
    true
}

static mut UID: usize = 0;

pub fn get_uid() -> usize {
    // SAFETY: this is only called when we are creating a new alarm which only happens in the main thread
    unsafe {
        UID += 1;
        UID
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alarm {
    pub name: Option<String>,
    #[serde(with = "toml_datetime_compat")]
    pub time: NaiveTime,
    pub volume: f32,
    #[serde(default = "Sound::get_default_name")]
    pub sound: String,
    #[serde(default = "always_true")]
    pub enabled: bool,
    #[serde(skip)]
    pub editing: Option<AlarmBuilder>,
    #[serde(skip)]
    pub rang_today: bool,
    #[serde(skip, default = "get_uid")]
    pub id: usize,
}

impl From<Alarm> for AlarmBuilder {
    fn from(alarm: Alarm) -> Self {
        let (ampm, hour) = alarm.time.hour12();
        Self {
            name: alarm.name.unwrap_or_default(),
            hour: if hour == 12 { 0 } else { hour } as u8,
            minute: alarm.time.minute() as u8,
            time_of_day: if ampm { TimeOfDay::PM } else { TimeOfDay::AM },
            sound: alarm.sound,
            volume: alarm.volume,
        }
    }
}

impl AddAssign for Alarm {
    /// used so that when we edit an alarm we don't lose its id
    /// also so to reset the rang_today field
    fn add_assign(&mut self, rhs: Self) {
        self.time = rhs.time;
        self.volume = rhs.volume;
        self.sound = rhs.sound;
        self.enabled = rhs.enabled;
        self.name = rhs.name;
    }
}

impl Alarm {
    // returns true if we edited the alarm
    pub(crate) fn render_alarm(
        &mut self,
        time_format: &str,
        ui: &mut eframe::egui::Ui,
        ctx: &eframe::egui::Context,
    ) -> bool {
        let mut ret = false;
        ui.scope(|ui| {
            // gray out color if alarm is disabled
            if !self.enabled {
                let faded = ui.visuals().fade_out_to_color();
                ui.visuals_mut().panel_fill = faded;
            }

            ui.horizontal(|ui| {
                // name
                ui.label(self.name.as_ref().unwrap_or(&"alarm".to_string()));
                // on off button
                if ui.checkbox(&mut self.enabled, "enabled").clicked() {
                    ret = true;
                }
            });
            ui.label(self.time.format(time_format).to_string());
            ui.label(format!("alarm sound: {}", self.sound));
            if ui
                .add(
                    egui::Slider::new(&mut self.volume, 0.0..=100.0)
                        .integer()
                        .suffix("%")
                        .text("volume"),
                )
                .changed()
            {
                ret = true;
            }

            if let Some(editing) = &mut self.editing {
                // TODO: passing the actual sounds
                match editing.render_alarm_editor(ctx, &mut HashMap::new()) {
                    EditingState::Done(new_alarm) => {
                        self.editing = None;
                        ret = true;
                        *self += new_alarm;
                    }
                    EditingState::Cancelled => {
                        self.editing = None;
                    }
                    _ => {}
                }
            }
            ui.horizontal(|ui| {
                if ui.button("edit").clicked() {
                    // if alarm is set for 5:00 PM and you click edit it will show 5:00 PM instead of 12:00 AM
                    // by using current alarm config
                    self.editing = Some(AlarmBuilder::from(self.clone()));
                }
            });
        });
        ret
    }

    pub fn send_stop(&mut self, sender: &std::sync::mpsc::Sender<crate::communication::Message>) {
        sender
            .send(crate::communication::Message::new(
                crate::communication::MessageType::AlarmStopped,
                self.id,
            ))
            .unwrap();
        self.rang_today = false;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sound {
    pub name: String,
    pub path: PathBuf,
}

impl fmt::Display for Sound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}",
            self.name,
            self.path.file_name().unwrap().to_string_lossy()
        )
    }
}

impl Default for Sound {
    fn default() -> Self {
        Self::ring()
    }
}

impl Sound {
    #[must_use]
    pub fn get_default_name() -> String {
        Self::ring().name
    }

    #[must_use]
    pub const fn new(name: String, path: PathBuf) -> Self {
        Self { name, path }
    }

    #[must_use]
    pub fn ring() -> Self {
        Self {
            name: "ring".to_string(),
            path: Config::sounds_path().join("ring.mp3"),
        }
    }

    #[must_use]
    pub fn bing_bong() -> Self {
        Self {
            name: "bing bong".to_string(),
            path: Config::sounds_path().join("bing_bong.mp3"),
        }
    }

    #[must_use]
    pub fn tick_tock() -> Self {
        Self {
            name: "tick tock".to_string(),
            path: Config::sounds_path().join("tick_tock.mp3"),
        }
    }

    #[must_use]
    pub fn beep_beep() -> Self {
        Self {
            name: "beep beep".to_string(),
            path: Config::sounds_path().join("beep_beep.mp3"),
        }
    }

    #[must_use]
    pub fn rain() -> Self {
        Self {
            name: "rain".to_string(),
            path: Config::sounds_path().join("rain.mp3"),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}
