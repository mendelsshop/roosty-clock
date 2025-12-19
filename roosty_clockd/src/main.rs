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
            conn.read_to_end(&mut buffer).unwrap();
            let message: ClientMessage = toml::from_slice(&buffer).unwrap();
            match message {
                ClientMessage::GetAlarms => todo!(),
                ClientMessage::SetAlarm(_, _alarm_edit) => todo!(),
                ClientMessage::AddAlarm(_alarm) => todo!(),
                ClientMessage::RemoveAlarm(_) => todo!(),
                ClientMessage::GetSounds(_) => todo!(),
                ClientMessage::AdddSound(_sound) => todo!(),
                ClientMessage::RemoveSound(_) => todo!(),
                ClientMessage::StopAlarm(i) => s.send(Alert::AlarmStopped(i)),
            };

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
    Ok(())
}
