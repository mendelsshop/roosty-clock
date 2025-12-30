#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(
    clippy::use_self,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::missing_panics_doc
)]
use chrono::{DateTime, Days};
use interprocess::local_socket::{GenericNamespaced, ListenerOptions, Stream, prelude::*};
use rodio::{Sink, Source, decoder};
use roosty_clockd::config::Config;
use roosty_clockd::config::{self, get_uid};
use roosty_clockd::read;
use roosty_clockd::{Alarm, AlarmEdit};
use roosty_clockd::{ClientMessage, ServerMessage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufReader, prelude::*};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use timer::{Guard, Timer};

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

    if !Config::is_config_present() {
        Config::new().save(Config::config_path());
        // write alarm sounds (from assets folder)
        std::fs::create_dir_all(Config::sounds_path()).unwrap();
        let mut beep_beep_file =
            fs::File::create(Config::sounds_path().join("beep_beep.mp3")).unwrap();
        beep_beep_file
            .write_all(std::include_bytes!("../../assets/beep_beep.mp3"))
            .unwrap();
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

    // listener.set_nonblocking(interprocess::local_socket::ListenerNonblockingMode::Stream);
    eprintln!("Server running at {printname}");

    let (s, r) = crossbeam_channel::unbounded();
    let (s_server, r_server) = mpsc::channel();

    let timer = Timer::new();
    let mut alarm_timers: HashMap<u64, Guard> =
        HashMap::from_iter(config.alarms.data.iter().map(|(id, alarm)| {
            (
                *id,
                alarm_to_timer(&config, &timer, chrono::Local::now(), alarm, s.clone()),
            )
        }));
    {
        let (s, r) = (s.clone(), r.clone());
        thread::spawn(move || {
            loop {
                if let Ok(m) = r.recv_timeout(Duration::from_millis(10)) {
                    match m {
                        Alert::AlarmSet(id, alarm_edit) => {
                            if let Some(alarm) = config.alarms.data.get_mut(&id) {
                                match alarm_edit {
                                    AlarmEdit::Time(new_time) => alarm.time = new_time,
                                    AlarmEdit::Name(new_name) => alarm.name = new_name,
                                    AlarmEdit::Sound(new_sound) => alarm.sound = new_sound,
                                    AlarmEdit::Volume(new_volume) => alarm.volume = new_volume,
                                    AlarmEdit::Enable(new_enabled) => alarm.enabled = new_enabled,
                                }
                                let alarm = config.alarms.data.get(&id).unwrap();
                                alarm_timers.insert(
                                    alarm.id,
                                    alarm_to_timer(
                                        &config,
                                        &timer,
                                        chrono::Local::now(),
                                        alarm,
                                        s.clone(),
                                    ),
                                );
                            }
                        }
                        Alert::AlaramAdded(alarm) => {
                            let alarm = config::Alarm {
                                name: alarm.name,
                                time: alarm.time,
                                volume: alarm.volume,
                                sound: alarm.sound,
                                enabled: true,
                                rang_today: false,
                                id: alarm.id,
                            };
                            alarm_timers.insert(
                                alarm.id,
                                alarm_to_timer(
                                    &config,
                                    &timer,
                                    chrono::Local::now(),
                                    &alarm,
                                    s.clone(),
                                ),
                            );
                            config.alarms.insert(alarm);
                        }
                        Alert::AlarmRemoved(id) => {
                            config.alarms.data.remove(&id).unwrap();
                            alarm_timers.remove(&id).unwrap();
                        }
                        Alert::SoundAdded(sound) => {
                            config.sounds.sounds.insert(sound.name.clone(), sound);
                        }
                        Alert::SoundRemoved(sound) => {
                            config.sounds.sounds.remove(&sound).unwrap();
                        }
                        Alert::AlarmRinging(_) => {}
                        Alert::AlarmStopped(id) => {
                            if let Some(alarm) = config.alarms.data.get(&id) {
                                alarm_timers.insert(
                                    id,
                                    alarm_to_timer(
                                        &config,
                                        &timer,
                                        chrono::Local::now()
                                            .checked_add_days(Days::new(0))
                                            .unwrap(),
                                        alarm,
                                        s.clone(),
                                    ),
                                );
                            }
                        }
                    }
                }
                if let Ok(ServerCommand { kind, reciever }) = r_server.recv() {
                    println!("got message");
                    match kind {
                        ServerCommandKind::NewUID => {
                            reciever.send(ServerResponce::NewUID(get_uid())).unwrap();
                        }
                        ServerCommandKind::GetAlarms => {
                            println!("get alarms - server server");
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
        // TODO: handle alerts from other threads, has to have access to writer
        let (s, _r) = (s.clone(), r.clone());
        let s_server = s_server.clone();
        // let mut conn = BufReader::new(conn);
        let (reader, mut writer) = conn.split();

        let (s_client, _r_client) = mpsc::channel();
        thread::spawn(move || -> ! {
            // let mut reader = BufReader::new(reader);
            let mut reader = reader;
            let mut buffer: Vec<u8> = Vec::new();
            // Wrap the connection into a buffered receiver right away
            // so that we could receive a single line from it.
            // let mut conn = BufReader::new(reader);
            println!("Incoming connection!");

            // Since our client example sends first, the server should receive a line and only then
            // send a response. Otherwise, because receiving from and sending to a connection cannot
            // be simultaneous without threads or async, we can deadlock the two processes by having
            // both sides wait for the send buffer to be emptied by the other.
            loop {
                println!("waiting");
                // TODO: maybe reading shouldn't block
                if read(&mut reader, &mut buffer).is_ok()
                    && let Ok(message) = {
                        println!("data found");
                        bitcode::deserialize(&buffer).map_err(|e| {
                            println!("ee{e}");
                            ();
                        })
                    }
                {
                    println!("got message {message:?} {buffer:?}");
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
                        ClientMessage::RemoveAlarm(id) => {
                            s.send(Alert::AlarmRemoved(id)).unwrap();
                        }
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
                if let Ok(message) = _r_client.recv_timeout(Duration::from_millis(10)) {
                    let message = match message {
                        ServerResponce::NewUID(id) => ServerMessage::UID(id),
                        ServerResponce::Alarms(alarms) => ServerMessage::Alarms(alarms),
                        ServerResponce::Sounds(sounds) => ServerMessage::Sounds(sounds),
                    };
                    let message = bitcode::serialize(&message).unwrap();
                    roosty_clockd::write(&mut writer, &message);
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

fn alarm_to_timer(
    config: &config::Config,
    timer: &Timer,
    time: DateTime<chrono::Local>,
    alarm: &config::Alarm,
    s: crossbeam_channel::Sender<Alert>,
) -> Guard {
    let date = time.with_time(alarm.time).unwrap();
    let path = config.sounds.sounds.get(&alarm.sound).unwrap().path.clone();
    let id = alarm.id;
    let enabled = alarm.enabled;
    // TODO: if alarm time before current time, add a day.

    timer.schedule_with_date(date, move || {
        if enabled {
            s.send(Alert::AlarmRinging(id));
            let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
            let input =
                decoder::Decoder::new(BufReader::new(std::fs::File::open(path.clone()).unwrap()))
                    .unwrap()
                    .repeat_infinite();
            let sink = Sink::connect_new(stream_handle.mixer());
            // sink.set_volume(volume / 100.0);
            sink.append(input);
            sink.play();
            loop {
                cpvc::set_mute(false);
            }
        }
    })
}
