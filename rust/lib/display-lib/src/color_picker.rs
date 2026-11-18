//! The Lux Color picker overlay.
//!
//! The picker is an opaque black overlay with a hue ring around a
//! saturation/value triangle and Back/Done buttons. Geometry is in physical
//! pixels with a top-left origin. Rendering assumes a glow context is current.

use std::path::Path;

use glow::HasContext as _;

use crate::shader;
use crate::text::{TextRenderer, FONT_FILE};
use crate::ui::{self, Rect};
use crate::DisplayError;

const RING_OUTER_FRACTION: f32 = 0.32;
const RING_INNER_FRACTION: f32 = 0.78;
const TRIANGLE_RADIUS_FRACTION: f32 = 0.86;
const BUTTON_WIDTH: f32 = 120.0;
const BUTTON_HEIGHT: f32 = 44.0;
const BUTTON_GAP: f32 = 12.0;
const BUTTON_MARGIN: f32 = 12.0;
const PREVIEW_HEIGHT: f32 = 36.0;
const WHEEL_MAX_DELTA: f32 = 20.0;

const BACKGROUND: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const BUTTON_BG: [f32; 4] = [0.10, 0.10, 0.13, 1.0];
const BUTTON_FOCUS_BG: [f32; 4] = [0.20, 0.32, 0.24, 1.0];
const TEXT_COLOR: [f32; 4] = [0.95, 0.95, 0.95, 1.0];
const FOCUS_COLOR: [f32; 4] = [0.75, 1.0, 0.80, 1.0];
const MARKER_DARK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const MARKER_LIGHT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const PREVIEW_BORDER: [f32; 4] = [0.65, 0.65, 0.70, 1.0];

/// A focusable picker control.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PickerControl {
    #[default]
    HueRing,
    Triangle,
    Back,
    Done,
}

impl PickerControl {
    /// Cycle focus in the confirmed order: ring, triangle, back, done.
    pub(crate) fn next(self, direction: i32) -> Self {
        let order = [
            PickerControl::HueRing,
            PickerControl::Triangle,
            PickerControl::Back,
            PickerControl::Done,
        ];
        let index = order.iter().position(|item| *item == self).unwrap_or(0);
        let step = if direction < 0 { -1 } else { 1 };
        order[(index as i32 + step).rem_euclid(order.len() as i32) as usize]
    }
}

/// Latches the hue wheel's direction until the stick returns to rest.
#[derive(Debug, Default)]
pub(crate) struct WheelLatch {
    direction: Option<i32>,
}

impl WheelLatch {
    pub(crate) fn clear(&mut self) {
        self.direction = None;
    }

    /// The direction and speed for one stick movement. A zero movement resets
    /// the latch and returns `None`.
    pub(crate) fn motion(&mut self, dx: i32, dy: i32) -> Option<(i32, f32)> {
        if dx == 0 && dy == 0 {
            self.clear();
            return None;
        }
        let direction = *self.direction.get_or_insert_with(|| {
            let projection = dx - dy;
            if projection > 0 {
                1
            } else if projection < 0 {
                -1
            } else if dx.abs() >= dy.abs() {
                if dx >= 0 {
                    1
                } else {
                    -1
                }
            } else if dy >= 0 {
                1
            } else {
                -1
            }
        });
        let magnitude = ((dx * dx + dy * dy) as f32).sqrt().min(WHEEL_MAX_DELTA);
        Some((direction, magnitude))
    }
}

/// What is under a picker point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PickerHit {
    HueRing,
    Triangle,
    Back,
    Done,
    None,
}

/// The picker's geometry for a format region.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PickerLayout {
    pub(crate) region: (i32, i32, i32, i32),
    pub(crate) center: (f32, f32),
    pub(crate) outer: f32,
    pub(crate) inner: f32,
    pub(crate) triangle: [(f32, f32); 3],
    pub(crate) preview: Rect,
    pub(crate) back: Rect,
    pub(crate) done: Rect,
    scale: f32,
}

