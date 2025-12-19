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
use std::io::{self, BufReader, prelude::*};
use std::path::PathBuf;
use std::thread;

pub mod config {
    use core::fmt;
    use std::{collections::HashMap, path::PathBuf};

    use chrono::NaiveTime;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Config {
        pub(crate) time_format: String,
        pub(crate) alarms: Vec<Alarm>,
        #[serde(flatten)]
        pub(crate) sounds: Sounds,
    }
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Sounds {
        pub(crate) sounds: HashMap<String, Sound>,
        pub(crate) default_sound: String,
    }

    impl Default for Config {
        fn default() -> Self {
            Self {
                time_format: "%l:%M %p".to_string(),
                alarms: vec![],
                // Ring,
                // BingBong,
                // TickTock,
                // Rain,
                sounds: Sounds {
                    sounds: vec![
                        ("ring".to_string(), Sound::ring()),
                        ("bing bong".to_string(), Sound::bing_bong()),
                        ("tick tock".to_string(), Sound::tick_tock()),
                        ("beep beep".to_string(), Sound::beep_beep()),
                        ("rain".to_string(), Sound::rain()),
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
        pub rang_today: bool,
        #[serde(skip)]
        pub ringing: bool,
        #[serde(skip, default = "get_uid")]
        pub id: usize,
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
    pub sound: u64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sound {
    pub name: String,
    pub path: PathBuf,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AlarmEdit {
    #[serde(with = "toml_datetime_compat")]
    Time(NaiveTime),
    Name(Option<String>),
    Sound(u64),
    Volume(f64),
    Enable(bool),
}
#[derive(Debug, Serialize, Deserialize, Clone)]
enum ClientMessage {
    GetAlarms,
    SetAlarm(u64, AlarmEdit),
    AddAlarm(Alarm),
    RemoveAlarm(u64),
    GetSounds(u64),
    AdddSound(Sound),
    RemoveSound(u64),
    StopAlarm(u64),
    GetNewUID,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ServerMessage {
    Alarms(Vec<()>),
    AlarmSet(u64, AlarmEdit),
    AlaramAdded(Alarm),
    AlarmRemoved(u64),
    Sounds(Vec<Sound>),
    SoundAdded(Sound),
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
    SoundAdded(Sound),
    SoundRemoved(u64),
    AlarmRinging(u64),
    AlarmStopped(u64),
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
    for conn in listener.incoming().filter_map(handle_error) {
        let (s, _r) = (s.clone(), r.clone());
        thread::spawn(move || {
            let (read, mut write) = conn.split();
            let mut buffer = Vec::new();
            // Wrap the connection into a buffered receiver right away
            // so that we could receive a single line from it.
            let mut conn = BufReader::new(read);
            println!("Incoming connection!");

            // Since our client example sends first, the server should receive a line and only then
            // send a response. Otherwise, because receiving from and sending to a connection cannot
            // be simultaneous without threads or async, we can deadlock the two processes by having
            // both sides wait for the send buffer to be emptied by the other.
            if conn.read_to_end(&mut buffer).is_ok()
                && let Ok(message) = toml::from_slice(&buffer)
            {
                match message {
                    ClientMessage::GetNewUID => todo!(),
                    ClientMessage::GetAlarms => todo!(),
                    ClientMessage::SetAlarm(_, _alarm_edit) => todo!(),
                    ClientMessage::AddAlarm(_alarm) => todo!(),
                    ClientMessage::RemoveAlarm(_) => todo!(),
                    ClientMessage::GetSounds(_) => todo!(),
                    ClientMessage::AdddSound(_sound) => todo!(),
                    ClientMessage::RemoveSound(_) => todo!(),
                    ClientMessage::StopAlarm(i) => s.send(Alert::AlarmStopped(i)),
                };
            }

            // Now that the receive has come through and the client is waiting on the server's send, do
            // it. (`.get_mut()` is to get the sender, `BufReader` doesn't implement a pass-through
            // `Write`.)
            write.write_all(b"Hello from server!\n").unwrap();

            // Print out the result, getting the newline for free!

            // Clear the buffer so that the next iteration will display new data instead of messages
            // stacking on top of one another.
            buffer.clear();
        });
    }
    loop {
        if let Ok(m) = r.recv() {
            match m {
                Alert::AlarmSet(_id, _alarm_edit) => {}
                Alert::AlaramAdded(alarm) => {
                    config.alarms.push(config::Alarm {
                        name: alarm.name,
                        time: alarm.time,
                        volume: alarm.volume,
                        sound: todo!(),
                        enabled: true,
                        rang_today: false,
                        ringing: false,
                        id: config::get_uid(),
                    });
                }
                Alert::AlarmRemoved(_) => todo!(),
                Alert::SoundAdded(_sound) => todo!(),
                Alert::SoundRemoved(_) => todo!(),
                Alert::AlarmRinging(_) => todo!(),
                Alert::AlarmStopped(_) => todo!(),
            }
        }
    }
    Ok(())
}
