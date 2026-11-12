//! PNG icon loading and drawing for the format chooser.
//!
//! Icons are authored as 64x64 RGBA PNGs under `assets/icons`. They are drawn
//! as-is, so the assets carry the CRT-green color and transparency. Every
//! function here assumes a glow context is current on the calling thread.

use std::path::Path;

use glow::HasContext as _;

use crate::ui::Rect;
use crate::DisplayError;

/// Icons are authored at this size, in pixels.
pub(crate) const ICON_SIZE: f32 = 64.0;

/// One loaded PNG icon.
pub(crate) struct Icon {
    texture: glow::Texture,
    width: i32,
    height: i32,
}

impl Icon {
    /// Load a PNG icon and upload it as an RGBA texture.
    pub(crate) fn load(gl: &glow::Context, path: &Path) -> Result<Self, DisplayError> {
        let image = image::open(path)
            .map_err(|e| DisplayError::Image(format!("load {}: {e}", path.display())))?;
        let rgba = image.to_rgba8();
        let (width, height) = (rgba.width() as i32, rgba.height() as i32);

        let texture = unsafe {
            let texture = gl.create_texture().map_err(DisplayError::Shader)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width,
                height,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba.as_raw())),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.bind_texture(glow::TEXTURE_2D, None);
            texture
        };

        Ok(Self {
            texture,
            width,
            height,
        })
    }

    /// Fit the icon inside `rect` without changing its aspect ratio.
    pub(crate) fn fit(&self, rect: Rect) -> Rect {
        fit_rect(rect, self.width, self.height)
    }
}

/// Fit an `image_w`x`image_h` image inside `rect`, centered and aspect-preserving.
pub(crate) fn fit_rect(rect: Rect, image_w: i32, image_h: i32) -> Rect {
    if rect.w <= 0 || rect.h <= 0 || image_w <= 0 || image_h <= 0 {
        return Rect::new(rect.x, rect.y, 0, 0);
    }
    let scale = (rect.w as f32 / image_w as f32).min(rect.h as f32 / image_h as f32);
    let w = ((image_w as f32 * scale).round() as i32).clamp(1, rect.w);
    let h = ((image_h as f32 * scale).round() as i32).clamp(1, rect.h);
    Rect::new(rect.x + (rect.w - w) / 2, rect.y + (rect.h - h) / 2, w, h)
}

const VERTEX_SRC: &str = r#"#version 330 core
layout (location = 0) in vec2 a_pos;
layout (location = 1) in vec2 a_tex;
out vec2 v_tex;
void main() {
    v_tex = a_tex;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}"#;

const FRAGMENT_SRC: &str = r#"#version 330 core
in vec2 v_tex;
uniform sampler2D u_icon;
out vec4 f_color;
void main() {
    f_color = texture(u_icon, v_tex);
}"#;

/// Floats per vertex: `x, y, u, v`.
const FLOATS_PER_VERTEX: usize = 4;
const BYTES_PER_VERTEX: i32 = (FLOATS_PER_VERTEX * std::mem::size_of::<f32>()) as i32;

/// The shared textured-quad renderer. The viewport is assumed to be set to the
/// format region.
pub(crate) struct IconRenderer {
    program: glow::Program,
    vao: glow::VertexArray,
    #[allow(dead_code)] // kept for a future `destroy`
    vbo: glow::Buffer,
    texture_loc: Option<glow::UniformLocation>,
}

impl IconRenderer {
    pub(crate) fn new(gl: &glow::Context) -> Result<Self, DisplayError> {
        let program = crate::shader::link(gl, VERTEX_SRC, FRAGMENT_SRC)?;
        let texture_loc = unsafe { gl.get_uniform_location(program, "u_icon") };
        let (vao, vbo) = unsafe {
            let vao = gl.create_vertex_array().map_err(DisplayError::Shader)?;
            let vbo = gl.create_buffer().map_err(DisplayError::Shader)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, BYTES_PER_VERTEX, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(
                1,
                2,
                glow::FLOAT,
                false,
                BYTES_PER_VERTEX,
                (2 * std::mem::size_of::<f32>()) as i32,
            );
            gl.bind_vertex_array(None);
            (vao, vbo)
        };
        Ok(Self {
            program,
            vao,
            vbo,
            texture_loc,
        })
    }

    /// Draw `icon` into `rect`, mapping the region to clip space.
    pub(crate) fn draw(&self, gl: &glow::Context, region: Rect, rect: Rect, icon: &Icon) {
        if region.w <= 0 || region.h <= 0 || rect.w <= 0 || rect.h <= 0 {
            return;
        }
        let local_x = (rect.x - region.x) as f32;
        let local_y = (rect.y - region.y) as f32;
        let x0 = 2.0 * local_x / region.w as f32 - 1.0;
        let x1 = 2.0 * (local_x + rect.w as f32) / region.w as f32 - 1.0;
        let y0 = 1.0 - 2.0 * local_y / region.h as f32;
        let y1 = 1.0 - 2.0 * (local_y + rect.h as f32) / region.h as f32;
        let vertices: [f32; 24] = [
            x0, y0, 0.0, 0.0, x1, y0, 1.0, 0.0, x0, y1, 0.0, 1.0, x0, y1, 0.0, 1.0, x1, y0, 1.0,
            0.0, x1, y1, 1.0, 1.0,
        ];
        let bytes = unsafe {
            std::slice::from_raw_parts(
                vertices.as_ptr() as *const u8,
                std::mem::size_of_val(&vertices),
            )
        };
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::DYNAMIC_DRAW);

            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(icon.texture));
            gl.uniform_1_i32(self.texture_loc.as_ref(), 0);

            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            gl.bind_vertex_array(None);
            gl.disable(glow::BLEND);
            gl.use_program(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_rect_centers_in_wide_rect() {
        let fitted = fit_rect(Rect::new(10, 20, 100, 50), 64, 64);
        assert_eq!(fitted, Rect::new(35, 20, 50, 50));
    }

    #[test]
    fn fit_rect_centers_in_tall_rect() {
        let fitted = fit_rect(Rect::new(10, 20, 50, 100), 64, 64);
        assert_eq!(fitted, Rect::new(10, 45, 50, 50));
    }

    #[test]
    fn fit_rect_handles_empty_or_invalid() {
        assert_eq!(
            fit_rect(Rect::new(1, 2, 0, 10), 64, 64),
            Rect::new(1, 2, 0, 0)
        );
        assert_eq!(
            fit_rect(Rect::new(1, 2, 10, 10), 0, 64),
            Rect::new(1, 2, 0, 0)
        );
    }

    #[test]
    fn icon_assets_are_64_square_rgba() {
        let root = std::env::var("REPO_ROOT").expect("REPO_ROOT is set by .cargo/config.toml");
        let assets = std::path::PathBuf::from(root).join("assets/icons");
        for file in ["hello-world.png", "default.png"] {
            let image = image::open(assets.join(file)).expect("open icon");
            assert_eq!(image.width(), 64, "{file} width");
            assert_eq!(image.height(), 64, "{file} height");
            assert!(image.color().has_alpha(), "{file} has no alpha channel");
        }
    }
}