impl PickerLayout {
    pub(crate) fn new(region: (i32, i32, i32, i32)) -> Self {
        let (x, y, width, height) = region;
        if width <= 0 || height <= 0 {
            let zero = Rect::new(x, y, 0, 0);
            return Self {
                region,
                center: (x as f32, y as f32),
                outer: 0.0,
                inner: 0.0,
                triangle: [(x as f32, y as f32); 3],
                preview: zero,
                back: zero,
                done: zero,
                scale: 1.0,
            };
        }

        let scale = ui::scale_for_height(height);
        let min_dim = width.min(height) as f32;
        let center = (
            x as f32 + width as f32 / 2.0,
            y as f32 + height as f32 * 0.44,
        );
        let outer = min_dim * RING_OUTER_FRACTION;
        let inner = outer * RING_INNER_FRACTION;
        let triangle_radius = inner * TRIANGLE_RADIUS_FRACTION;
        let triangle = [
            (center.0, center.1 - triangle_radius),
            (
                center.0 + triangle_radius * 0.866_025_4,
                center.1 + triangle_radius * 0.5,
            ),
            (
                center.0 - triangle_radius * 0.866_025_4,
                center.1 + triangle_radius * 0.5,
            ),
        ];

        let button_w = (BUTTON_WIDTH * scale).round() as i32;
        let button_h = (BUTTON_HEIGHT * scale).round() as i32;
        let gap = (BUTTON_GAP * scale).round() as i32;
        let margin = (BUTTON_MARGIN * scale).round() as i32;
        let total = button_w * 2 + gap;
        let button_x = (center.0.round() as i32 - total / 2)
            .max(x)
            .min((x + width - total).max(x));
        let preview_y = (center.1 + outer + margin as f32).round() as i32;
        let preview_h = (PREVIEW_HEIGHT * scale).round() as i32;
        let preview = Rect::new(button_x, preview_y, total, preview_h);
        let button_y = preview_y + preview_h + margin;
        let back = Rect::new(button_x, button_y, button_w, button_h);
        let done = Rect::new(button_x + button_w + gap, button_y, button_w, button_h);

        Self {
            region,
            center,
            outer,
            inner,
            triangle,
            preview,
            back,
            done,
            scale,
        }
    }

    pub(crate) fn hit(&self, x: f32, y: f32) -> PickerHit {
        if self.back.contains(x as i32, y as i32) {
            return PickerHit::Back;
        }
        if self.done.contains(x as i32, y as i32) {
            return PickerHit::Done;
        }
        let dx = x - self.center.0;
        let dy = y - self.center.1;
        let radius = (dx * dx + dy * dy).sqrt();
        if radius >= self.inner && radius <= self.outer {
            return PickerHit::HueRing;
        }
        if point_in_triangle((x, y), self.triangle) {
            return PickerHit::Triangle;
        }
        PickerHit::None
    }

    pub(crate) fn hue_at(&self, x: f32, y: f32) -> f32 {
        let angle = (x - self.center.0).atan2(-(y - self.center.1));
        (angle / std::f32::consts::TAU).rem_euclid(1.0)
    }

    pub(crate) fn sv_at(&self, x: f32, y: f32) -> (f32, f32) {
        let weights = barycentric((x, y), self.triangle);
        let saturation = weights[1].clamp(0.0, 1.0);
        let value = (weights[0] + weights[1]).clamp(0.0, 1.0);
        (saturation.min(value), value)
    }

    pub(crate) fn hue_marker(&self, hue: f32) -> (f32, f32) {
        let angle = hue.rem_euclid(1.0) * std::f32::consts::TAU;
        let radius = (self.inner + self.outer) / 2.0;
        (
            self.center.0 + angle.sin() * radius,
            self.center.1 - angle.cos() * radius,
        )
    }

    pub(crate) fn sv_marker(&self, saturation: f32, value: f32) -> (f32, f32) {
        let s = saturation.clamp(0.0, 1.0).min(value.clamp(0.0, 1.0));
        let v = value.clamp(0.0, 1.0);
        let w0 = v - s;
        let w1 = s;
        let w2 = 1.0 - v;
        (
            self.triangle[0].0 * w0 + self.triangle[1].0 * w1 + self.triangle[2].0 * w2,
            self.triangle[0].1 * w0 + self.triangle[1].1 * w1 + self.triangle[2].1 * w2,
        )
    }

