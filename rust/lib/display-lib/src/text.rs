//! Text rendering with HarfBuzz's GPU library (`hb-gpu`).
//!
//! Glyph outlines are shaped with HarfBuzz, encoded on the CPU into `RGBA16I`
//! blobs, and uploaded into a `GL_TEXTURE_BUFFER` atlas. The fragment shader
//! supplied by HarfBuzz rasterizes them (the Slug algorithm). Every function
//! here assumes a glow context is current on the calling thread (see
//! `architecture.md`, "Text rendering").

use std::collections::HashMap;
use std::path::Path;

use glow::HasContext as _;
use harfbuzz::{ShaderLang, ShaderStage};

use crate::DisplayError;

/// Initial atlas capacity in bytes (a multiple of 8, the `RGBA16I` texel size).
const INITIAL_ATLAS_BYTES: usize = 4096;
/// Bytes per vertex: `pos` + `tex` + `normal` (2 f32 each) + `em_per_pos` (f32)
/// + `glyph_loc` (u32).
const VERTEX_STRIDE: i32 = 32;

/// A loaded font, shaped at the face's units per em.
pub(crate) struct Font {
    _blob: harfbuzz::Blob,
    _face: harfbuzz::Face,
    font: harfbuzz::Font,
    upem: f32,
}

impl Font {
    /// Load a font file.
    pub(crate) fn load(path: &Path) -> Result<Font, DisplayError> {
        let bytes = std::fs::read(path)
            .map_err(|e| DisplayError::Io(format!("read font {}: {e}", path.display())))?;
        let blob = harfbuzz::Blob::from_bytes(&bytes);
        let face = harfbuzz::Face::new(&blob);
        let upem = face.upem();
        if upem == 0 {
            return Err(DisplayError::Font("font has no units per em".to_string()));
        }
        let mut font = harfbuzz::Font::new(&face);
        font.set_scale(upem as i32, upem as i32);
        Ok(Font {
            _blob: blob,
            _face: face,
            font,
            upem: upem as f32,
        })
    }

    /// Shape `text` into glyphs, in font units.
    pub(crate) fn shape(&self, text: &str) -> Vec<Glyph> {
        let mut buffer = harfbuzz::Buffer::new();
        buffer.add_str(text);
        buffer.guess_segment_properties();
        buffer.shape(&self.font);
        let infos = buffer.glyph_infos();
        let positions = buffer.glyph_positions();
        infos
            .iter()
            .zip(positions.iter())
            .map(|(i, p)| Glyph {
                id: i.codepoint,
                x_advance: p.x_advance as f32,
                y_advance: p.y_advance as f32,
                x_offset: p.x_offset as f32,
                y_offset: p.y_offset as f32,
            })
            .collect()
    }
}

/// A shaped glyph, in font units.
pub(crate) struct Glyph {
    pub(crate) id: u32,
    pub(crate) x_advance: f32,
    #[allow(dead_code)] // reserved for vertical text
    pub(crate) y_advance: f32,
    pub(crate) x_offset: f32,
    pub(crate) y_offset: f32,
}

/// A glyph's encoded location and ink extents, in font units.
struct AtlasGlyph {
    loc: u32,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    empty: bool,
}

impl AtlasGlyph {
    fn empty() -> Self {
        AtlasGlyph {
            loc: 0,
            min_x: 0.0,
            max_x: 0.0,
            min_y: 0.0,
            max_y: 0.0,
            empty: true,
        }
    }
}

/// One glyph-quad vertex.
#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    tex: [f32; 2],
    normal: [f32; 2],
    em_per_pos: f32,
    glyph_loc: u32,
}

/// Renders text: a shader program, a growable glyph atlas, and a vertex buffer.
pub(crate) struct TextRenderer {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    atlas_buffer: glow::Buffer,
    atlas_texture: glow::Texture,
    atlas_loc: Option<glow::UniformLocation>,
    mvp_loc: Option<glow::UniformLocation>,
    viewport_loc: Option<glow::UniformLocation>,
    fg_loc: Option<glow::UniformLocation>,
    atlas_data: Vec<u8>,
    atlas_capacity: usize,
    glyphs: HashMap<u32, AtlasGlyph>,
    encoder: harfbuzz::GpuDraw,
    font: Font,
    vertices: Vec<Vertex>,
}

