use std::time::Instant;

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::{Window, WindowBuilder};

use crate::core::config::BootMode;
use crate::core::hardware::input::InputEvent;
use crate::core::video::Screen;
use crate::core::System;
use crate::util::Shared;

pub struct Application {
    system: Shared<System>,

    event_loop: EventLoop<()>,
    window: Window,
    pixels: Pixels,
}

impl Application {
    pub fn new() -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(256 * 2, 192 * 2 * 2))
            .with_resizable(false)
            .build(&event_loop)
            .unwrap();

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(256, 192 * 2, surface_texture).unwrap()
        };

        Self {
            system: System::new(),
            event_loop,
            window,
            pixels,
        }
    }

    pub fn start(&mut self) {
        self.boot_game("roms/armwrestler.nds");

        self.event_loop.run_return(|event, _, flow| {
            flow.set_poll();
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(new) => {
                        self.pixels.resize_surface(new.width, new.height).unwrap()
                    }
                    WindowEvent::CloseRequested => return flow.set_exit(),
                    WindowEvent::KeyboardInput { input, .. } => {
                        let pressed = matches!(input.state, ElementState::Pressed);
                        if let Some(key) = input.virtual_keycode {
                            if let Some(event) = convert(key) {
                                self.system.input.handle_input(event, pressed);
                            }
                        }
                    }
                    _ => {}
                },
                Event::MainEventsCleared => {
                    let start = Instant::now();
                    self.system.run_frame();
                    self.window.set_title(&format!(
                        "fps: {:.01?}",
                        1.0 / start.elapsed().as_secs_f32()
                    ));
                    let top = self.system.video_unit.fetch_framebuffer(Screen::Top);
                    let bot = self.system.video_unit.fetch_framebuffer(Screen::Bottom);

                    self.pixels.frame_mut()[..256 * 192 * 4].copy_from_slice(top);
                    self.pixels.frame_mut()[256 * 192 * 4..].copy_from_slice(bot);

                    if let Err(err) = self.pixels.render() {
                        error!("Application: {err:?}");
                        return flow.set_exit();
                    }
                }
                _ => {}
            }
        });
    }

    fn boot_game(&mut self, path: &str) {
        self.system.set_game_path(path);
        self.system.set_boot_mode(BootMode::Direct);
        self.system.reset();
    }
}

const fn convert(key: VirtualKeyCode) -> Option<InputEvent> {
    Some(match key {
        VirtualKeyCode::A => InputEvent::A,
        VirtualKeyCode::B => InputEvent::B,
        VirtualKeyCode::Tab => InputEvent::Select,
        VirtualKeyCode::Return => InputEvent::Start,
        VirtualKeyCode::Right => InputEvent::Right,
        VirtualKeyCode::Left => InputEvent::Left,
        VirtualKeyCode::Up => InputEvent::Up,
        VirtualKeyCode::Down => InputEvent::Down,
        VirtualKeyCode::E => InputEvent::R,
        VirtualKeyCode::W => InputEvent::L,
        _ => return None,
    })
}
