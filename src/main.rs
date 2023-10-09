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
use winit::event_loop::EventLoop;

use crate::application::Application;

mod application;
mod arm;
mod core;
mod framehelper;
mod util;
mod renderer;

fn main() {
    color_backtrace::install();

    let config = ConfigBuilder::default().build();
    TinyLogger::init(LevelFilter::Trace, config, Some(ColorChoice::Auto), Some("out.log")).unwrap();

    let mut event_loop = EventLoop::new();
    let mut app = Application::new(&event_loop);
    app.boot_game("roms/Pokemon Mystery Dungeon.nds");
    app.run(&mut event_loop);
}
