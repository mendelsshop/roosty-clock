use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod config;
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "t", content = "c")]
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
    #[serde(with = "toml_datetime_compat")]
    pub time: NaiveTime,
    pub volume: f32,
    pub sound: String,
    pub id: u64,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AlarmEdit {
    #[serde(with = "toml_datetime_compat")]
    Time(NaiveTime),
    Name(Option<String>),
    Sound(String),
    Volume(f32),
    Enable(bool),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "t", content = "c")]
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
