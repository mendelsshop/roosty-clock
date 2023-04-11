#![warn(clippy::pedantic, clippy::nursery, clippy::cargo)]
#![deny(clippy::use_self, rust_2018_idioms)]

use eframe::egui::{Visuals, self};

/// represnts an alarm
/// contains the time that the alarm should go of at.
/// as well as an optinal sound and name
struct Alarm {
    time: (),
    name: Option<String>,
    /// there is a default sound
    sound: Option<()>,
    snooze_time: (),
    // possibly volume
}

#[doc(hidden)]
#[derive(Default)]
pub struct App {
    /// if true, the app will use the dark theme (default)
    dark_theme: bool,
    alarms: Vec<Alarm>,
}

impl App {
    pub fn new() -> Self {
        Self {
            dark_theme: true,
            alarms: Vec::new(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        if self.dark_theme {
            ctx.set_visuals(Visuals::dark());
        } else {
            ctx.set_visuals(Visuals::light());
        }
    }
}