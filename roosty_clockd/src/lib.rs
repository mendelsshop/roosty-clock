use chrono::NaiveTime;
use interprocess::local_socket::SendHalf;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{self, Read, Write},
};

pub mod config;
#[derive(Debug, Serialize, Deserialize, Clone)]

pub enum ClientMessage {
    Init,
    SetAlarm(u64, AlarmEdit),
    AddAlarm(Alarm),
    RemoveAlarm(u64),
    AddedSounds(Vec<config::Sound>),
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
    Init {
        alarms: HashMap<u64, config::Alarm>,
        sounds: HashMap<String, config::Sound>,
        ringing_alarms: HashSet<u64>,
    },
    AlarmSet(u64, AlarmEdit),
    AlaramAdded(Alarm),
    AlarmRemoved(u64),
    SoundsAdded(Vec<config::Sound>),
    SoundRemoved(String),
    AlarmRinging(u64),
    AlarmStopped(u64),
    UID(u64),
}

pub fn write(w: &mut SendHalf, message: &[u8]) -> io::Result<usize> {
    let mut len = message.len().to_ne_bytes().to_vec();
    len.extend_from_slice(message);
    w.write(&len)
}
pub fn read<R: Read + ?Sized>(r: &mut R, buf: &mut Vec<u8>) -> io::Result<()> {
    let mut header = 0_usize.to_ne_bytes();
    r.read_exact(&mut header)?;
    let size = usize::from_ne_bytes(header);
    buf.resize(size, 0);
    r.read_exact(buf.as_mut_slice())
}
