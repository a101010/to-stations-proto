//! FFI binding to HarfBuzz, including the experimental GPU library
//! (`libharfbuzz-gpu`) used by the display library to render format text.
//!
//! The installed HarfBuzz is located through the external prefix
//! (`EXT_PATH_PREFIX`, default `C:\Libraries`) by `build.rs`, which links
//! `harfbuzz-gpu` and `harfbuzz`. This crate exposes only what the display
//! library needs: shaping and glyph encoding for GPU rendering. The upstream
//! `harfbuzz-sys` bindings do not cover the `hb-gpu` API, so the declarations
//! here are written directly against `hb.h` and `hb-gpu.h`.

#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]

use std::ffi::{c_char, c_int, c_uint, c_void, CStr};

/// `hb_bool_t`.
pub type hb_bool_t = c_int;
/// `hb_codepoint_t`.
pub type hb_codepoint_t = u32;
/// `hb_position_t`.
pub type hb_position_t = c_int;

/// `hb_destroy_func_t`.
type hb_destroy_func_t = Option<unsafe extern "C" fn(*mut c_void)>;

/// `hb_memory_mode_t::HB_MEMORY_MODE_DUPLICATE`.
const HB_MEMORY_MODE_DUPLICATE: c_int = 0;

/// `hb_gpu_shader_lang_t`.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderLang {
    Invalid = 0,
    /// GLSL (OpenGL 3.3 / OpenGL ES 3.0 / WebGL 2.0).
    Glsl = 1,
    /// WGSL (WebGPU).
    Wgsl = 2,
    /// MSL (Metal).
    Msl = 3,
    /// HLSL (Direct3D).
    Hlsl = 4,
}

/// `hb_gpu_shader_stage_t`.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaderStage {
    Vertex = 0,
    Fragment = 1,
}

#[repr(C)]
pub struct hb_blob_t {
    _private: [u8; 0],
}
#[repr(C)]
pub struct hb_face_t {
    _private: [u8; 0],
}
#[repr(C)]
pub struct hb_font_t {
    _private: [u8; 0],
}
#[repr(C)]
pub struct hb_buffer_t {
    _private: [u8; 0],
}
#[repr(C)]
pub struct hb_gpu_draw_t {
    _private: [u8; 0],
}

/// `hb_glyph_info_t`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct hb_glyph_info_t {
    pub codepoint: hb_codepoint_t,
    pub mask: u32,
    pub cluster: u32,
    pub var1: u32,
    pub var2: u32,
}

/// `hb_glyph_position_t`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct hb_glyph_position_t {
    pub x_advance: hb_position_t,
    pub y_advance: hb_position_t,
    pub x_offset: hb_position_t,
    pub y_offset: hb_position_t,
    pub var: u32,
}

/// `hb_glyph_extents_t`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct hb_glyph_extents_t {
    pub x_bearing: hb_position_t,
    pub y_bearing: hb_position_t,
    pub width: hb_position_t,
    pub height: hb_position_t,
}

