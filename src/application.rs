use log::info;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::core::config::BootMode;
use crate::core::System;
use crate::core::video::Screen;
use crate::util::Shared;

pub struct Application {
    system: Shared<System>,

    framebuffer: Vec<u32>,
    window: Window,
}

impl Application {
    pub fn new() -> Self {
        let opts = WindowOptions {
            scale: Scale::X2,
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
        self.boot_game("roms/armwrestler.nds");
        let start = Instant::now();

        while self.window.is_open() {
            // if start.elapsed() >= Duration::from_secs(5) {
            //     self.system.step();
            //     println!("{:08x}", self.system.arm9.cpu.instruction);
            // } else {
            //     self.system.run_frame();
            // }
            self.system.run_frame();

            let top = self.system.video_unit.fetch_framebuffer(Screen::Top);
            let bot = self.system.video_unit.fetch_framebuffer(Screen::Bottom);
            self.framebuffer[..256 * 192].copy_from_slice(top);
            self.framebuffer[256 * 192..].copy_from_slice(bot);

            // dbg!(top.iter().collect::<HashSet<_>>());
            // dbg!(bot.iter().collect::<HashSet<_>>());

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
