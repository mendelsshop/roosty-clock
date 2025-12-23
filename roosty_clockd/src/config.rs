use core::fmt;
use std::{collections::HashMap, hash::Hash, path::PathBuf};

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
// idea is to have ids be non persistant so users do not have to worry about manually enteriing
// ids, but maybe better idea is:
// alarms use ids
// sounds are by name. (as sounds are referenced so they need to have a presistant way to
// refrence them)
pub struct Config {
    pub alarms: Collection<u64, Alarm>,
    #[serde(flatten)]
    pub sounds: Sounds,
}
// https://stackoverflow.com/questions/79314434/rust-serde-serialization-to-from-vec-into-hashmap
pub trait GetId<T> {
    fn get_id(&self) -> &T;
}

/// Serializable collection
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(from = "Vec<V>", into = "Vec<V>")]
pub struct Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: GetId<K> + Clone,
{
    pub data: HashMap<K, V>,
}

impl<K: Default, V> Default for Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: GetId<K> + Clone,
{
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl<K, V> Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: GetId<K> + Clone,
{
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, item: V) -> Option<V> {
        let id = item.get_id().to_owned();
        self.data.insert(id, item)
    }
}

impl<K, V> From<Vec<V>> for Collection<K, V>
where
    K: Eq + Hash + Clone,
    V: GetId<K> + Clone,
{
    fn from(value: Vec<V>) -> Self {
        let mut obj: Self = Self::new();
        value.into_iter().for_each(|v| {
            obj.insert(v);
        });
        obj
    }
}

impl<K, V> From<Collection<K, V>> for Vec<V>
where
    K: Eq + Hash + Clone,
    V: GetId<K> + Clone,
{
    fn from(val: Collection<K, V>) -> Self {
        Self::from_iter(val.data.into_values())
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sounds {
    pub sounds: HashMap<String, Sound>,
    pub default_sound: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            alarms: Collection::default(),
            // Ring,
            // BingBong,
            // TickTock,
            // Rain,
            sounds: Sounds {
                sounds: [
                    ("ring".to_string(), Sound::ring()),
                    ("bing bong".to_string(), Sound::bing_bong()),
                    ("tick tock".to_string(), Sound::tick_tock()),
                    ("beep beep".to_string(), Sound::beep_beep()),
                    ("raing".to_string(), Sound::rain()),
                ]
                .into_iter()
                .collect(),
                default_sound: "beep beep".to_string(),
            },
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

static mut UID: u64 = 0;
pub fn get_uid() -> u64 {
    // SAFETY: this is only called when we are creating a new alarm which only happens in the main thread
    unsafe {
        UID += 1;
        UID
    }
}

impl GetId<u64> for Alarm {
    fn get_id(&self) -> &u64 {
        &self.id
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
    pub rang_today: bool,
    #[serde(skip, default = "get_uid")]
    pub id: u64,
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
        Self::beep_beep()
    }
}

impl Sound {
    #[must_use]
    pub fn get_default_name() -> String {
        Self::default().name
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
