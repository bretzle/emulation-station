use std::fs::File;

use log::LevelFilter;
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
};

use crate::application::Application;

mod application;
mod arm;
mod core;
mod util;
mod framelimiter;
mod framecounter;

fn main() {
    color_backtrace::install();
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

    let conf = miniquad::conf::Conf {
        window_width: 256 * 2,
        window_height: 192 * 2 * 2,
        window_title: "emulation station".to_string(),
        window_resizable: false,
        ..Default::default()
    };

    miniquad::start(conf, || {
        let mut app = Application::new();
        app.boot_game("roms/rockwrestler.nds");
        app
    });
}