impl TextRenderer {
    /// Build the renderer for the font at `font_path`.
    pub(crate) fn new(gl: &glow::Context, font_path: &Path) -> Result<Self, DisplayError> {
        let font = Font::load(font_path)?;
        let program = crate::shader::link(gl, &assemble_vertex_shader(), &assemble_fragment_shader())?;

        let (vao, vbo) = unsafe {
            let vao = gl.create_vertex_array().map_err(DisplayError::Shader)?;
            let vbo = gl.create_buffer().map_err(DisplayError::Shader)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, VERTEX_STRIDE, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, VERTEX_STRIDE, 8);
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, VERTEX_STRIDE, 16);
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, glow::FLOAT, false, VERTEX_STRIDE, 24);
            gl.enable_vertex_attrib_array(4);
            gl.vertex_attrib_pointer_i32(4, 1, glow::UNSIGNED_INT, VERTEX_STRIDE, 28);
            gl.bind_vertex_array(None);
            (vao, vbo)
        };

        let (atlas_buffer, atlas_texture) = unsafe {
            let buffer = gl.create_buffer().map_err(DisplayError::Shader)?;
            gl.bind_buffer(glow::TEXTURE_BUFFER, Some(buffer));
            gl.buffer_data_size(
                glow::TEXTURE_BUFFER,
                INITIAL_ATLAS_BYTES as i32,
                glow::DYNAMIC_DRAW,
            );
            let texture = gl.create_texture().map_err(DisplayError::Shader)?;
            gl.bind_texture(glow::TEXTURE_BUFFER, Some(texture));
            gl.tex_buffer(glow::TEXTURE_BUFFER, glow::RGBA16I, Some(buffer));
            (buffer, texture)
        };

        let (atlas_loc, mvp_loc, viewport_loc, fg_loc) = unsafe {
            (
                gl.get_uniform_location(program, "hb_gpu_atlas"),
                gl.get_uniform_location(program, "u_matViewProjection"),
                gl.get_uniform_location(program, "u_viewport"),
                gl.get_uniform_location(program, "u_foreground"),
            )
        };

        let encoder = harfbuzz::GpuDraw::new()
            .ok_or_else(|| DisplayError::Font("failed to create hb-gpu encoder".to_string()))?;

        Ok(TextRenderer {
            program,
            vao,
            vbo,
            atlas_buffer,
            atlas_texture,
            atlas_loc,
            mvp_loc,
            viewport_loc,
            fg_loc,
            atlas_data: Vec::new(),
            atlas_capacity: INITIAL_ATLAS_BYTES,
            glyphs: HashMap::new(),
            encoder,
            font,
            vertices: Vec::new(),
        })
    }

    /// Draw `text` centered in a `width`x`height` pixel region. The viewport is
    /// assumed to be set to that region.
    pub(crate) fn draw(
        &mut self,
        gl: &glow::Context,
        width: i32,
        height: i32,
        text: &str,
        color: [f32; 4],
        font_size_px: f32,
    ) {
        if width <= 0 || height <= 0 || font_size_px <= 0.0 {
            return;
        }
        let glyphs = self.font.shape(text);
        for glyph in &glyphs {
            self.ensure_glyph(gl, glyph.id);
        }
        self.vertices = build_vertices(
            self.font.upem,
            &self.glyphs,
            &glyphs,
            width,
            height,
            font_size_px,
        );
        if self.vertices.is_empty() {
            return;
        }

        let mvp = ortho_pixels(width, height);
        let vertex_count = self.vertices.len() as i32;
        let bytes = unsafe {
            std::slice::from_raw_parts(
                self.vertices.as_ptr() as *const u8,
                self.vertices.len() * std::mem::size_of::<Vertex>(),
            )
        };
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::DYNAMIC_DRAW);

            gl.use_program(Some(self.program));
            gl.uniform_matrix_4_f32_slice(self.mvp_loc.as_ref(), false, &mvp);
            gl.uniform_2_f32(self.viewport_loc.as_ref(), width as f32, height as f32);
            gl.uniform_4_f32(self.fg_loc.as_ref(), color[0], color[1], color[2], color[3]);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_BUFFER, Some(self.atlas_texture));
            gl.uniform_1_i32(self.atlas_loc.as_ref(), 0);

            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, vertex_count);
            gl.bind_vertex_array(None);
            gl.disable(glow::BLEND);
            gl.use_program(None);
        }
    }

    /// Make sure the atlas holds `gid`, encoding and uploading it on first use.
    fn ensure_glyph(&mut self, gl: &glow::Context, gid: u32) {
        if self.glyphs.contains_key(&gid) {
            return;
        }
        self.encoder.glyph(&self.font.font, gid);
        let (bytes, extents) = self.encoder.encode();
        if bytes.is_empty() {
            self.glyphs.insert(gid, AtlasGlyph::empty());
            return;
        }

        let loc = (self.atlas_data.len() / 8) as u32;
        self.atlas_data.extend_from_slice(&bytes);
        unsafe {
            gl.bind_buffer(glow::TEXTURE_BUFFER, Some(self.atlas_buffer));
            if self.atlas_data.len() > self.atlas_capacity {
                self.atlas_capacity = (self.atlas_capacity * 2).max(self.atlas_data.len());
                gl.buffer_data_size(
                    glow::TEXTURE_BUFFER,
                    self.atlas_capacity as i32,
                    glow::DYNAMIC_DRAW,
                );
                gl.buffer_sub_data_u8_slice(glow::TEXTURE_BUFFER, 0, &self.atlas_data);
            } else {
                gl.buffer_sub_data_u8_slice(glow::TEXTURE_BUFFER, (loc as i32) * 8, &bytes);
            }
        }

        self.glyphs.insert(
            gid,
            AtlasGlyph {
                loc,
                min_x: extents.x_bearing as f32,
                max_x: (extents.x_bearing + extents.width) as f32,
                min_y: (extents.y_bearing + extents.height) as f32,
                max_y: extents.y_bearing as f32,
                empty: false,
            },
        );
    }
}

