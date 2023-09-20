use log::LevelFilter;
use once_cell::sync::Lazy;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};
use std::collections::HashSet;
use std::fs::File;

use crate::application::Application;

mod application;
mod arm;
mod core;
mod util;

fn main() {
    color_eyre::install().unwrap();
    TermLogger::init(
        LevelFilter::Trace,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let mut app = Application::new();
    app.start();
}
