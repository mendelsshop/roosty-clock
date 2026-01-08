#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(
    clippy::use_self,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::missing_panics_doc
)]
use chrono::Duration;
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
use timer::Timer;

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

    listener.set_nonblocking(interprocess::local_socket::ListenerNonblockingMode::Stream);
    eprintln!("Server running at {printname}");

    let (mut s, r) = async_broadcast::broadcast(10);
    s.set_overflow(true);
    let (s_server, r_server) = mpsc::channel();

    let _timer = Timer::new();
    {
        let mut sounds = config.sounds.sounds.clone();
        let s = s.clone();

        let alarms = config.alarms.data.clone();
        let mut r = r.new_receiver();
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        thread::spawn(move || {
            let mut alarms: HashMap<_, _> = alarms
                .into_iter()
                .map(
                    |(
                        id,
                        config::Alarm {
                            time,
                            volume,
                            enabled,
                            sound,
                            ..
                        },
                    )| {
                        let path = sounds.get(&sound).unwrap().path.clone();
                        let input = decoder::Decoder::new(BufReader::new(
                            std::fs::File::open(path).unwrap(),
                        ))
                        .unwrap()
                        .repeat_infinite();
                        let sink = Sink::connect_new(stream_handle.mixer());
                        sink.pause();
                        sink.set_volume(volume / 100.0);
                        sink.append(input);
                        (
                            id,
                            (chrono::Local::now().with_time(time).unwrap(), enabled, sink),
                        )
                    },
                )
                .collect();
            loop {
                if let Ok(a) = r.try_recv() {
                    match a {
                        Alert::AlarmSet(id, alarm_edit) => {
                            if let Some(a) = alarms.get_mut(&id) {
                                match alarm_edit {
                                    AlarmEdit::Time(naive_time) => {
                                        a.0 = chrono::Local::now().with_time(naive_time).unwrap();
                                    }
                                    AlarmEdit::Name(_) => {}
                                    AlarmEdit::Sound(sound) => {
                                        let is_paused = a.2.is_paused();
                                        a.2.clear();
                                        let path = sounds.get(&sound).unwrap().path.clone();
                                        let input = decoder::Decoder::new(BufReader::new(
                                            std::fs::File::open(path.clone()).unwrap(),
                                        ))
                                        .unwrap()
                                        .repeat_infinite();
                                        a.2.append(input);
                                        a.2.pause();
                                        if !is_paused {
                                            a.2.play();
                                        }
                                    }

                                    AlarmEdit::Volume(volume) => a.2.set_volume(volume / 100.),
                                    AlarmEdit::Enable(enable) => {
                                        if !enable {
                                            a.2.stop();
                                        }
                                        a.1 = enable;
                                    }
                                }
                            }
                        }
                        Alert::AlaramAdded(alarm) => {
                            let path = sounds.get(&alarm.sound).unwrap().path.clone();
                            let input = decoder::Decoder::new(BufReader::new(
                                std::fs::File::open(path.clone()).unwrap(),
                            ))
                            .unwrap()
                            .repeat_infinite();
                            let sink = Sink::connect_new(stream_handle.mixer());
                            sink.set_volume(alarm.volume / 100.0);
                            sink.append(input);
                            sink.pause();
                            alarms.insert(
                                alarm.id,
                                (
                                    chrono::Local::now().with_time(alarm.time).unwrap(),
                                    true,
                                    sink,
                                ),
                            );
                        }
                        Alert::AlarmRemoved(id) => {
                            if let Some(a) = alarms.remove(&id) {
                                a.2.stop();
                            }
                        }
                        Alert::SoundAdded(sound) => {
                            sounds.insert(sound.name.clone(), sound);
                        }
                        Alert::SoundRemoved(id) => {
                            sounds.remove(&id);
                        }
                        Alert::AlarmRinging(_) => {}
                        Alert::AlarmStopped(id) => {
                            if let Some(a) = alarms.get_mut(&id) {
                                a.2.stop();
                            }
                        }
                    }
                }
                // TODO: iter over alarms and see if any of them need to ring and play, and unmute,
                // and send ringing alert
                // maybe also unmute if any alarm is ringing
                let now = chrono::Local::now() - Duration::minutes(2);
                for (id, alarm) in alarms
                    .iter_mut()
                    .filter(|(_, alarm)| alarm.1 && alarm.0 > now)
                    .filter(|(_, alarm)| alarm.2.is_paused())
                {
                    s.broadcast_blocking(Alert::AlarmRinging(*id));
                    alarm.2.play();
                    cpvc::set_mute(false);
                }
            }
        });
    }

    {
        let mut r = r.new_receiver();
        thread::spawn(move || {
            loop {
                if let Ok(m) = r.try_recv() {
                    println!("alart");
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
                            }
                            config.save(Config::config_path());
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
                            config.alarms.insert(alarm);
                            config.save(Config::config_path());
                        }
                        Alert::AlarmRemoved(id) => {
                            config.alarms.data.remove(&id).unwrap();
                            config.save(Config::config_path());
                        }
                        Alert::SoundAdded(sound) => {
                            config.sounds.sounds.insert(sound.name.clone(), sound);
                            config.save(Config::config_path());
                        }
                        Alert::SoundRemoved(sound) => {
                            config.sounds.sounds.remove(&sound).unwrap();
                            config.save(Config::config_path());
                        }
                        Alert::AlarmRinging(_) => {}
                        Alert::AlarmStopped(_id) => {}
                    }
                    println!("{config:?}");
                }
                if let Ok(ServerCommand { kind, reciever }) = r_server.try_recv() {
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
    // problem with the crossbeam channel is that a message can only be read once (I think), so we
    // need an alert reciever for each client, and the main server thread will send to all these
    // recievers the alert, instead of the alert coming from the client thread that it got the
    // message from over ipc
    // so will need a new servercommand to add a new reciever to get alerts (to init a new client)
    // and also servercommands for any alert sent from the client
    // also from alarm thread will need connection to server thread to tell when alarm ringing
    // main problem is that crossbeam is not a broadcaster channel(and bus is to limited)
    for conn in listener.incoming().filter_map(handle_error) {
        // TODO: handle alerts from other threads, has to have access to writer
        let (s, mut _r) = (s.clone(), r.new_receiver());
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
                // println!("waiting");
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
                            s.broadcast_blocking(Alert::AlarmSet(alarm, alarm_edit))
                                .unwrap();
                        }
                        ClientMessage::AddAlarm(alarm) => {
                            s.try_broadcast(Alert::AlaramAdded(alarm)).unwrap();
                        }
                        ClientMessage::RemoveAlarm(id) => {
                            s.broadcast_blocking(Alert::AlarmRemoved(id)).unwrap();
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
                            s.broadcast_blocking(Alert::SoundAdded(sound)).unwrap();
                        }

                        ClientMessage::RemoveSound(sound) => {
                            s.broadcast_blocking(Alert::SoundRemoved(sound)).unwrap();
                        }
                        ClientMessage::StopAlarm(i) => {
                            s.broadcast_blocking(Alert::AlarmStopped(i)).unwrap();
                        }
                    }
                }
                if let Ok(message) = _r_client.try_recv() {
                    let message = match message {
                        ServerResponce::NewUID(id) => ServerMessage::UID(id),
                        ServerResponce::Alarms(alarms) => ServerMessage::Alarms(alarms),
                        ServerResponce::Sounds(sounds) => ServerMessage::Sounds(sounds),
                    };
                    let message = bitcode::serialize(&message).unwrap();
                    roosty_clockd::write(&mut writer, &message);
                }

                if let Ok(message) = _r.try_recv() {
                    let message = match message {
                        Alert::AlarmSet(id, alarm_edit) => ServerMessage::AlarmSet(id, alarm_edit),
                        Alert::AlaramAdded(alarm) => ServerMessage::AlaramAdded(alarm),
                        Alert::AlarmRemoved(id) => ServerMessage::AlarmRemoved(id),
                        Alert::SoundAdded(sound) => ServerMessage::SoundAdded(sound),
                        Alert::SoundRemoved(sound) => ServerMessage::SoundRemoved(sound),
                        Alert::AlarmRinging(id) => ServerMessage::AlarmRinging(id),
                        Alert::AlarmStopped(id) => ServerMessage::AlarmStopped(id),
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
