//! Exercises the binding end to end: load the bundled FiraSans, shape the
//! greeting, and encode a glyph for GPU rendering. No GL context is needed.

use std::path::PathBuf;

use harfbuzz::{Blob, Buffer, Face, Font, GpuDraw, ShaderLang, ShaderStage};

fn font_bytes() -> Vec<u8> {
    let root = std::env::var("REPO_ROOT").expect("REPO_ROOT is set by .cargo/config.toml");
    let path = PathBuf::from(root)
        .join("assets")
        .join("fonts")
        .join("FiraSans-Regular.ttf");
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn shapes_and_encodes() {
    let bytes = font_bytes();
    let blob = Blob::from_bytes(&bytes);
    let face = Face::new(&blob);
    assert!(face.upem() > 0, "face has no units per em");

    let mut font = Font::new(&face);
    let upem = face.upem() as i32;
    font.set_scale(upem, upem);
    assert_eq!(font.scale(), (upem, upem));

    let mut buffer = Buffer::new();
    buffer.add_str("Hello, world!");
    buffer.guess_segment_properties();
    buffer.shape(&font);

    let infos = buffer.glyph_infos();
    let positions = buffer.glyph_positions();
    assert_eq!(infos.len(), buffer.len() as usize);
    assert_eq!(positions.len(), infos.len());
    assert!(!buffer.is_empty(), "shaped no glyphs");
    assert!(
        positions.iter().any(|p| p.x_advance != 0),
        "no glyph advanced"
    );

    let mut draw = GpuDraw::new().expect("create gpu encoder");
    draw.glyph(&font, infos[0].codepoint);
    let (blob_bytes, extents) = draw.encode();
    assert!(!blob_bytes.is_empty(), "encoded blob is empty");
    assert_eq!(blob_bytes.len() % 8, 0, "blob is not a whole number of texels");
    assert!(extents.width > 0 || extents.height > 0, "no ink extents");
}

#[test]
fn shader_sources_are_present() {
    let vertex = harfbuzz::shader_source(ShaderStage::Vertex, ShaderLang::Glsl).unwrap_or("");
    let fragment = harfbuzz::shader_source(ShaderStage::Fragment, ShaderLang::Glsl).unwrap_or("");
    let draw_fragment =
        harfbuzz::draw_shader_source(ShaderStage::Fragment, ShaderLang::Glsl).unwrap_or("");
    assert!(!vertex.is_empty(), "empty shared vertex shader");
    assert!(!fragment.is_empty(), "empty shared fragment shader");
    assert!(!draw_fragment.is_empty(), "empty draw fragment shader");
}
