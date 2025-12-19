use std::{fs::File, io::BufReader};

use eframe::egui::Context;

pub struct Message {
    pub kind: MessageType,
    pub alarm_id: usize,
}

impl Message {
    #[must_use]
    pub const fn new(kind: MessageType, alarm_id: usize) -> Self {
        Self { kind, alarm_id }
    }
}

#[derive(Debug)]
pub enum MessageType {
    AlarmTriggered {
        volume: f32,
        sound: BufReader<File>,
        ctx: Context,
    },
    // if the alarm is disabled/removed/time changed
    AlarmStopped,
    UpdateCtx(Context),
}
