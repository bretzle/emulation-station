use std::hash::{Hash, Hasher};
use gfx::{Bindings, QuadContext};
use gfx::buffer::{BufferLayout, BufferSource, BufferType, BufferUsage};
use gfx::pass::PassAction;
use gfx::pipeline::{BlendFactor, BlendState, BlendValue, Equation, Pipeline, PipelineParams, VertexAttribute, VertexFormat};
use gfx::shader::{ShaderMeta, ShaderSource};
use gfx::texture::{FilterMode, TextureAccess, TextureFormat, TextureParams, TextureWrap};
use gfx::uniform::{UniformBlockLayout, UniformDesc, UniformsSource, UniformType};
use microui::atlas::{ATLAS, ATLAS_FONT, ATLAS_HEIGHT, ATLAS_TEXTURE, ATLAS_WHITE, ATLAS_WIDTH};
use microui::{Color, FontId, Icon, Rect, rect, Vec2};

const VERTEX_SHADER: &str = r#"#version 330
    uniform mat4 projection;

    in vec3 position;
    in vec2 texcoord;
    in vec4 color0;

    out vec2 uv;
    out vec4 color;

    void main() {
        gl_Position = projection * vec4(position, 1);
        color = color0 / 255.0;
        uv = texcoord;
    }"#;

const FRAGMENT_SHADER: &str = r#"#version 330
    in vec4 color;
    in vec2 uv;

    uniform sampler2D Texture;

    void main() {
        vec4 col = texture2D(Texture, uv).aaaa;
        gl_FragColor = vec4(color.rgb, col.a * color.a);
    }"#;

#[repr(C)]
#[derive(Default, Copy, Clone)]
struct DebugVertex {
    pos: [f32; 3],
    tex: [f32; 2],
    color: Color,
}

impl Hash for DebugVertex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let slice = unsafe {
            std::slice::from_raw_parts(self as *const Self as *const u8, std::mem::size_of::<Self>())
        };
        state.write(slice)
    }
}

pub struct Renderer {
    vertices: Vec<DebugVertex>,
    bindings: Bindings,
    pipeline: Pipeline,
    last_hash: u64,
}

impl Renderer {
    pub fn new(ctx: &mut QuadContext) -> Self {
        let atlas = ctx.new_texture(TextureAccess::Static, Some(&ATLAS_TEXTURE), TextureParams {
            format: TextureFormat::Alpha,
            wrap: TextureWrap::Clamp,
            filter: FilterMode::Nearest,
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
        });
        let vbo = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<DebugVertex>(1024));

        let bindings = Bindings {
            vertex_buffers: vec![vbo],
            images: vec![atlas],
        };

        let shader = ctx
            .new_shader(
                ShaderSource {
                    vertex: VERTEX_SHADER,
                    fragment: FRAGMENT_SHADER,
                },
                ShaderMeta {
                    images: vec!["Texture".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![UniformDesc::new("projection", UniformType::Mat4)],
                    },
                }
            )
            .unwrap();

        let pipeline = ctx.new_pipeline_with_params(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("position", VertexFormat::Float3),
                VertexAttribute::new("texcoord", VertexFormat::Float2),
                VertexAttribute::new("color0", VertexFormat::Byte4),
            ],
            shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );

        Self {
            vertices: vec![],
            bindings,
            pipeline,
            last_hash: 0,
        }
    }

    pub const fn get_char_width(_font: FontId, c: char) -> usize { ATLAS[ATLAS_FONT as usize + c as usize].w as usize }

    pub const fn get_font_height(_font: FontId) -> usize { 18 }

    fn push_rect(&mut self, dst: Rect, src: Rect, color: Color) {
        let x = src.x as f32 / ATLAS_WIDTH as f32;
        let y = src.y as f32 / ATLAS_HEIGHT as f32;
        let w = src.w as f32 / ATLAS_WIDTH as f32;
        let h = src.h as f32 / ATLAS_HEIGHT as f32;

        let dx = dst.x as f32;
        let dy = dst.y as f32;
        let dw = dst.w as f32;
        let dh = dst.h as f32;

        // 01
        // 32
        #[rustfmt::skip]
        let vertices = [
            DebugVertex { pos: [dx     , dy     , 0.0], tex: [x    , y    ], color }, // 0
            DebugVertex { pos: [dx + dw, dy     , 0.0], tex: [x + w, y    ], color }, // 1
            DebugVertex { pos: [dx + dw, dy + dh, 0.0], tex: [x + w, y + h], color }, // 2

            DebugVertex { pos: [dx     , dy     , 0.0], tex: [x    , y    ], color }, // 0
            DebugVertex { pos: [dx + dw, dy + dh, 0.0], tex: [x + w, y + h], color }, // 2
            DebugVertex { pos: [dx     , dy + dh, 0.0], tex: [x    , y + h], color }, // 3
        ];

        self.vertices.extend_from_slice(&vertices);
    }

    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.push_rect(rect, ATLAS[ATLAS_WHITE as usize], color)
    }

    pub fn draw_icon(&mut self, id: Icon, r: Rect, color: Color) {
        let src = ATLAS[id as usize];
        let x = r.x + (r.w - src.w) / 2;
        let y = r.y + (r.h - src.h) / 2;
        self.push_rect(rect(x, y, src.w, src.h), src, color);
    }

    pub fn draw_text(&mut self, text: &str, pos: Vec2, color: Color) {
        let mut dst = Rect { x: pos.x, y: pos.y, w: 0, h: 0 };
        for p in text.chars() {
            if (p as usize) < 127 {
                let chr = usize::min(p as usize, 127);
                let src = ATLAS[ATLAS_FONT as usize + chr];
                dst.w = src.w;
                dst.h = src.h;
                self.push_rect(dst, src, color);
                dst.x += dst.w;
            }
        }
    }

    pub fn render(&mut self, ctx: &mut QuadContext) {
        let hash = {
            let mut hasher = seahash::SeaHasher::new();
            self.vertices.hash(&mut hasher);
            hasher.finish()
        };

        if self.last_hash != hash {
            ctx.buffer_update(self.bindings.vertex_buffers[0], BufferSource::slice(&self.vertices));

            ctx.apply_pipeline(&self.pipeline);
            ctx.apply_bindings(&self.bindings);
            ctx.apply_uniforms(UniformsSource::table(&ortho4(-512.0, 512.0, 768.0, 0.0, -1.0, 1.0)));
            ctx.draw(0, self.vertices.len() as i32, 1);
        }

        self.vertices.clear()
    }

    pub fn clear(&mut self) {
        self.vertices.clear()
    }
}

fn ortho4(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> [f32; 16] {
    let width = right - left;
    let height = top - bottom;
    let depth = far - near;
    let r00 = 2.0 / width;
    let r11 = 2.0 / height;
    let r22 = -2.0 / depth;
    let r03 = -(right + left) / width;
    let r13 = -(top + bottom) / height;
    let r23 = -(far + near) / depth;
    [r00, 0.0, 0.0, 0.0, 0.0, r11, 0.0, 0.0, 0.0, 0.0, r22, 0.0, r03, r13, r23, 1.0]
}