extern "C" {
    fn hb_version_string() -> *const c_char;

    fn hb_blob_create(
        data: *const c_char,
        length: c_uint,
        mode: c_int,
        user_data: *mut c_void,
        destroy: hb_destroy_func_t,
    ) -> *mut hb_blob_t;
    fn hb_blob_destroy(blob: *mut hb_blob_t);
    fn hb_blob_get_data(blob: *mut hb_blob_t, length: *mut c_uint) -> *const c_char;

    fn hb_face_create(blob: *mut hb_blob_t, index: c_uint) -> *mut hb_face_t;
    fn hb_face_destroy(face: *mut hb_face_t);
    fn hb_face_get_upem(face: *mut hb_face_t) -> c_uint;

    fn hb_font_create(face: *mut hb_face_t) -> *mut hb_font_t;
    fn hb_font_destroy(font: *mut hb_font_t);
    fn hb_ot_font_set_funcs(font: *mut hb_font_t);
    fn hb_font_set_scale(font: *mut hb_font_t, x_scale: c_int, y_scale: c_int);
    fn hb_font_get_scale(font: *mut hb_font_t, x_scale: *mut c_int, y_scale: *mut c_int);

    fn hb_buffer_create() -> *mut hb_buffer_t;
    fn hb_buffer_destroy(buffer: *mut hb_buffer_t);
    fn hb_buffer_add_utf8(
        buffer: *mut hb_buffer_t,
        text: *const c_char,
        text_length: c_int,
        item_offset: c_uint,
        item_length: c_int,
    );
    fn hb_buffer_guess_segment_properties(buffer: *mut hb_buffer_t);
    fn hb_buffer_get_length(buffer: *mut hb_buffer_t) -> c_uint;
    fn hb_buffer_get_glyph_infos(
        buffer: *mut hb_buffer_t,
        length: *mut c_uint,
    ) -> *mut hb_glyph_info_t;
    fn hb_buffer_get_glyph_positions(
        buffer: *mut hb_buffer_t,
        length: *mut c_uint,
    ) -> *mut hb_glyph_position_t;

    fn hb_shape(
        font: *mut hb_font_t,
        buffer: *mut hb_buffer_t,
        features: *const c_void,
        num_features: c_uint,
    );

    fn hb_gpu_shader_source(stage: c_int, lang: c_int) -> *const c_char;
    fn hb_gpu_draw_shader_source(stage: c_int, lang: c_int) -> *const c_char;
    fn hb_gpu_draw_create_or_fail() -> *mut hb_gpu_draw_t;
    fn hb_gpu_draw_destroy(draw: *mut hb_gpu_draw_t);
    fn hb_gpu_draw_glyph(draw: *mut hb_gpu_draw_t, font: *mut hb_font_t, glyph: hb_codepoint_t);
    fn hb_gpu_draw_encode(
        draw: *mut hb_gpu_draw_t,
        extents: *mut hb_glyph_extents_t,
    ) -> *mut hb_blob_t;
    fn hb_gpu_draw_recycle_blob(draw: *mut hb_gpu_draw_t, blob: *mut hb_blob_t);
    fn hb_gpu_draw_get_scale(draw: *const hb_gpu_draw_t, x_scale: *mut c_int, y_scale: *mut c_int);
}

/// The HarfBuzz version string (for diagnostics).
pub fn version_string() -> String {
    unsafe { CStr::from_ptr(hb_version_string()) }
        .to_string_lossy()
        .into_owned()
}

fn cstr(ptr: *const c_char) -> Option<&'static str> {
    if ptr.is_null() {
        return None;
    }
    // HarfBuzz documents the shader sources as static strings that must not be
    // freed, so borrowing them for 'static is sound.
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

/// The shared helper shader source for `stage`.
pub fn shader_source(stage: ShaderStage, lang: ShaderLang) -> Option<&'static str> {
    cstr(unsafe { hb_gpu_shader_source(stage as c_int, lang as c_int) })
}

/// The draw-renderer shader source for `stage` (empty for the vertex stage).
pub fn draw_shader_source(stage: ShaderStage, lang: ShaderLang) -> Option<&'static str> {
    cstr(unsafe { hb_gpu_draw_shader_source(stage as c_int, lang as c_int) })
}

/// An `hb_blob_t`.
pub struct Blob(*mut hb_blob_t);

impl Blob {
    /// Create a blob that owns a copy of `bytes`.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let raw = unsafe {
            hb_blob_create(
                bytes.as_ptr() as *const c_char,
                bytes.len() as c_uint,
                HB_MEMORY_MODE_DUPLICATE,
                std::ptr::null_mut(),
                None,
            )
        };
        Blob(raw)
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        unsafe { hb_blob_destroy(self.0) }
    }
}

/// An `hb_face_t`.
pub struct Face(*mut hb_face_t);

impl Face {
    /// Create a face from the first font in `blob`.
    pub fn new(blob: &Blob) -> Self {
        Face(unsafe { hb_face_create(blob.0, 0) })
    }

    /// Units per em.
    pub fn upem(&self) -> u32 {
        unsafe { hb_face_get_upem(self.0) }
    }
}

impl Drop for Face {
    fn drop(&mut self) {
        unsafe { hb_face_destroy(self.0) }
    }
}

/// An `hb_font_t`.
pub struct Font(*mut hb_font_t);

impl Font {
    /// Create a font from `face`, using the OpenType font functions.
    pub fn new(face: &Face) -> Self {
        let raw = unsafe { hb_font_create(face.0) };
        unsafe { hb_ot_font_set_funcs(raw) };
        Font(raw)
    }

