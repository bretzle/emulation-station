use log::LevelFilter;
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};

use crate::application::Application;

mod application;
mod arm;
mod core;
mod util;

fn main() {
    color_eyre::install().unwrap();
    TermLogger::init(
        LevelFilter::Trace,
        ConfigBuilder::new().add_filter_ignore_str("wgpu").build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let mut app = Application::new();
    app.start();
}
