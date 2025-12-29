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
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use eframe::{egui::ViewportBuilder, run_native};
use interprocess::local_socket::{
    prelude::*, GenericFilePath, GenericNamespaced, RecvHalf, SendHalf, Stream,
};
use roosty_clock::{config::Config, Clock};

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

    let conn = get_socket()?;
    let (recv, send) = conn.split();
    let mut recv = BufReader::new(recv);
    let mut send = BufWriter::new(send);
    let alarms = get_alarms(&mut recv, &mut send);
    let sounds = get_sounds(&mut recv, &mut send);

    // Print out the result, getting the newline for free!
    // print!("Server answered: {buffer}");
    //{

    // run the gui
    run_native(
        "Roosty Clock",
        native_options,
        Box::new(|_| Ok(Box::new(Clock::new(send, recv, sounds, alarms)))),
    )
    .map_err(std::convert::Into::into)
}

fn get_alarms(
    recv: &mut BufReader<RecvHalf>,
    send: &mut BufWriter<SendHalf>,
) -> HashMap<u64, roosty_clockd::config::Alarm> {
    roosty_clock::send_to_server(send, roosty_clockd::ClientMessage::GetAlarms).unwrap();

    println!("alarms");
    if let Ok(roosty_clockd::ServerMessage::Alarms(alarms)) =
        roosty_clock::recieve_from_server(recv)
    {
        alarms
    } else {
        panic!()
    }
    // todo!()
}

fn get_sounds(
    recv: &mut BufReader<RecvHalf>,
    send: &mut BufWriter<SendHalf>,
) -> HashMap<String, roosty_clockd::config::Sound> {
    roosty_clock::send_to_server(send, roosty_clockd::ClientMessage::GetSounds);

    println!("sounds");
    if let Ok(roosty_clockd::ServerMessage::Sounds(sounds)) =
        roosty_clock::recieve_from_server(recv)
    {
        sounds
    } else {
        panic!()
    }
}

fn get_socket() -> Result<LocalSocketStream, Box<dyn Error + 'static>> {
    let name = if GenericNamespaced::is_supported() {
        "roosty-clockd.sock".to_ns_name::<GenericNamespaced>()?
    } else {
        "/tmp/roosty-clockd.sock".to_fs_name::<GenericFilePath>()?
    };
    let conn = Stream::connect(name)?;
    Ok(conn)
}
