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
};

use clap::{Parser, Subcommand};
use eframe::{egui::ViewportBuilder, run_native};
use roosty_clock::{
    config::Config,
    Clock,
};
use interprocess::local_socket::{prelude::*, GenericFilePath, GenericNamespaced, Stream};

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

    let mut conn = get_socket()?;
    let alarms = get_alarms(&mut conn);
    let sounds = get_sounds(&mut conn);

    // Print out the result, getting the newline for free!
    // print!("Server answered: {buffer}");
    //{

    // run the gui
    run_native(
        "Roosty Clock",
        native_options,
        Box::new(|_| Ok(Box::new(Clock::new(conn, sounds, alarms)))),
    )
    .map_err(std::convert::Into::into)
}

fn get_alarms(
    conn: &mut BufReader<LocalSocketStream>,
) -> HashMap<u64, roosty_clockd::config::Alarm> {
    roosty_clock::send_to_server(conn, roosty_clockd::ClientMessage::GetAlarms).unwrap();

    println!("alarms");
    if let Ok(roosty_clockd::ServerMessage::Alarms(alarms)) =
        roosty_clock::recieve_from_server(conn)
    {
        alarms
    } else {
        panic!()
    }
    // todo!()
}

fn get_sounds(
    conn: &mut BufReader<LocalSocketStream>,
) -> HashMap<String, roosty_clockd::config::Sound> {
    roosty_clock::send_to_server(conn, roosty_clockd::ClientMessage::GetSounds);

    println!("sounds");
    if let Ok(roosty_clockd::ServerMessage::Sounds(sounds)) =
        roosty_clock::recieve_from_server(conn)
    {
        sounds
    } else {
        panic!()
    }
}

fn get_socket() -> Result<BufReader<LocalSocketStream>, Box<dyn Error + 'static>> {
    let name = if GenericNamespaced::is_supported() {
        "roosty-clockd.sock".to_ns_name::<GenericNamespaced>()?
    } else {
        "/tmp/roosty-clockd.sock".to_fs_name::<GenericFilePath>()?
    };
    let conn = Stream::connect(name)?;
    let conn = BufReader::new(conn);
    Ok(conn)
}
