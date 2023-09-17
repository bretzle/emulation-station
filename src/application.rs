use minifb::{Window, WindowOptions};

use crate::core::config::BootMode;
use crate::core::System;
use crate::util::Shared;

pub struct Application {
    system: Shared<System>,

    window: Window,
}

impl Application {
    pub fn new() -> Self {
        let opts = WindowOptions {
            ..Default::default()
        };
        let window = Window::new("Emulation Station", 500, 500, opts).unwrap();

        Self {
            system: System::new(),
            window,
        }
    }

    pub fn start(&mut self) {
        self.boot_game("roms/armwrestler.nds");

        while self.window.is_open() {
            self.system.run_frame();
            self.window.update();
        }
    }

    fn boot_game(&mut self, path: &str) {
        self.system.set_game_path(path);
        self.system.set_boot_mode(BootMode::Direct);
    }
}