    /// Project a point back inside the triangle.
    pub(crate) fn clamp_triangle(&self, x: f32, y: f32) -> (f32, f32) {
        let weights = barycentric((x, y), self.triangle);
        let total = weights.iter().map(|weight| weight.max(0.0)).sum::<f32>();
        if total <= 1e-6 {
            return self.sv_marker(0.0, 0.0);
        }
        let w0 = weights[0].max(0.0) / total;
        let w1 = weights[1].max(0.0) / total;
        let w2 = weights[2].max(0.0) / total;
        (
            self.triangle[0].0 * w0 + self.triangle[1].0 * w1 + self.triangle[2].0 * w2,
            self.triangle[0].1 * w0 + self.triangle[1].1 * w1 + self.triangle[2].1 * w2,
        )
    }

    fn triangle_bounds(&self) -> Rect {
        let (mut min_x, mut min_y) = self.triangle[0];
        let (mut max_x, mut max_y) = self.triangle[0];
        for (x, y) in self.triangle.iter().skip(1) {
            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }
        Rect::new(
            min_x.round() as i32 - 2,
            min_y.round() as i32 - 2,
            (max_x - min_x).round() as i32 + 4,
            (max_y - min_y).round() as i32 + 4,
        )
    }

    fn ring_bounds(&self) -> Rect {
        Rect::new(
            (self.center.0 - self.outer).round() as i32 - 2,
            (self.center.1 - self.outer).round() as i32 - 2,
            (self.outer * 2.0).round() as i32 + 4,
            (self.outer * 2.0).round() as i32 + 4,
        )
    }
}

/// The shared Back/Done row used by Lux Color and Lux Pos.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ButtonRow {
    pub(crate) back: Rect,
    pub(crate) done: Rect,
    pub(crate) font_size: f32,
}

/// The shared Back/Done geometry for a format region.
pub(crate) fn button_row(region: (i32, i32, i32, i32)) -> ButtonRow {
    let layout = PickerLayout::new(region);
    ButtonRow {
        back: layout.back,
        done: layout.done,
        font_size: (18.0 * layout.scale).max(10.0),
    }
}

/// A renderable Lux Color picker.
pub(crate) struct ColorPicker {
    program: glow::Program,
    vao: glow::VertexArray,
    #[allow(dead_code)] // kept for a future destroy
    vbo: glow::Buffer,
    center_loc: Option<glow::UniformLocation>,
    outer_loc: Option<glow::UniformLocation>,
    inner_loc: Option<glow::UniformLocation>,
    tri0_loc: Option<glow::UniformLocation>,
    tri1_loc: Option<glow::UniformLocation>,
    tri2_loc: Option<glow::UniformLocation>,
    hue_loc: Option<glow::UniformLocation>,
    size_loc: Option<glow::UniformLocation>,
    text: TextRenderer,
}

