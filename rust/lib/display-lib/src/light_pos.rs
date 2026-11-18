//! The Lux Pos light-position overlay.
//!
//! The globe is rendered in a square above the shared Back/Done row. The light
//! is a view-space direction constrained to the user-facing hemisphere and is
//! shown with an iconic spotlight-can marker pointing at the globe center.

use glow::HasContext as _;

use crate::color_picker::{button_row, ButtonRow};
use crate::earth;
use crate::shader;
use crate::ui::{self, Rect};
use crate::DisplayError;

const GLOBE_MARGIN: f32 = 12.0;
const MARKER_RADIUS_FRACTION: f32 = 1.05;
const SPOTLIGHT_SIZE: f32 = 34.0;

/// A focusable Lux Pos control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LightFocus {
    #[default]
    Back,
    Done,
}

impl LightFocus {
    pub(crate) fn next(self, direction: i32) -> Self {
        match (self, direction < 0) {
            (LightFocus::Back, false) => LightFocus::Done,
            (LightFocus::Done, true) => LightFocus::Back,
            (other, _) => other,
        }
    }
}

/// What is under a Lux Pos point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightHit {
    Back,
    Done,
    None,
}

/// The Lux Pos geometry for a format region.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LightLayout {
    pub(crate) region: (i32, i32, i32, i32),
    pub(crate) globe: Rect,
    pub(crate) center: (f32, f32),
    pub(crate) radius: f32,
    pub(crate) back: Rect,
    pub(crate) done: Rect,
    pub(crate) font_size: f32,
    pub(crate) scale: f32,
}

impl LightLayout {
    pub(crate) fn new(region: (i32, i32, i32, i32)) -> Self {
        let (x, y, width, height) = region;
        if width <= 0 || height <= 0 {
            let zero = Rect::new(x, y, 0, 0);
            return Self {
                region,
                globe: zero,
                center: (x as f32, y as f32),
                radius: 0.0,
                back: zero,
                done: zero,
                font_size: 0.0,
                scale: 1.0,
            };
        }

        let buttons: ButtonRow = button_row(region);
        let scale = ui::scale_for_height(height);
        let margin = (GLOBE_MARGIN * scale).round() as i32;
        let top = y;
        let bottom = buttons.back.y - margin;
        let side = width.min((bottom - top).max(1));
        let globe = Rect::new(x + (width - side) / 2, top, side, side);
        let center = (
            globe.x as f32 + side as f32 / 2.0,
            globe.y as f32 + side as f32 / 2.0,
        );
        let radius = earth::silhouette_radius(side, side);

        Self {
            region,
            globe,
            center,
            radius,
            back: buttons.back,
            done: buttons.done,
            font_size: buttons.font_size,
            scale,
        }
    }

    pub(crate) fn hit(&self, x: i32, y: i32) -> LightHit {
        if self.back.contains(x, y) {
            LightHit::Back
        } else if self.done.contains(x, y) {
            LightHit::Done
        } else {
            LightHit::None
        }
    }

    /// Map a pointer inside the globe area to normalized front-hemisphere x-y.
    pub(crate) fn pointer_xy(&self, x: f32, y: f32) -> (f32, f32) {
        if self.radius <= 0.0 {
            return (0.0, 0.0);
        }
        let dx = (x - self.center.0) / self.radius;
        let dy = (self.center.1 - y) / self.radius;
        clamp_disk(dx, dy)
    }

    /// The projected spotlight position for front-hemisphere x-y.
    pub(crate) fn marker(&self, x: f32, y: f32) -> (f32, f32) {
        let (x, y) = clamp_disk(x, y);
        let marker_radius = self.radius * MARKER_RADIUS_FRACTION;
        (
            self.center.0 + x * marker_radius,
            self.center.1 - y * marker_radius,
        )
    }

    /// The spotlight-can direction in screen space, pointing at the center.
    pub(crate) fn can_angle(&self, marker: (f32, f32)) -> f32 {
        (self.center.1 - marker.1).atan2(self.center.0 - marker.0)
    }

    pub(crate) fn spotlight_size(&self) -> f32 {
        (SPOTLIGHT_SIZE * self.scale).max(12.0)
    }
}

/// Clamp normalized x-y to the unit disk and derive the front-hemisphere z.
pub(crate) fn front_light(x: f32, y: f32) -> (f32, f32, f32) {
    let (x, y) = clamp_disk(x, y);
    let z = (1.0 - x * x - y * y).max(0.0).sqrt();
    (x, y, z)
}

fn clamp_disk(x: f32, y: f32) -> (f32, f32) {
    let radius = (x * x + y * y).sqrt();
    if radius > 1.0 {
        (x / radius, y / radius)
    } else {
        (x, y)
    }
}

/// The iconic 2D spotlight-can renderer.
pub(crate) struct Spotlight {
    program: glow::Program,
    vao: glow::VertexArray,
    #[allow(dead_code)] // kept for a future destroy
    vbo: glow::Buffer,
    marker_loc: Option<glow::UniformLocation>,
    angle_loc: Option<glow::UniformLocation>,
    size_loc: Option<glow::UniformLocation>,
    region_loc: Option<glow::UniformLocation>,
    color_loc: Option<glow::UniformLocation>,
}

