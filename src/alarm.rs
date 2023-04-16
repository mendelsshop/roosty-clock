use std::{fmt, path::PathBuf};

/// represnts an alarm
/// contains the time that the alarm should go of at.
/// as well as an optinal sound and name
pub(crate) struct Alarm {
    pub(crate) time: chrono::NaiveTime,
    pub(crate) name: Option<String>,
    /// there is a default sound
    pub(crate) sound: AlarmSound,
    pub(crate) snooze_time: (),
    pub(crate) enabled_days: (),
    // time_of_day: TimeOfDay,
    // possibly volume
}

impl Alarm {
    // TODO: create a new method
    pub(crate) fn render_alarm(&self, time_format: &str, ui: &mut eframe::egui::Ui) {
        if let Some(name) = &self.name {
            ui.label(name);
        }
        ui.label(self.time.format(&time_format).to_string());
        ui.label(format!("alarm sound: {}", self.sound));
    }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) enum AlarmSound {
    // TODO: better names/more of them
    #[default]
    Ring,
    BingBong,
    TickTock,
    Rain,
    Custom(PathBuf, String),
}

impl fmt::Display for AlarmSound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                // for custom shows the name and file path, otherwise shows name of alarm
                AlarmSound::Custom(file, name) => format!("{name} ({})", file.to_string_lossy()),
                AlarmSound::Ring => stringify!(Ring).to_string(),
                AlarmSound::BingBong => stringify!(BingBong).to_string(),
                AlarmSound::TickTock => stringify!(TickTock).to_string(),
                AlarmSound::Rain => stringify!(Rain).to_string(),
            }
        )
    }
}
