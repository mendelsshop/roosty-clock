use std::{collections::HashMap, error::Error, fs, io::Write, path::PathBuf, thread};

use clap::{command, Parser, Subcommand};
use eframe::{
    egui::{ViewportBuilder, Window},
    run_native,
};
use rodio::{decoder, Sink, Source};
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
// TODO: make sure alarm ring is audible even when the system volume is low or muted
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

    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let mut alarm_map: HashMap<usize, (f32, Sink)> = HashMap::new();
        let mut ctx = None;
        loop {
            for alarm in &alarm_map {
                alarm.1 .1.set_volume(alarm.1 .0 / 100.0);
                // passing this context around makes panic
                // window to turn off the alarm
                Window::new("Alarm Triggered")
                    .auto_sized()
                    .show(ctx.as_ref().unwrap(), |ui| {
                        ui.label(format!(
                            "alarm {} triggered with volume {}",
                            alarm.0, alarm.1 .0
                        ));
                        if ui.button("stop").clicked() {
                            alarm.1 .1.stop();
                        }
                    });
            }
            match rx.recv_timeout(std::time::Duration::from_millis(10)) {
                Ok(Message {
                    kind: MessageType::AlarmTriggered { volume, sound, ctx },
                    alarm_id,
                }) => {
                    println!("alarm {alarm_id} triggered with volume {volume}");
                    // create source that repeatedly plays the sound at the specified volume and play it
                    let input = decoder::Decoder::new(sound).unwrap().repeat_infinite();
                    let sink = Sink::connect_new(stream_handle.mixer());
                    sink.set_volume(volume / 100.0);
                    sink.append(input);
                    sink.play();
                    alarm_map.insert(alarm_id, (volume, sink));
                }
                Ok(Message {
                    kind: MessageType::AlarmStopped,
                    alarm_id,
                }) => {
                    if let Some(alarm) = alarm_map.get(&alarm_id) {
                        println!("alarm {alarm_id} stopped");
                        alarm.1.stop();
                    }
                }
                Ok(Message {
                    kind: MessageType::UpdateCtx(new_ctx),
                    alarm_id,
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
    .map_err(|e| e.into())
}