impl Spotlight {
    pub(crate) fn new(gl: &glow::Context) -> Result<Self, DisplayError> {
        let program = shader::link(gl, VERTEX_SRC, FRAGMENT_SRC)?;
        let (vao, vbo) = unsafe {
            let vao = gl.create_vertex_array().map_err(DisplayError::Shader)?;
            let vbo = gl.create_buffer().map_err(DisplayError::Shader)?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                as_bytes(&QUAD_VERTICES),
                glow::STATIC_DRAW,
            );
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
            gl.bind_vertex_array(None);
            (vao, vbo)
        };
        let (marker_loc, angle_loc, size_loc, region_loc, color_loc) = unsafe {
            (
                gl.get_uniform_location(program, "u_marker"),
                gl.get_uniform_location(program, "u_angle"),
                gl.get_uniform_location(program, "u_size"),
                gl.get_uniform_location(program, "u_region_size"),
                gl.get_uniform_location(program, "u_color"),
            )
        };
        Ok(Self {
            program,
            vao,
            vbo,
            marker_loc,
            angle_loc,
            size_loc,
            region_loc,
            color_loc,
        })
    }

    pub(crate) fn render(
        &self,
        gl: &glow::Context,
        framebuffer_height: i32,
        region: (i32, i32, i32, i32),
        marker: (f32, f32),
        angle: f32,
        size: f32,
        color: [f32; 3],
    ) {
        if region.2 <= 0 || region.3 <= 0 {
            return;
        }
        let y_gl = framebuffer_height - region.1 - region.3;
        let marker = (marker.0 - region.0 as f32, marker.1 - region.1 as f32);
        unsafe {
            gl.viewport(region.0, y_gl, region.2, region.3);
            gl.disable(glow::DEPTH_TEST);
            gl.enable(glow::BLEND);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(self.region_loc.as_ref(), region.2 as f32, region.3 as f32);
            gl.uniform_2_f32(self.marker_loc.as_ref(), marker.0, marker.1);
            gl.uniform_1_f32(self.angle_loc.as_ref(), angle);
            gl.uniform_1_f32(self.size_loc.as_ref(), size);
            gl.uniform_3_f32(self.color_loc.as_ref(), color[0], color[1], color[2]);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            gl.bind_vertex_array(None);
            gl.use_program(None);
            gl.disable(glow::BLEND);
        }
    }
}

const QUAD_VERTICES: [[f32; 2]; 6] = [
    [-1.0, -1.0],
    [1.0, -1.0],
    [-1.0, 1.0],
    [-1.0, 1.0],
    [1.0, -1.0],
    [1.0, 1.0],
];

const VERTEX_SRC: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
uniform vec2 u_marker;
uniform float u_angle;
uniform float u_size;
uniform vec2 u_region_size;
out vec2 v_local;
void main() {
    v_local = a_pos;
    vec2 local = a_pos * u_size;
    float c = cos(u_angle);
    float s = sin(u_angle);
    vec2 rotated = vec2(local.x * c - local.y * s, local.x * s + local.y * c);
    vec2 p = u_marker + rotated;
    vec2 ndc = vec2(p.x / u_region_size.x * 2.0 - 1.0,
                    1.0 - p.y / u_region_size.y * 2.0);
    gl_Position = vec4(ndc, 0.0, 1.0);
}"#;

const FRAGMENT_SRC: &str = r#"#version 330 core
in vec2 v_local;
out vec4 f_color;
uniform vec3 u_color;
void main() {
    vec2 p = v_local;
    float body = step(abs(p.y), 0.28) * step(-0.75, p.x) * step(p.x, 0.10);
    float nose_half = 0.28 * (1.0 - clamp((p.x - 0.10) / 0.50, 0.0, 1.0));
    float nose = step(abs(p.y), nose_half) * step(0.10, p.x) * step(p.x, 0.60);
    float shape = max(body, nose);
    float lens = step(length(p - vec2(0.55, 0.0)), 0.22);
    vec3 col = mix(vec3(0.62, 0.65, 0.70), u_color, lens);
    f_color = vec4(col, shape);
}"#;

fn as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> LightLayout {
        LightLayout::new((0, 0, 720, 720))
    }

    #[test]
    fn buttons_match_lux_color_and_globe_sits_above() {
        let row = button_row((0, 0, 720, 720));
        let layout = layout();
        assert_eq!(layout.back, row.back);
        assert_eq!(layout.done, row.done);
        assert!(layout.globe.y + layout.globe.h <= layout.back.y);
    }

    #[test]
    fn front_light_clamps_to_the_disk() {
        let (x, y, z) = front_light(2.0, 0.0);
        assert!((x - 1.0).abs() < 1e-6);
        assert!(y.abs() < 1e-6);
        assert!(z.abs() < 1e-6);
        let (_, _, z) = front_light(0.0, 0.0);
        assert!((z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn marker_is_inside_the_globe_area_and_points_inward() {
        let layout = layout();
        let marker = layout.marker(0.5, -0.5);
        assert!(layout.globe.contains(marker.0 as i32, marker.1 as i32));
        let angle = layout.can_angle(marker);
        assert!(angle.cos() < 0.0);
        assert!(angle.sin() < 0.0);
    }

    #[test]
    fn focus_cycles_between_back_and_done() {
        assert_eq!(LightFocus::Back.next(1), LightFocus::Done);
        assert_eq!(LightFocus::Done.next(-1), LightFocus::Back);
        assert_eq!(LightFocus::Done.next(1), LightFocus::Done);
    }
}
