use log::{debug, info, trace};
use minifb::Key::P;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::collections::HashSet;
use std::io::read_to_string;
use std::time::{Duration, Instant};

use crate::core::config::BootMode;
use crate::core::hardware::input::InputEvent;
use crate::core::video::Screen;
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

        while self.window.is_open() {
            let start = Instant::now();
            self.handle_input();
            self.system.run_frame();

            self.window.set_title(&format!(
                "fps: {:.01?}",
                1.0 / start.elapsed().as_secs_f32()
            ));

            let top = self.system.video_unit.fetch_framebuffer(Screen::Top);
            let bot = self.system.video_unit.fetch_framebuffer(Screen::Bottom);
            self.framebuffer[..256 * 192].copy_from_slice(top);
            self.framebuffer[256 * 192..].copy_from_slice(bot);

            self.window
                .update_with_buffer(&self.framebuffer, 256, 192 * 2)
                .unwrap();
        }
    }

    fn handle_input(&mut self) {
        const fn convert(key: Key) -> Option<InputEvent> {
            Some(match key {
                Key::A => InputEvent::A,
                Key::B => InputEvent::B,
                Key::Tab => InputEvent::Select,
                Key::Enter => InputEvent::Start,
                Key::Right => InputEvent::Right,
                Key::Left => InputEvent::Left,
                Key::Up => InputEvent::Up,
                Key::Down => InputEvent::Down,
                Key::E => InputEvent::R,
                Key::W => InputEvent::L,
                _ => return None,
            })
        }
        for key in self.window.get_keys_pressed(KeyRepeat::Yes) {
            if let Some(event) = convert(key) {
                debug!("pressing {key:?}");
                self.system.input.handle_input(event, true)
            }
        }

        for key in self.window.get_keys_released() {
            if let Some(event) = convert(key) {
                debug!("releasing {key:?}");
                self.system.input.handle_input(event, false)
            }
        }
    }

    fn boot_game(&mut self, path: &str) {
        self.system.set_game_path(path);
        self.system.set_boot_mode(BootMode::Direct);
        self.system.reset();
    }
}
