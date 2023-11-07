use std::path::PathBuf;

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

#[derive(Debug, Clone)]
pub enum MessageType {
    AlarmTriggered { volume: f32, sound_path: PathBuf },
    // if the alarm is disabled/removed/time changed
    AlarmStopped,
}