    /// Set the font scale (typically the face's units per em on both axes).
    pub fn set_scale(&mut self, x_scale: i32, y_scale: i32) {
        unsafe { hb_font_set_scale(self.0, x_scale, y_scale) }
    }

    /// The current font scale.
    pub fn scale(&self) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        unsafe { hb_font_get_scale(self.0, &mut x, &mut y) };
        (x, y)
    }

    fn as_raw(&self) -> *mut hb_font_t {
        self.0
    }
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe { hb_font_destroy(self.0) }
    }
}

/// An `hb_buffer_t`.
pub struct Buffer(*mut hb_buffer_t);

impl Default for Buffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer {
    /// Create an empty buffer.
    pub fn new() -> Self {
        Buffer(unsafe { hb_buffer_create() })
    }

    /// Append UTF-8 `text`.
    pub fn add_str(&mut self, text: &str) {
        unsafe {
            hb_buffer_add_utf8(
                self.0,
                text.as_ptr() as *const c_char,
                text.len() as c_int,
                0,
                text.len() as c_int,
            );
        }
    }

    /// Guess the segment properties (direction, script, language) from the text.
    pub fn guess_segment_properties(&mut self) {
        unsafe { hb_buffer_guess_segment_properties(self.0) }
    }

    /// Shape the buffer with `font`.
    pub fn shape(&mut self, font: &Font) {
        unsafe { hb_shape(font.as_raw(), self.0, std::ptr::null(), 0) }
    }

    /// The number of glyphs after shaping.
    pub fn len(&self) -> u32 {
        unsafe { hb_buffer_get_length(self.0) }
    }

    /// Whether the buffer holds no glyphs.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The shaped glyph infos.
    pub fn glyph_infos(&self) -> Vec<hb_glyph_info_t> {
        let mut n = 0u32;
        let ptr = unsafe { hb_buffer_get_glyph_infos(self.0, &mut n) };
        if ptr.is_null() || n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, n as usize).to_vec() }
    }

    /// The shaped glyph positions.
    pub fn glyph_positions(&self) -> Vec<hb_glyph_position_t> {
        let mut n = 0u32;
        let ptr = unsafe { hb_buffer_get_glyph_positions(self.0, &mut n) };
        if ptr.is_null() || n == 0 {
            return Vec::new();
        }
        unsafe { std::slice::from_raw_parts(ptr, n as usize).to_vec() }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { hb_buffer_destroy(self.0) }
    }
}

/// An `hb_gpu_draw_t` encoder.
pub struct GpuDraw(*mut hb_gpu_draw_t);

impl GpuDraw {
    /// Create an encoder, or `None` on allocation failure.
    pub fn new() -> Option<Self> {
        let raw = unsafe { hb_gpu_draw_create_or_fail() };
        if raw.is_null() {
            None
        } else {
            Some(GpuDraw(raw))
        }
    }

    /// Accumulate the outline of `glyph`.
    pub fn glyph(&mut self, font: &Font, glyph: u32) {
        unsafe { hb_gpu_draw_glyph(self.0, font.as_raw(), glyph) }
    }

    /// Encode the accumulated outline into GPU blob bytes and its extents.
    ///
    /// The bytes are an array of `RGBA16I` texels (8 bytes each) for a texture
    /// buffer. An empty vector means the glyph has no ink; the extents are then
    /// zero.
    pub fn encode(&mut self) -> (Vec<u8>, hb_glyph_extents_t) {
        let mut extents = hb_glyph_extents_t::default();
        let blob = unsafe { hb_gpu_draw_encode(self.0, &mut extents) };
        if blob.is_null() {
            return (Vec::new(), extents);
        }

        let mut len = 0u32;
        let data = unsafe { hb_blob_get_data(blob, &mut len) };
        let bytes = if data.is_null() || len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(data as *const u8, len as usize).to_vec() }
        };
        unsafe { hb_gpu_draw_recycle_blob(self.0, blob) };
        (bytes, extents)
    }

    /// The encoder's font scale.
    pub fn scale(&self) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        unsafe { hb_gpu_draw_get_scale(self.0, &mut x, &mut y) };
        (x, y)
    }
}

impl Drop for GpuDraw {
    fn drop(&mut self) {
        unsafe { hb_gpu_draw_destroy(self.0) }
    }
}