impl ColorPicker {
    pub(crate) fn new(gl: &glow::Context, assets_root: &Path) -> Result<Self, DisplayError> {
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
        let (center_loc, outer_loc, inner_loc, tri0_loc, tri1_loc, tri2_loc, hue_loc, size_loc) = unsafe {
            (
                gl.get_uniform_location(program, "u_center"),
                gl.get_uniform_location(program, "u_outer"),
                gl.get_uniform_location(program, "u_inner"),
                gl.get_uniform_location(program, "u_tri0"),
                gl.get_uniform_location(program, "u_tri1"),
                gl.get_uniform_location(program, "u_tri2"),
                gl.get_uniform_location(program, "u_hue"),
                gl.get_uniform_location(program, "u_size"),
            )
        };
        let text = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self {
            program,
            vao,
            vbo,
            center_loc,
            outer_loc,
            inner_loc,
            tri0_loc,
            tri1_loc,
            tri2_loc,
            hue_loc,
            size_loc,
            text,
        })
    }

    pub(crate) fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        region: (i32, i32, i32, i32),
        hue: f32,
        saturation: f32,
        value: f32,
        output_color: [f32; 3],
        triangle_marker: Option<(f32, f32)>,
        focus: PickerControl,
        captured: bool,
    ) {
        if region.2 <= 0 || region.3 <= 0 {
            return;
        }
        let layout = PickerLayout::new(region);
        let y_gl = framebuffer_height - region.1 - region.3;

        ui::clear_rect(
            gl,
            framebuffer_height,
            Rect::new(region.0, region.1, region.2, region.3),
            BACKGROUND,
        );
        unsafe {
            gl.viewport(region.0, y_gl, region.2, region.3);
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
            gl.use_program(Some(self.program));
            gl.uniform_2_f32(self.size_loc.as_ref(), region.2 as f32, region.3 as f32);
            gl.uniform_2_f32(
                self.center_loc.as_ref(),
                layout.center.0 - region.0 as f32,
                layout.center.1 - region.1 as f32,
            );
            gl.uniform_1_f32(self.outer_loc.as_ref(), layout.outer);
            gl.uniform_1_f32(self.inner_loc.as_ref(), layout.inner);
            for (location, point) in [
                (self.tri0_loc.as_ref(), layout.triangle[0]),
                (self.tri1_loc.as_ref(), layout.triangle[1]),
                (self.tri2_loc.as_ref(), layout.triangle[2]),
            ] {
                gl.uniform_2_f32(
                    location,
                    point.0 - region.0 as f32,
                    point.1 - region.1 as f32,
                );
            }
            gl.uniform_1_f32(self.hue_loc.as_ref(), hue);
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }

        // Focus outline.
        let focus_rect = match focus {
            PickerControl::HueRing => layout.ring_bounds(),
            PickerControl::Triangle => layout.triangle_bounds(),
            PickerControl::Back => layout.back,
            PickerControl::Done => layout.done,
        };
        if focus != PickerControl::Back && focus != PickerControl::Done {
            draw_outline(gl, framebuffer_height, focus_rect, FOCUS_COLOR);
        }

        // Markers.
        let (hx, hy) = layout.hue_marker(hue);
        draw_marker(
            gl,
            framebuffer_height,
            hx,
            hy,
            captured && focus == PickerControl::HueRing,
        );
        let (sx, sy) = triangle_marker.unwrap_or_else(|| layout.sv_marker(saturation, value));
        draw_marker(
            gl,
            framebuffer_height,
            sx,
            sy,
            captured && focus == PickerControl::Triangle,
        );

        // Output color preview.
        ui::clear_rect(
            gl,
            framebuffer_height,
            layout.preview,
            [output_color[0], output_color[1], output_color[2], 1.0],
        );
        draw_outline(gl, framebuffer_height, layout.preview, PREVIEW_BORDER);

        // Buttons.
        let button_font = (18.0 * layout.scale).max(10.0);
        draw_button(
            &mut self.text,
            gl,
            framebuffer_height,
            region,
            layout.back,
            "Back",
            focus == PickerControl::Back,
            button_font,
        );
        draw_button(
            &mut self.text,
            gl,
            framebuffer_height,
            region,
            layout.done,
            "Done",
            focus == PickerControl::Done,
            button_font,
        );
    }
}

const QUAD_VERTICES: [[f32; 2]; 6] = [
    [0.0, 0.0],
    [1.0, 0.0],
    [0.0, 1.0],
    [0.0, 1.0],
    [1.0, 0.0],
    [1.0, 1.0],
];

const VERTEX_SRC: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
uniform vec2 u_size;
out vec2 v_pos;
void main() {
    v_pos = a_pos * u_size;
    vec2 p = vec2(a_pos.x, 1.0 - a_pos.y);
    gl_Position = vec4(p * 2.0 - 1.0, 0.0, 1.0);
}"#;

const FRAGMENT_SRC: &str = r#"#version 330 core
in vec2 v_pos;
out vec4 f_color;
uniform vec2 u_center;
uniform float u_outer;
uniform float u_inner;
uniform vec2 u_tri0;
uniform vec2 u_tri1;
uniform vec2 u_tri2;
uniform float u_hue;

const float PI = 3.14159265358979323846;