/// Lay out `glyphs` centered in the region and emit their quad vertices.
fn build_vertices(
    upem: f32,
    atlas: &HashMap<u32, AtlasGlyph>,
    glyphs: &[Glyph],
    width: i32,
    height: i32,
    font_size_px: f32,
) -> Vec<Vertex> {
    let scale = font_size_px / upem;
    let em_per_pos = upem / font_size_px;
    let total: f32 = glyphs.iter().map(|g| g.x_advance).sum::<f32>() * scale;
    let mut pen_x = (width as f32 - total) / 2.0;
    let pen_y = height as f32 / 2.0 + 0.5 * font_size_px;

    let mut vertices = Vec::with_capacity(glyphs.len() * 6);
    for glyph in glyphs {
        if let Some(glyph_atlas) = atlas.get(&glyph.id) {
            if !glyph_atlas.empty {
                push_quad(
                    &mut vertices,
                    glyph_atlas,
                    pen_x + scale * glyph.x_offset,
                    pen_y - scale * glyph.y_offset,
                    scale,
                    em_per_pos,
                );
            }
        }
        pen_x += glyph.x_advance * scale;
    }
    vertices
}

/// Append the two triangles of one glyph quad.
fn push_quad(
    vertices: &mut Vec<Vertex>,
    glyph: &AtlasGlyph,
    pen_x: f32,
    pen_y: f32,
    scale: f32,
    em_per_pos: f32,
) {
    for (cx, cy) in [
        (0.0f32, 0.0f32),
        (1.0, 0.0),
        (0.0, 1.0),
        (0.0, 1.0),
        (1.0, 0.0),
        (1.0, 1.0),
    ] {
        let ex = glyph.min_x + (glyph.max_x - glyph.min_x) * cx;
        let ey = glyph.min_y + (glyph.max_y - glyph.min_y) * cy;
        vertices.push(Vertex {
            pos: [pen_x + scale * ex, pen_y - scale * ey],
            tex: [ex, ey],
            normal: [2.0 * cx - 1.0, 1.0 - 2.0 * cy],
            em_per_pos,
            glyph_loc: glyph.loc,
        });
    }
}

