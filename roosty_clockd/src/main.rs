#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(
    clippy::use_self,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::missing_panics_doc
)]
use chrono::NaiveTime;
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, BufReader, prelude::*};
use std::sync::mpsc;
use std::thread;

use crate::config::get_uid;

pub mod config {
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
        pub(crate) alarms: Collection<u64, Alarm>,
        pub(crate) sounds: Sounds,
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
        pub(crate) sounds: HashMap<String, Sound>,
        pub(crate) default_sound: String,
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
        #[serde(skip)]
        pub ringing: bool,
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
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alarm {
    pub name: Option<String>,
    #[serde(with = "toml_datetime_compat")]
    pub time: NaiveTime,
    pub volume: f32,
    pub sound: String,
    pub id: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AlarmEdit {
    #[serde(with = "toml_datetime_compat")]
    Time(NaiveTime),
    Name(Option<String>),
    Sound(String),
    Volume(f32),
    Enable(bool),
}
#[derive(Debug, Serialize, Deserialize, Clone)]
enum ClientMessage {
    GetAlarms,
    SetAlarm(u64, AlarmEdit),
    AddAlarm(Alarm),
    RemoveAlarm(u64),
    GetSounds,
    AdddSound(config::Sound),
    RemoveSound(String),
    StopAlarm(u64),
    GetNewUID,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    Alarms(HashMap<u64, config::Alarm>),
    AlarmSet(u64, AlarmEdit),
    AlaramAdded(Alarm),
    AlarmRemoved(u64),
    Sounds(HashMap<String, config::Sound>),
    SoundAdded(config::Sound),
    SoundRemoved(u64),
    AlarmRinging(u64),
    AlarmStopped(u64),
    UID(u64),
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Alert {
    AlarmSet(u64, AlarmEdit),
    AlaramAdded(Alarm),
    AlarmRemoved(u64),
    SoundAdded(config::Sound),
    SoundRemoved(String),
    AlarmRinging(u64),
    AlarmStopped(u64),
}
#[allow(missing_debug_implementations)]
pub struct ServerCommand {
    kind: ServerCommandKind,
    reciever: mpsc::Sender<ServerResponce>,
}
#[allow(missing_debug_implementations)]
pub enum ServerResponce {
    NewUID(u64),
    Alarms(HashMap<u64, config::Alarm>),
    Sounds(HashMap<String, config::Sound>),
}
#[allow(missing_debug_implementations)]
pub enum ServerCommandKind {
    NewUID,
    GetAlarms,
    GetSounds,
}
fn main() -> std::io::Result<()> {
    // Define a function that checks for errors in incoming connections. We'll use this to filter
    // through connections that fail on initialization for one reason or another.
    fn handle_error(conn: io::Result<Stream>) -> Option<Stream> {
        match conn {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("Incoming connection failed: {e}");
                None
            }
        }
    }

    let mut config = config::Config::load(config::Config::config_path());
    // Pick a name.
    let printname = "roosty-clockd.sock";
    let name = printname.to_ns_name::<GenericNamespaced>()?;

    // Configure our listener...
    let opts = ListenerOptions::new().name(name);

    // ...then create it.
    let listener = match opts.create_sync() {
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
            eprintln!(
                "Error: could not start server because the socket file is occupied. Please check
                if {printname} is in use by another process and try again."
            );
            return Err(e);
        }
        x => x?,
    };

    eprintln!("Server running at {printname}");

    let (s, r) = crossbeam_channel::unbounded();
    let (s_server, r_server) = mpsc::channel();

    {
        let (_s, r) = (s.clone(), r.clone());
        thread::spawn(move || -> ! {
            loop {
                if let Ok(m) = r.recv() {
                    match m {
                        Alert::AlarmSet(id, alarm_edit) => {
                            if let Some(config::Alarm {
                                name,
                                time,
                                volume,
                                sound,
                                enabled,
                                ..
                            }) = config.alarms.data.get_mut(&id)
                            {
                                match alarm_edit {
                                    AlarmEdit::Time(new_time) => *time = new_time,
                                    AlarmEdit::Name(new_name) => *name = new_name,
                                    AlarmEdit::Sound(new_sound) => *sound = new_sound,
                                    AlarmEdit::Volume(new_volume) => *volume = new_volume,
                                    AlarmEdit::Enable(new_enabled) => *enabled = new_enabled,
                                }
                            }
                        }
                        Alert::AlaramAdded(alarm) => {
                            config.alarms.insert(config::Alarm {
                                name: alarm.name,
                                time: alarm.time,
                                volume: alarm.volume,
                                sound: alarm.sound,
                                enabled: true,
                                rang_today: false,
                                ringing: false,
                                id: alarm.id,
                            });
                        }
                        Alert::AlarmRemoved(id) => {
                            config.alarms.data.remove(&id).unwrap();
                        }
                        Alert::SoundAdded(sound) => {
                            config.sounds.sounds.insert(sound.name.clone(), sound);
                        }
                        Alert::SoundRemoved(sound) => {
                            config.sounds.sounds.remove(&sound).unwrap();
                        }
                        Alert::AlarmRinging(_) => {}
                        Alert::AlarmStopped(id) => {
                            if let Some(config::Alarm { ringing, .. }) =
                                config.alarms.data.get_mut(&id)
                            {
                                *ringing = false;
                            }
                        }
                    }
                }
                if let Ok(ServerCommand { kind, reciever }) = r_server.recv() {
                    match kind {
                        ServerCommandKind::NewUID => {
                            reciever.send(ServerResponce::NewUID(get_uid())).unwrap();
                        }
                        ServerCommandKind::GetAlarms => {
                            reciever
                                .send(ServerResponce::Alarms(config.alarms.data.clone()))
                                .unwrap();
                        }
                        ServerCommandKind::GetSounds => {
                            reciever
                                .send(ServerResponce::Sounds(config.sounds.sounds.clone()))
                                .unwrap();
                        }
                    }
                }
            }
        });
    }
    for conn in listener.incoming().filter_map(handle_error) {
        let (s, _r) = (s.clone(), r.clone());
        let s_server = s_server.clone();
        thread::spawn(move || {
            let (read, mut write) = conn.split();
            let mut buffer = Vec::new();
            // Wrap the connection into a buffered receiver right away
            // so that we could receive a single line from it.
            let mut conn = BufReader::new(read);
            println!("Incoming connection!");

            let (s_client, r_client) = mpsc::channel();
            // Since our client example sends first, the server should receive a line and only then
            // send a response. Otherwise, because receiving from and sending to a connection cannot
            // be simultaneous without threads or async, we can deadlock the two processes by having
            // both sides wait for the send buffer to be emptied by the other.
            loop {
                if conn.read_to_end(&mut buffer).is_ok()
                    && let Ok(message) = toml::from_slice(&buffer)
                {
                    match message {
                        ClientMessage::GetNewUID => {
                            s_server
                                .send(ServerCommand {
                                    kind: ServerCommandKind::NewUID,
                                    reciever: s_client.clone(),
                                })
                                .unwrap();
                        }
                        ClientMessage::GetAlarms => {
                            s_server
                                .send(ServerCommand {
                                    kind: ServerCommandKind::GetAlarms,
                                    reciever: s_client.clone(),
                                })
                                .unwrap();
                        }
                        ClientMessage::SetAlarm(alarm, alarm_edit) => {
                            s.send(Alert::AlarmSet(alarm, alarm_edit)).unwrap();
                        }
                        ClientMessage::AddAlarm(alarm) => {
                            s.send(Alert::AlaramAdded(alarm)).unwrap();
                        }
                        ClientMessage::RemoveAlarm(id) => s.send(Alert::AlarmRemoved(id)).unwrap(),
                        ClientMessage::GetSounds => {
                            s_server
                                .send(ServerCommand {
                                    kind: ServerCommandKind::GetSounds,
                                    reciever: s_client.clone(),
                                })
                                .unwrap();
                        }
                        ClientMessage::AdddSound(sound) => {
                            s.send(Alert::SoundAdded(sound)).unwrap();
                        }

                        ClientMessage::RemoveSound(sound) => {
                            s.send(Alert::SoundRemoved(sound)).unwrap();
                        }
                        ClientMessage::StopAlarm(i) => s.send(Alert::AlarmStopped(i)).unwrap(),
                    }
                }
                match r_client.recv().ok() {
                    Some(ServerResponce::NewUID(id)) => {
                        write
                            .write(toml::to_string(&ServerMessage::UID(id)).unwrap().as_bytes())
                            .unwrap();
                    }
                    Some(ServerResponce::Alarms(alarms)) => {
                        write
                            .write(
                                toml::to_string(&ServerMessage::Alarms(alarms))
                                    .unwrap()
                                    .as_bytes(),
                            )
                            .unwrap();
                    }
                    Some(ServerResponce::Sounds(_sounds)) => {
                        write
                            .write(
                                toml::to_string(&ServerMessage::Sounds(_sounds))
                                    .unwrap()
                                    .as_bytes(),
                            )
                            .unwrap();
                    }
                    None => {}
                }

                // Now that the receive has come through and the client is waiting on the server's send, do
                // it. (`.get_mut()` is to get the sender, `BufReader` doesn't implement a pass-through
                // `Write`.)

                // Print out the result, getting the newline for free!

                // Clear the buffer so that the next iteration will display new data instead of messages
                // stacking on top of one another.
                buffer.clear();
            }
        });
    }

    Ok(())
}
