use std::{
    fmt,
    path::PathBuf,
    sync::mpsc::{channel, Sender},
    thread,
    time::Duration,
};

use crate::TimeOfDay;

/// represnts an alarm
/// contains the time that the alarm should go of at.
/// as well as an optinal sound and name
#[derive(Debug)]
pub struct Alarm {
    /// the time as in the time of day that the alarm should go off
    pub(crate) time: chrono::NaiveTime,
    pub(crate) name: String,
    /// there is a default sound
    pub(crate) sound: AlarmSound,
    pub(crate) snooze_time: (),
    pub(crate) enabled_days: (),
    pub(crate) enabled: bool,
    pub(crate) time_of_day: TimeOfDay,
    pub(crate) minute: u32,
    pub(crate) hour: u32,
    // possibly volume
    /// the next time the alarm should go off as in the full date and time
    pub(crate) next_alarm: chrono::NaiveDateTime,
    // we need to not only send a message, but the alarm sound as well
    // because the sound may be changed before the alarm goes off
    pub(crate) tx: Sender<Message>,
    rang_today: bool,
}

impl Default for Alarm {
    fn default() -> Self {
        Self::new(
            chrono::NaiveTime::default(),
            String::default(),
            AlarmSound::Ring,
            (),
            (),
            false,
            // the cuurent date,
            TimeOfDay::default(),
        )
    }
}

pub (crate) enum Message {
    Start(AlarmSound),
    Stop,
}

impl Alarm {
    pub(crate) fn new(
        time: chrono::NaiveTime,
        name: String,
        sound: AlarmSound,
        snooze_time: (),
        enabled_days: (),
        enabled: bool,
        time_of_day: TimeOfDay,
    ) -> Self {
        let (tx, t_rx) = channel();
        thread::spawn(move || {
            let mut ring = None;
            loop {
                if let Some(_sound) = ring.clone() {
                    // TODO: play sound
                    println!("ringing");
                }
                match t_rx.recv_timeout(Duration::from_secs(1)) {
                    Ok(Message::Start(sound)) => {
                        println!("alarm started");
                        ring = Some(sound);
                    }
                    Ok(Message::Stop) => {
                        println!("alarm stopped");
                        ring = None;
                    }
                    Err(_) => {}
                }
            }
        });
        let time = if time_of_day == TimeOfDay::PM {
            time + chrono::Duration::hours(12)
        } else {
            time
        };
        Self {
            time,
            name,
            sound,
            snooze_time,
            enabled_days,
            enabled,
            next_alarm:  {
                let alarm_time = time;
                let mut time = chrono::Local::now().naive_local();
                // set time to alarm_time
                if time.time() > alarm_time {
                    time += chrono::Duration::days(1);
                }
                time.date().and_time(alarm_time)
            },
            tx,
            rang_today: false,
            time_of_day,
            minute: 0,
            hour: 0,
        }
    }

    // TODO: create a new method
    pub(crate) fn render_alarm(&mut self, time_format: &str, ui: &mut eframe::egui::Ui) {
        ui.scope(|ui| {
            // gray out color if alarm is disabled
            if !self.enabled {
                let faded = ui.visuals().fade_out_to_color();
                ui.visuals_mut().panel_fill = faded;
            }

            ui.horizontal(|ui| {
                // name
                if self.name.is_empty() {
                    ui.label("alarm");
                } else {
                    ui.label(&self.name);
                }
                // on off button
                ui.checkbox(&mut self.enabled, "enabled");
                if !self.enabled && self.rang_today {
                    self.tx.send(Message::Stop).unwrap();
                    self.rang_today = false;
                }
                ui.label(format!("next alarm: {}", self.next_alarm));
            });
            ui.label(self.time.format(time_format).to_string());
            ui.label(format!("alarm sound: {}", self.sound));
            self.edit_alarm(ui);
        });
    }

    pub(crate) fn ring(&mut self) {
        self.tx.send(Message::Start(self.sound.clone())).unwrap();
        // add a day until the next time the alarm should ring
        self.next_alarm += chrono::Duration::days(1);
        // set rang_today to true so when the alarm is disabled it will stop ringing once
        self.rang_today = true;
    }
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub enum AlarmSound {
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
                Self::Custom(file, name) => format!("{name} ({})", file.to_string_lossy()),
                Self::Ring => stringify!(Ring).to_string(),
                Self::BingBong => stringify!(BingBong).to_string(),
                Self::TickTock => stringify!(TickTock).to_string(),
                Self::Rain => stringify!(Rain).to_string(),
            }
        )
    }
}
