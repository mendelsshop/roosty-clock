use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{self, BufRead, ErrorKind},
};

pub mod config;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ClientMessage {
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
pub struct Alarm {
    pub name: Option<String>,
    pub time: NaiveTime,
    pub volume: f32,
    pub sound: String,
    pub id: u64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AlarmEdit {
    Time(NaiveTime),
    Name(Option<String>),
    Sound(String),
    Volume(f32),
    Enable(bool),
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
pub(crate) fn is_interrupted(e: &io::Error) -> bool {
    match e.kind() {
        ErrorKind::Interrupted => true,
        _ => false,
    }
}
pub fn read<R: BufRead + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> io::Result<usize> {
    let mut read = 0;
    loop {
        let used = {
            println!("got {buf:?}");
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if is_interrupted(e) => {
                    println!("skuo ");
                    continue;
                }
                Err(e) => return Err(e),
            };
            buf.extend_from_slice(available);
            println!("foo");
            available.len()
        };
        r.consume(used);
        read += used;
        // println!("{used} {read:?}");
        if used == 0 {
            return Ok(read);
        }
    }
}
