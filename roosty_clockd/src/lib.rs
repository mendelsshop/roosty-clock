use chrono::NaiveTime;
use interprocess::local_socket::SendHalf;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
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
    SoundRemoved(String),
    AlarmRinging(u64),
    AlarmStopped(u64),
    UID(u64),
}

pub fn write(w: &mut SendHalf, message: &[u8]) -> io::Result<usize> {
    let mut len = message.len().to_ne_bytes().to_vec();
    // println!("{len:?}");
    len.extend_from_slice(message);
    // println!("{len:?}");
    w.write(&len)
}
pub fn read<R: Read + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> io::Result<()> {
    let mut header = 0_usize.to_ne_bytes();
    // println!("reading header {header:?}");
    r.read_exact(&mut header)?;
    // println!("read header {header:?}");
    let size = usize::from_ne_bytes(header);
    buf.resize(size, 0);
    r.read_exact(buf.as_mut_slice())
}