/// Orthographic projection mapping region pixels (top-left origin) to NDC,
/// column-major.
fn ortho_pixels(width: i32, height: i32) -> [f32; 16] {
    let w = width as f32;
    let h = height as f32;
    [
        2.0 / w,
        0.0,
        0.0,
        0.0,
        0.0,
        -2.0 / h,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        -1.0,
        1.0,
        0.0,
        1.0,
    ]
}

fn assemble_vertex_shader() -> String {
    let shared = harfbuzz::shader_source(ShaderStage::Vertex, ShaderLang::Glsl).unwrap_or("");
    let draw = harfbuzz::draw_shader_source(ShaderStage::Vertex, ShaderLang::Glsl).unwrap_or("");
    format!("#version 330\n{shared}\n{draw}\n{VERTEX_MAIN}")
}

fn assemble_fragment_shader() -> String {
    let shared = harfbuzz::shader_source(ShaderStage::Fragment, ShaderLang::Glsl).unwrap_or("");
    let draw = harfbuzz::draw_shader_source(ShaderStage::Fragment, ShaderLang::Glsl).unwrap_or("");
    format!("#version 330\n{shared}\n{draw}\n{FRAGMENT_MAIN}")
}

const VERTEX_MAIN: &str = r#"
layout(location = 0) in vec2 a_position;
layout(location = 1) in vec2 a_texcoord;
layout(location = 2) in vec2 a_normal;
layout(location = 3) in float a_emPerPos;
layout(location = 4) in uint a_glyphLoc;
uniform mat4 u_matViewProjection;
uniform vec2 u_viewport;
out vec2 v_texcoord;
flat out uint v_glyphLoc;
void main() {
    vec2 pos = a_position;
    vec2 tex = a_texcoord;
    vec4 jac = vec4(a_emPerPos, 0.0, 0.0, -a_emPerPos);
    hb_gpu_dilate(pos, tex, a_normal, jac, u_matViewProjection, u_viewport);
    gl_Position = u_matViewProjection * vec4(pos, 0.0, 1.0);
    v_texcoord = tex;
    v_glyphLoc = a_glyphLoc;
}"#;

const FRAGMENT_MAIN: &str = r#"
uniform vec4 u_foreground;
in vec2 v_texcoord;
flat in uint v_glyphLoc;
out vec4 f_color;
void main() {
    float cov = hb_gpu_draw(v_texcoord, v_glyphLoc);
    f_color = vec4(u_foreground.rgb, u_foreground.a * cov);
}"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_vertices_centers_and_offsets() {
        let upem = 1000.0;
        let mut atlas = HashMap::new();
        atlas.insert(
            1u32,
            AtlasGlyph {
                loc: 0,
                min_x: 0.0,
                max_x: 100.0,
                min_y: -200.0,
                max_y: 0.0,
                empty: false,
            },
        );
        atlas.insert(2u32, AtlasGlyph::empty());

        let glyphs = vec![
            Glyph {
                id: 1,
                x_advance: 500.0,
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
            Glyph {
                id: 2,
                x_advance: 200.0,
                y_advance: 0.0,
                x_offset: 50.0,
                y_offset: 0.0,
            },
            Glyph {
                id: 1,
                x_advance: 500.0,
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
        ];

        let vertices = build_vertices(upem, &atlas, &glyphs, 1000, 400, 100.0);

        // The empty glyph contributes no quad: two non-empty glyphs, six each.
        assert_eq!(vertices.len(), 12);
        assert_eq!(vertices[0].em_per_pos, 10.0);

        // total advance = (500 + 200 + 500) * (100/1000) = 120; pen starts at
        // (1000 - 120) / 2 = 440; the first glyph has min_x 0 and no offset.
        assert!((vertices[0].pos[0] - 440.0).abs() < 1e-3);
    }

    #[test]
    fn loads_and_shapes_font() {
        let root = std::env::var("REPO_ROOT").expect("REPO_ROOT is set by .cargo/config.toml");
        let path = std::path::PathBuf::from(root)
            .join("assets")
            .join("fonts")
            .join("FiraSans-Regular.ttf");
        let font = Font::load(&path).expect("load font");
        let glyphs = font.shape("Hello, world!");
        assert!(!glyphs.is_empty());
        assert!(glyphs.iter().any(|g| g.x_advance > 0.0));
    }
}
