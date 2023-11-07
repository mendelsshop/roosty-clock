use std::{collections::HashMap, error::Error, path::PathBuf, thread};

use clap::{command, Parser, Subcommand};
use eframe::run_native;
use roosty_clock::{
    communication::{Message, MessageType},
    config::Config,
    Clock,
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
        transparent: true,
        ..Default::default()
    };

    let args = Args::parse();
    match args.command {
        Some(Command::Init { force }) => {
            if force && Config::is_config_present() || !Config::is_config_present() {
                Config::new().save(Config::config_path());
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

    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut alarm_map = HashMap::new();
        loop {
            match rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(Message {
                    kind: MessageType::AlarmTriggered { volume, sound_path },
                    alarm_id,
                }) => {
                    println!("alarm {alarm_id} triggered with volume {volume}");
                    alarm_map.insert(alarm_id, (volume, sound_path));
                }
                Ok(Message {
                    kind: MessageType::AlarmStopped,
                    alarm_id,
                }) => {
                    if alarm_map.remove(&alarm_id).is_some() {
                        println!("alarm {alarm_id} stopped");
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                Err(_) => {}
            }
        }
    });
    // TODO: make config file
    // TODO: check if user has changed time format in config
    // run the gui
    run_native(
        "Roosty Clock",
        native_options,
        Box::new(|_| Box::new(Clock::new(tx))),
    )
    .map_err(|e| e.into())
}
