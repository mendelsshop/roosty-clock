#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(
    clippy::use_self,
    rust_2018_idioms,
    missing_debug_implementations,
    clippy::missing_panics_doc
)]

use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{BufReader, Write},
    path::PathBuf,
    thread,
};

use clap::{Parser, Subcommand};
use eframe::{egui::ViewportBuilder, run_native};
use rodio::{decoder, Sink, Source};
use roosty_clock::{
    communication::{Message, MessageType},
    config::Config,
    Clock,
};
use {
    interprocess::local_socket::{prelude::*, GenericFilePath, GenericNamespaced, Stream},
    std::io::prelude::*,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Option<Command>,
}
#[derive(Subcommand)]
enum Command {
    Init {
        #[clap(long, short)]
        force: bool,
    },
    NewSound {
        name: String,
        path: PathBuf,
    },
    NewAlarm {
        name: String,
        time: String,
        sound: String,
    },
}
fn main() -> Result<(), Box<dyn Error>> {
    // initilize the logger
    simple_file_logger::init_logger!("roosty_clock").expect("couldn't initialize logger");
    // make app trnsparent
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder {
            transparent: Some(true),
            ..Default::default()
        },
        ..Default::default()
    };

    let args = Args::parse();
    match args.command {
        Some(Command::Init { force }) => {
            if force && Config::is_config_present() || !Config::is_config_present() {
                Config::new().save(Config::config_path());
                // write alarm sounds (from assets folder)
                std::fs::create_dir_all(Config::sounds_path()).unwrap();
                let mut beep_beep_file =
                    fs::File::create(Config::sounds_path().join("beep_beep.mp3")).unwrap();
                beep_beep_file
                    .write_all(std::include_bytes!("../assets/beep_beep.mp3"))
                    .unwrap();
            }
        }
        Some(Command::NewSound { name: _, path: _ }) => {}
        Some(Command::NewAlarm {
            name: _,
            time: _,
            sound: _,
        }) => {}
        None => {}
    }

    // Pick a name.
    let name = if GenericNamespaced::is_supported() {
        "roosty-clockd.sock".to_ns_name::<GenericNamespaced>()?
    } else {
        "/tmp/roosty-clockd.sock".to_fs_name::<GenericFilePath>()?
    };

    // Preemptively allocate a sizeable buffer for receiving. This size should be enough and
    // should be easy to find for the allocator.
    let mut buffer = String::with_capacity(128);

    // Create our connection. This will block until the server accepts our connection, but will
    // fail immediately if the server hasn't even started yet; somewhat similar to how happens
    // with TCP, where connecting to a port that's not bound to any server will send a "connection
    // refused" response, but that will take twice the ping, the roundtrip time, to reach the
    // client.
    let conn = Stream::connect(name)?;
    // Wrap it into a buffered reader right away so that we could receive a single line out of it.
    let mut conn = BufReader::new(conn);

    // Send our message into the stream. This will finish either when the whole message has been
    // sent or if a send operation returns an error. (`.get_mut()` is to get the sender,
    // `BufReader` doesn't implement pass-through `Write`.)
    conn.get_mut().write_all(b"Hello from client!\n")?;

    // We now employ the buffer we allocated prior and receive a single line, interpreting a
    // newline character as an end-of-file (because local sockets cannot be portably shut down),
    // verifying validity of UTF-8 on the fly.
    conn.read_line(&mut buffer)?;

    // Print out the result, getting the newline for free!
    print!("Server answered: {buffer}");
    //{
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let mut alarm_map: HashMap<usize, (f32, Sink)> = HashMap::new();
        let mut ctx = None;
        loop {
            for alarm in &alarm_map {
                cpvc::set_mute(false);
                cpvc::set_system_volume(alarm.1 .0 as u8);
            }

            match rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(Message {
                    kind:
                        MessageType::AlarmTriggered {
                            volume,
                            sound,
                            ctx: _,
                        },
                    alarm_id,
                }) => {
                    println!("alarm {alarm_id} triggered with volume {volume}");
                    // create source that repeatedly plays the sound at the specified volume and play it
                    let input = decoder::Decoder::new(sound).unwrap().repeat_infinite();
                    let sink = Sink::connect_new(stream_handle.mixer());
                    // sink.set_volume(volume / 100.0);
                    sink.append(input);
                    sink.play();
                    cpvc::set_mute(false);
                    // cpvc::set_system_volume(volume as u8);
                    alarm_map.insert(alarm_id, (volume, sink));
                }
                Ok(Message {
                    kind: MessageType::AlarmStopped,
                    alarm_id,
                }) => {
                    if let Some(alarm) = alarm_map.remove(&alarm_id) {
                        println!("alarm {alarm_id} stopped");
                        alarm.1.stop();
                    }
                }
                Ok(Message {
                    kind: MessageType::UpdateCtx(new_ctx),
                    alarm_id: _,
                }) => {
                    // println!("updating context");
                    // if ctx.is_none() {
                    ctx = Some(new_ctx);
                    // }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Err(_) => {}
            }
        }
    });
    // run the gui
    run_native(
        "Roosty Clock",
        native_options,
        Box::new(|_| Ok(Box::new(Clock::new(tx)))),
    )
    .map_err(std::convert::Into::into)
}
