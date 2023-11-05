use std::{collections::HashMap, fmt, ops::Not, path::PathBuf};

use chrono::NaiveTime;
use eframe::egui;
use serde::{Deserialize, Serialize};

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
                (
                    "ring".to_string(),
                    Sound::new("ring".to_string(), Self::sounds_path().join("ring.mp3")),
                ),
                (
                    "bing bong".to_string(),
                    Sound::new(
                        "bing bong".to_string(),
                        Self::sounds_path().join("bing_bong.mp3"),
                    ),
                ),
                (
                    "tick tock".to_string(),
                    Sound::new(
                        "tick tock".to_string(),
                        Self::sounds_path().join("tick_tock.mp3"),
                    ),
                ),
                (
                    "beep beep".to_string(),
                    Sound::new(
                        "beep beep".to_string(),
                        Self::sounds_path().join("beep_beep.mp3"),
                    ),
                ),
                (
                    "rain".to_string(),
                    Sound::new("rain".to_string(), Self::sounds_path().join("rain.mp3")),
                ),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Alarm {
    pub name: Option<String>,
    #[serde(with = "toml_datetime_compat")]
    pub time: NaiveTime,
    pub volume: f32,
    #[serde(default = "Sound::get_default_name")]
    pub sound: String,
    #[serde(default = "always_true")]
    pub enabled: bool,
}

impl Alarm {
    pub(crate) fn render_alarm(&mut self, time_format: &str, ui: &mut eframe::egui::Ui) {
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
                ui.checkbox(&mut self.enabled, "enabled");
            });
            ui.label(self.time.format(time_format).to_string());
            ui.label(format!("alarm sound: {}", self.sound));
            // self.edit_alarm(ui);
        });
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
    pub fn new(name: String, path: PathBuf) -> Self {
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
