#![allow(
    clippy::upper_case_acronyms,
    clippy::identity_op,
    unused,
    clippy::collapsible_else_if,
    clippy::collapsible_if
)]

use std::fs::File;

use color_backtrace::termcolor::ColorChoice;
use log::LevelFilter;
use tinylog::*;

use crate::application::Application;

mod application;
mod arm;
mod core;
mod framehelper;
mod util;

fn main() {
    color_backtrace::install();

    let config = ConfigBuilder::default().build();
    TinyLogger::init(LevelFilter::Trace, config, Some(ColorChoice::Auto), Some("out.log")).unwrap();

    let mut app = Application::new();
    app.boot_game("roms/Pokemon Mystery Dungeon.nds");
    app.run();
}
