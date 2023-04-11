use std::error::Error;

use eframe::{run_native, epaint::Vec2};
use roosty_clock::App;

fn main() -> Result<(), Box<dyn Error>> {
    // initilize the logger
    simple_file_logger::init_logger!("roosty_clock").expect("couldn't initialize logger");
    // set app intial size and set transparency
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(Vec2::new(800.0, 600.0)),
        transparent: true,
        ..Default::default()
    };
    // run the gui
    run_native(
        "Roosty Clock",
        native_options,
        Box::new(|cc| Box::new(App::new())),
    ).map_err(|e| e.into())
}
