use std::hash::Hasher;

use gfx::buffer::{Arg, BufferLayout, BufferSource, BufferType, BufferUsage};
use gfx::glue::GlContext;
use gfx::pipeline::{Pipeline, VertexAttribute, VertexFormat};
use gfx::shader::{ShaderMeta, ShaderSource};
use gfx::texture::{FilterMode, TextureAccess, TextureFormat, TextureParams, TextureWrap};
use gfx::{Bindings, QuadContext};
use gfx::pass::PassAction;
use gfx::uniform::{UniformBlockLayout, UniformDesc, UniformsSource, UniformType};
use microui::atlas::{ATLAS, ATLAS_FONT, ATLAS_HEIGHT, ATLAS_TEXTURE, ATLAS_WHITE, ATLAS_WIDTH};
use microui::{Color, Command, FontId, Rect};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::{Window, WindowBuilder};

use crate::core::config::BootMode;
use crate::core::hardware::input::InputEvent;
use crate::core::video::Screen;
use crate::core::System;
use crate::framehelper::FrameHelper;
use crate::renderer::Renderer;
use crate::util::Shared;

#[repr(C)]
struct Vec2 {
    x: f32,
    y: f32,
}

#[repr(C)]
struct Vertex {
    pos: Vec2,
    uv: Vec2,
}

#[rustfmt::skip]
const NORMAL_VERTICES: [Vertex; 6] = [
    Vertex { pos: Vec2 { x: -1.0, y: -1.0 }, uv: Vec2 { x: 0., y: 1. } },
    Vertex { pos: Vec2 { x: 1.0, y: -1.0 }, uv: Vec2 { x: 1., y: 1. } },
    Vertex { pos: Vec2 { x: 1.0, y: 1.0 }, uv: Vec2 { x: 1., y: 0. } },
    Vertex { pos: Vec2 { x: -1.0, y: -1.0 }, uv: Vec2 { x: 0., y: 1. } },
    Vertex { pos: Vec2 { x: 1.0, y: 1.0 }, uv: Vec2 { x: 1., y: 0. } },
    Vertex { pos: Vec2 { x: -1.0, y: 1.0 }, uv: Vec2 { x: 0., y: 0. } },
];
#[rustfmt::skip]
const DEBUGGER_VERTICES: [Vertex; 6] = [
    Vertex { pos: Vec2 { x: -1.0, y: -1.0 }, uv: Vec2 { x: 0., y: 1. } },
    Vertex { pos: Vec2 { x: 0.0, y: -1.0 }, uv: Vec2 { x: 1., y: 1. } },
    Vertex { pos: Vec2 { x: 0.0, y: 1.0 }, uv: Vec2 { x: 1., y: 0. } },
    Vertex { pos: Vec2 { x: -1.0, y: -1.0 }, uv: Vec2 { x: 0., y: 1. } },
    Vertex { pos: Vec2 { x: 0.0, y: 1.0 }, uv: Vec2 { x: 1., y: 0. } },
    Vertex { pos: Vec2 { x: -1.0, y: 1.0 }, uv: Vec2 { x: 0., y: 0. } },
];

pub struct Application {
    system: Shared<System>,
    ctx: QuadContext,
    gl: GlContext,
    window: Window,
    pipeline: Pipeline,
    bindings: Bindings,
    framehelper: FrameHelper,
    last: u64,
    in_debugger: bool,
    microui: microui::Context,
    renderer: Renderer,
}

impl Application {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = WindowBuilder::new()
            .with_inner_size(PhysicalSize::new(256 * 2, 192 * 2 * 2))
            .with_resizable(false)
            .build(&event_loop)
            .unwrap();
        let gl = unsafe { GlContext::create(Default::default(), &window).unwrap() };
        gl.make_current();
        gl.set_swap_interval(true);

        let mut ctx = QuadContext::new(gl.glow());

