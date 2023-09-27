use std::hash::Hasher;

use miniquad::{
    Backend, Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, EventHandler,
    FilterMode, KeyCode, KeyMods, Pipeline, RenderingBackend, ShaderSource, TextureFormat,
    TextureKind, TextureParams, VertexAttribute, VertexFormat,
};

use crate::core::config::BootMode;
use crate::core::hardware::input::InputEvent;
use crate::core::System;
use crate::core::video::Screen;
use crate::framecounter::FrameCounter;
use crate::framelimiter::FrameLimiter;
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

pub struct Application {
    system: Shared<System>,
    ctx: Box<dyn RenderingBackend>,
    pipeline: Pipeline,
    bindings: Bindings,
    framelimiter: FrameLimiter,
    framecounter: FrameCounter,
}

impl Application {
    pub fn new() -> Box<Self> {
        let mut ctx = miniquad::window::new_rendering_backend();

        #[rustfmt::skip] let vertices: [Vertex; 4] = [
            Vertex { pos: Vec2 { x: -1.0, y: -1.0 }, uv: Vec2 { x: 0., y: 1. } },
            Vertex { pos: Vec2 { x: 1.0, y: -1.0 }, uv: Vec2 { x: 1., y: 1. } },
            Vertex { pos: Vec2 { x: 1.0, y: 1.0 }, uv: Vec2 { x: 1., y: 0. } },
            Vertex { pos: Vec2 { x: -1.0, y: 1.0 }, uv: Vec2 { x: 0., y: 0. } },
        ];
        let vertex_buffer = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&vertices),
        );

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&indices),
        );

        let screen = ctx.new_render_texture(TextureParams {
            kind: TextureKind::Texture2D,
            format: TextureFormat::RGBA8,
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            width: 256,
            height: 192 * 2,
            ..Default::default()
        });

        let bindings = Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: vec![screen],
        };

        let shader = ctx.new_shader(
            match ctx.info().backend {
                Backend::OpenGl => ShaderSource::Glsl {
                    vertex: shader::VERTEX,
                    fragment: shader::FRAGMENT,
                },
                _ => unimplemented!(),
            },
            shader::meta(),
        ).unwrap();

        let pipeline = ctx.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_pos", VertexFormat::Float2),
                VertexAttribute::new("in_uv", VertexFormat::Float2),
            ],
            shader,
        );

        Box::new(Self {
            system: System::new(),
            ctx,
            pipeline,
            bindings,
            framelimiter: FrameLimiter::new(),
            framecounter: FrameCounter::new(),
        })
    }

    pub fn boot_game(&mut self, path: &str) {
        self.system.set_game_path(path);
        self.system.set_boot_mode(BootMode::Direct);
        self.system.reset();
    }

    const fn convert(key: KeyCode) -> Option<InputEvent> {
        Some(match key {
            KeyCode::A => InputEvent::A,
            KeyCode::B => InputEvent::B,
            KeyCode::Tab => InputEvent::Select,
            KeyCode::Enter => InputEvent::Start,
            KeyCode::Right => InputEvent::Right,
            KeyCode::Left => InputEvent::Left,
            KeyCode::Up => InputEvent::Up,
            KeyCode::Down => InputEvent::Down,
            KeyCode::E => InputEvent::R,
            KeyCode::W => InputEvent::L,
            _ => return None,
        })
    }
}

impl EventHandler for Application {
    fn update(&mut self) {
        self.framelimiter.run(|| {
            self.system.run_frame();
        })
    }

    fn draw(&mut self) {
        let top = self.system.video_unit.fetch_framebuffer(Screen::Top);
        let bot = self.system.video_unit.fetch_framebuffer(Screen::Bottom);

        self.ctx.texture_update_part(self.bindings.images[0], 0, 0, 256, 192, top);
        self.ctx.texture_update_part(self.bindings.images[0], 0, 192, 256, 192, bot);

        self.ctx.begin_default_pass(Default::default());
        self.ctx.apply_pipeline(&self.pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx.draw(0, 6, 1);
        self.ctx.end_render_pass();
        self.ctx.commit_frame();

        if let Some(_fps) = self.framecounter.inc().fps() {
            dbg!(_fps);
        }
    }

    fn key_down_event(&mut self, keycode: KeyCode, _: KeyMods, _: bool) {
        if let Some(event) = Self::convert(keycode) {
            self.system.input.handle_input(event, true);
        } else {
            match keycode {
                KeyCode::F => {
                    if self.framelimiter.is_fast_forward() {
                        self.framelimiter.set_fast_forward(1.0)
                    } else {
                        self.framelimiter.set_fast_forward(4.0)
                    }
                }
                _ => {}
            }
        }
    }

    fn key_up_event(&mut self, keycode: KeyCode, _: KeyMods) {
        if let Some(event) = Self::convert(keycode) {
            self.system.input.handle_input(event, false);
        }
    }
}

mod shader {
    use miniquad::*;

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
                uniforms: vec![UniformDesc::new("offset", UniformType::Float2)],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub offset: (f32, f32),
    }
}
