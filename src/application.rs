use log::info;
use minifb::{Window, WindowOptions};

use crate::core::config::BootMode;
use crate::core::System;
use crate::util::Shared;

pub struct Application {
    system: Shared<System>,

    framebuffer: Vec<u32>,
    window: Window,
}

impl Application {
    pub fn new() -> Self {
        let opts = WindowOptions {
            ..Default::default()
        };
        let window = Window::new("Emulation Station", 256, 192 * 2, opts).unwrap();

        Self {
            system: System::new(),
            framebuffer: vec![0; 256 * 192 * 2],
            window,
        }
    }

    pub fn start(&mut self) {
        self.boot_game("roms/TinyFB.nds");

        while self.window.is_open() {
            info!("frame");
            self.system.run_frame();
            let top = self.system.video_unit.fetch_framebuffer(true);
            let bot = self.system.video_unit.fetch_framebuffer(false);
            self.framebuffer[..256 * 192].copy_from_slice(top);
            self.framebuffer[256 * 192..].copy_from_slice(bot);

            // dbg!(self.framebuffer.iter().filter(|p| **dbg!(p) != 0).count());
            self.window
                .update_with_buffer(&self.framebuffer, 256, 192 * 2)
                .unwrap();
        }
    }

    fn boot_game(&mut self, path: &str) {
        self.system.set_game_path(path);
        self.system.set_boot_mode(BootMode::Direct);
        self.system.reset();
    }
}