        let vertex_buffer = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&NORMAL_VERTICES));

        let screen = ctx.new_texture(
            TextureAccess::RenderTarget,
            None,
            TextureParams {
                format: TextureFormat::RGBA8,
                filter: FilterMode::Nearest,
                width: 256,
                height: 192 * 2,
                ..Default::default()
            },
        );

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            images: vec![screen],
        };

        let shader = ctx
            .new_shader(
                ShaderSource {
                    vertex: shader::VERTEX,
                    fragment: shader::FRAGMENT,
                },
                shader::meta(),
            )
            .unwrap();

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
        );

        let renderer = Renderer::new(&mut ctx);

        Self {
            system: System::new(),
            ctx,
            gl,
            window,
            pipeline,
            bindings,
            framehelper: FrameHelper::new(),
            last: 0,
            in_debugger: false,
            microui: microui::Context::new(Renderer::get_char_width, Renderer::get_font_height),
            renderer,
        }
    }

    pub fn boot_game(&mut self, path: &str) {
        self.system.set_game_path(path);
        self.system.set_boot_mode(BootMode::Direct);
        self.system.reset();
    }

    pub fn run(&mut self, event_loop: &mut EventLoop<()>) {
        self.center_window();
        let _ = event_loop.run_return(|event, _, flow| match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => flow.set_exit(),
                WindowEvent::Resized(new) => self.ctx.resize(new.width as _, new.height as _),
                WindowEvent::KeyboardInput { input, .. } => {
                    let pressed = matches!(input.state, ElementState::Pressed);
                    if let Some(code) = input.virtual_keycode {
                        match code {
                            VirtualKeyCode::Minus => self.framehelper.set_fast_forward(1.0),
                            VirtualKeyCode::Equals => self.framehelper.set_fast_forward(2.0),
                            VirtualKeyCode::RBracket => {
                                if pressed {
                                    self.toggle_debugger();
                                    self.center_window();
                                }
                            },
                            _ => {
                                if let Some(event) = Self::convert(code) {
                                    self.system.input.handle_input(event, pressed);
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                self.framehelper.run(|| {
                    self.system.run_frame();
                    if self.in_debugger {
                        self.microui.frame(|ui| {
                            ui.window("hello").size(512, 768).show(ui, |ui| {});
                        });
                    }
                });
            }
            Event::RedrawEventsCleared => {
                let top = self.system.video_unit.fetch_framebuffer(Screen::Top);
                let bot = self.system.video_unit.fetch_framebuffer(Screen::Bottom);

                let hash = {
                    let mut h = seahash::SeaHasher::new();
                    h.write(top);
                    h.write(bot);
                    h.finish()
                };

                if self.last != hash {
                    self.last = hash;
                    self.ctx.texture_update_part(self.bindings.images[0], 0, 0, 256, 192, top);
                    self.ctx.texture_update_part(self.bindings.images[0], 0, 192, 256, 192, bot);

                    self.ctx.begin_default_pass(Default::default());
                    self.ctx.apply_pipeline(&self.pipeline);
                    self.ctx.apply_bindings(&self.bindings);
                    self.ctx.draw(0, 6, 1);

                    if self.in_debugger {
                        self.draw_debugger();
                        self.renderer.render(&mut self.ctx)
                    }

                    self.ctx.end_render_pass();
                    self.ctx.commit_frame();

                    self.gl.swap_buffers();
                }

                if let Some((fps, ups)) = self.framehelper.inc().fps() {
                    self.window.set_title(&format!("fps: {fps} ups: {ups}"))
                }
            }
            _ => {}
        });
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

    fn toggle_debugger(&mut self) {
        let mut size = self.window.inner_size();
        if self.in_debugger {
            size.width /= 2
        } else {
            size.width *= 2
        }
        self.window.set_inner_size(size);

        let data = if self.in_debugger {
            &NORMAL_VERTICES
        } else {
            &DEBUGGER_VERTICES
        };
        self.ctx.buffer_update(self.bindings.vertex_buffers[0], BufferSource::slice(data));

        self.in_debugger ^= true;
        self.renderer.clear();
        self.last = 0xdeadbeeef_8008135; // force a redraw
    }

    fn center_window(&self) {
        let monitor_size = self.window.current_monitor().unwrap().size();
        let window_size = self.window.outer_size();

        let pos = PhysicalPosition::new(
            monitor_size.width / 2 - window_size.width / 2,
            monitor_size.height / 2 - window_size.height / 2,
        );
        self.window.set_outer_position(pos);
    }

    fn draw_debugger(&mut self) {
        for &cmd in self.microui.commands() {
            match cmd {
                Command::Clip { .. } => todo!(),
                Command::Rect { rect, color } => self.renderer.draw_rect(rect, color),
                Command::Text { str_start, str_len, pos, color, .. } => {
                    let str = &self.microui.text_stack[str_start..str_start + str_len];
                    self.renderer.draw_text(str, pos, color)
                }
                Command::Icon { rect, id, color } => self.renderer.draw_icon(id, rect, color),
            }
        }
    }
}

mod shader {
    use gfx::shader::ShaderMeta;
    use gfx::uniform::{UniformBlockLayout, UniformDesc, UniformType};

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 in_pos;
    attribute vec2 in_uv;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = vec4(in_pos, 0, 1);
        texcoord = in_uv;
    }"#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;

    uniform sampler2D tex;

    void main() {
        gl_FragColor = texture2D(tex, texcoord);
    }"#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![],
            },
        }
    }
}
