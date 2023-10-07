#![allow(
    clippy::upper_case_acronyms,
    clippy::identity_op,
    unused,
    clippy::collapsible_else_if,
    clippy::collapsible_if
)]

use std::fs::File;

use log::LevelFilter;
use simplelog::{ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger};

use crate::application::Application;

mod application;
mod arm;
mod core;
mod framehelper;
mod util;

fn main() {
    color_backtrace::install();
    let config = ConfigBuilder::default().build();
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Trace, config.clone(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Trace, config, File::create("out.log").unwrap()),
    ])
    .unwrap();

    let mut app = Application::new();
    app.boot_game("roms/yuugen-suite.nds");
    app.run();
}
