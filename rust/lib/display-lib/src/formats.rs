//! Formats. Each format is a submodule.
//!
//! A format renders itself into a region (physical pixels, top-left origin).
//! `formats::registry` builds the formats the format manager can switch
//! between; the manager selects the active one and forwards input to it.

pub(crate) mod default;
pub(crate) mod hello_world;

use std::path::Path;

use format_list::FormatList;

use crate::DisplayError;

/// A displayable format.
pub(crate) trait Format {
    /// The format's registry name.
    fn name(&self) -> &'static str;

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

/// Build the format registry in contract order. The first format is the
/// default for unassigned format windows.
pub(crate) fn registry(
    gl: &glow::Context,
    assets_root: &Path,
    state: crate::view::ViewState,
    formats: &FormatList,
) -> Result<Vec<Box<dyn Format>>, DisplayError> {
    let mut result = Vec::with_capacity(formats.names().len());
    for name in formats.names() {
        match name.as_str() {
            "hello-world" => result.push(Box::new(hello_world::HelloWorld::new(
                gl,
                assets_root,
                state.clone(),
            )?) as Box<dyn Format>),
            "default" => result.push(Box::new(default::Default::new(gl)?) as Box<dyn Format>),
            other => {
                return Err(DisplayError::Formats(format!(
                    "unknown format in formats.toml: {other}"
                )))
            }
        }
    }
    Ok(result)
}
