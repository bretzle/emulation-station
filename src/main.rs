use log::LevelFilter;
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
};
use std::fs::File;

use crate::application::Application;

mod application;
mod arm;
mod core;
mod util;

fn main() {
    color_eyre::install().unwrap();
    let config = ConfigBuilder::new()
        .add_filter_ignore_str("wgpu")
        .add_filter_ignore_str("naga")
        .set_time_level(LevelFilter::Off)
        .build();
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Trace,
            config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(LevelFilter::Trace, config, File::create("out.log").unwrap()),
    ])
    .unwrap();

    let mut app = Application::new();
    app.start();
}
