//! Formats. Each format is a submodule.
//!
//! A format renders itself into a region (physical pixels, top-left origin).
//! `formats::registry` builds the formats the format manager can switch
//! between; the manager selects the active one and forwards input to it.

pub(crate) mod default;
pub(crate) mod hello_world;

use std::path::Path;

use crate::DisplayError;

/// A displayable format.
pub(crate) trait Format {
    /// Render into the region (physical pixels, top-left origin).
    /// `framebuffer_height` is the whole framebuffer height.
    fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    );
}

/// Build the format registry, in display order. The first format is the
/// initial active format.
pub(crate) fn registry(
    gl: &glow::Context,
    assets_root: &Path,
    state: crate::view::ViewState,
) -> Result<Vec<Box<dyn Format>>, DisplayError> {
    Ok(vec![
        Box::new(hello_world::HelloWorld::new(gl, assets_root, state)?),
        Box::new(default::Default::new(gl)?),
    ])
}