vec3 hsv2rgb(float h, float s, float v) {
    vec3 k = vec3(1.0, 2.0 / 3.0, 1.0 / 3.0);
    vec3 p = abs(fract(vec3(h) + k) * 6.0 - 3.0);
    return v * mix(vec3(1.0), clamp(p - 1.0, 0.0, 1.0), s);
}

float cross2(vec2 a, vec2 b) {
    return a.x * b.y - a.y * b.x;
}

void main() {
    vec2 p = v_pos;
    vec3 col = vec3(0.0);
    float r = length(p - u_center);
    if (r >= u_inner && r <= u_outer) {
        float a = atan(p.x - u_center.x, -(p.y - u_center.y));
        float h = a / (2.0 * PI);
        if (h < 0.0) h += 1.0;
        col = hsv2rgb(h, 1.0, 1.0);
    } else {
        float d = cross2(u_tri1 - u_tri0, u_tri2 - u_tri0);
        float w0 = cross2(u_tri1 - p, u_tri2 - p) / d;
        float w1 = cross2(u_tri2 - p, u_tri0 - p) / d;
        float w2 = 1.0 - w0 - w1;
        if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) {
            float s = clamp(w1, 0.0, 1.0);
            float v = clamp(w0 + w1, 0.0, 1.0);
            col = hsv2rgb(u_hue, s, v);
        }
    }
    f_color = vec4(col, 1.0);
}"#;

fn point_in_triangle(point: (f32, f32), triangle: [(f32, f32); 3]) -> bool {
    let weights = barycentric(point, triangle);
    weights.iter().all(|weight| *weight >= -1e-5)
}

fn barycentric(point: (f32, f32), triangle: [(f32, f32); 3]) -> [f32; 3] {
    let (x, y) = point;
    let (x0, y0) = triangle[0];
    let (x1, y1) = triangle[1];
    let (x2, y2) = triangle[2];
    let denominator = (y1 - y2) * (x0 - x2) + (x2 - x1) * (y0 - y2);
    if denominator.abs() < 1e-6 {
        return [-1.0; 3];
    }
    let w0 = ((y1 - y2) * (x - x2) + (x2 - x1) * (y - y2)) / denominator;
    let w1 = ((y2 - y0) * (x - x2) + (x0 - x2) * (y - y2)) / denominator;
    [w0, w1, 1.0 - w0 - w1]
}

fn draw_outline(gl: &glow::Context, framebuffer_height: i32, rect: Rect, color: [f32; 4]) {
    for edge in [
        Rect::new(rect.x, rect.y, rect.w, 2),
        Rect::new(rect.x, rect.y + rect.h - 2, rect.w, 2),
        Rect::new(rect.x, rect.y, 2, rect.h),
        Rect::new(rect.x + rect.w - 2, rect.y, 2, rect.h),
    ] {
        ui::clear_rect(gl, framebuffer_height, edge, color);
    }
}

fn draw_marker(gl: &glow::Context, framebuffer_height: i32, x: f32, y: f32, captured: bool) {
    let size = if captured { 12 } else { 9 };
    let dark = Rect::new(
        x.round() as i32 - size / 2,
        y.round() as i32 - size / 2,
        size,
        size,
    );
    let light = Rect::new(dark.x + 2, dark.y + 2, size - 4, size - 4);
    ui::clear_rect(gl, framebuffer_height, dark, MARKER_DARK);
    ui::clear_rect(gl, framebuffer_height, light, MARKER_LIGHT);
}

pub(crate) fn draw_button(
    text: &mut TextRenderer,
    gl: &glow::Context,
    framebuffer_height: i32,
    region: (i32, i32, i32, i32),
    rect: Rect,
    label: &str,
    focused: bool,
    font_size: f32,
) {
    ui::clear_rect(
        gl,
        framebuffer_height,
        rect,
        if focused { BUTTON_FOCUS_BG } else { BUTTON_BG },
    );
    if focused {
        draw_outline(gl, framebuffer_height, rect, FOCUS_COLOR);
    }
    let width = text.text_width(label, font_size);
    let left = (rect.w as f32 - width) / 2.0;
    let top = (rect.h as f32 - font_size) / 2.0;
    text.draw_at(
        gl,
        region.2,
        region.3,
        label,
        TEXT_COLOR,
        font_size,
        (rect.x - region.0) as f32 + left.max(0.0),
        (rect.y - region.1) as f32 + top.max(0.0),
    );
}

