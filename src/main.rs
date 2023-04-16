use std::error::Error;

use eframe::run_native;
use roosty_clock::App;

fn main() -> Result<(), Box<dyn Error>> {
    // initilize the logger
    simple_file_logger::init_logger!("roosty_clock").expect("couldn't initialize logger");
    // make app trnsparent
    let native_options = eframe::NativeOptions {
        transparent: true,
        ..Default::default()
    };
    // TODO: make config file
    // TODO: check if user has changed time format in config
    // run the gui
    run_native(
        "Roosty Clock",
        native_options,
        Box::new(|cc| {
            Box::new(App::new(
                "%r".to_string(), // rn will just use time with am/pm as default
            ))
        }),
    )
    .map_err(|e| e.into())
}