fn as_bytes<T: Copy>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> PickerLayout {
        PickerLayout::new((0, 0, 720, 720))
    }

    #[test]
    fn wheel_latch_keeps_direction_through_a_circle() {
        let mut wheel = WheelLatch::default();
        assert_eq!(wheel.motion(20, 0), Some((1, 20.0)));
        assert_eq!(wheel.motion(0, -20), Some((1, 20.0)));
        assert_eq!(wheel.motion(-20, 0), Some((1, 20.0)));
        assert_eq!(wheel.motion(0, 20), Some((1, 20.0)));
    }

    #[test]
    fn wheel_latch_changes_direction_only_after_rest() {
        let mut wheel = WheelLatch::default();
        assert_eq!(wheel.motion(20, 0), Some((1, 20.0)));
        assert_eq!(wheel.motion(0, 0), None);
        assert_eq!(wheel.motion(-20, 0), Some((-1, 20.0)));
    }

    #[test]
    fn wheel_latch_speed_is_capped() {
        let mut wheel = WheelLatch::default();
        assert_eq!(wheel.motion(20, 20), Some((1, 20.0)));
    }

    #[test]
    fn back_is_left_of_done() {
        let layout = layout();
        assert!(layout.back.x + layout.back.w <= layout.done.x);
        assert_eq!(layout.back.y, layout.done.y);
    }

    #[test]
    fn preview_sits_between_the_ring_and_buttons() {
        let layout = layout();
        assert!(layout.preview.y as f32 > layout.center.1 + layout.outer);
        assert!(layout.preview.y + layout.preview.h < layout.back.y);
        assert_eq!(layout.preview.x, layout.back.x);
        assert_eq!(
            layout.preview.w,
            layout.done.x + layout.done.w - layout.back.x
        );
    }

    #[test]
    fn triangle_points_are_clamped_inside() {
        let layout = layout();
        let inside = layout.sv_marker(0.3, 0.7);
        let clamped = layout.clamp_triangle(inside.0, inside.1);
        assert!((clamped.0 - inside.0).abs() < 1e-3);
        assert!((clamped.1 - inside.1).abs() < 1e-3);

        let outside = layout.clamp_triangle(layout.center.0, layout.center.1 + layout.outer);
        assert!(point_in_triangle(outside, layout.triangle));
    }

    #[test]
    fn focus_cycles_in_the_confirmed_order() {
        let mut focus = PickerControl::HueRing;
        for expected in [
            PickerControl::Triangle,
            PickerControl::Back,
            PickerControl::Done,
            PickerControl::HueRing,
        ] {
            focus = focus.next(1);
            assert_eq!(focus, expected);
        }
        assert_eq!(PickerControl::HueRing.next(-1), PickerControl::Done);
    }

    #[test]
    fn ring_and_triangle_hit_tests() {
        let layout = layout();
        let (hx, hy) = layout.hue_marker(0.25);
        assert_eq!(layout.hit(hx, hy), PickerHit::HueRing);
        let (sx, sy) = layout.sv_marker(0.5, 0.75);
        assert_eq!(layout.hit(sx, sy), PickerHit::Triangle);
        assert_eq!(
            layout.hit(layout.back.x as f32 + 2.0, layout.back.y as f32 + 2.0),
            PickerHit::Back
        );
        assert_eq!(
            layout.hit(layout.done.x as f32 + 2.0, layout.done.y as f32 + 2.0),
            PickerHit::Done
        );
    }

    #[test]
    fn hue_and_sv_round_trip_through_markers() {
        let layout = layout();
        let (x, y) = layout.hue_marker(0.4);
        assert!(
            (layout.hue_at(x, y) - 0.4).abs() < 1e-4,
            "{}",
            layout.hue_at(x, y)
        );

        let (x, y) = layout.sv_marker(0.25, 0.75);
        let (s, v) = layout.sv_at(x, y);
        assert!((s - 0.25).abs() < 1e-4, "s {s}");
        assert!((v - 0.75).abs() < 1e-4, "v {v}");
    }
}
