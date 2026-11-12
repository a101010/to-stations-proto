//! Formats. Each format is a submodule.
//!
//! A format renders itself into a region (physical pixels, top-left origin)
//! and supplies the title and icon the chooser displays. `formats::registry`
//! builds the formats the format manager can switch between; the manager
//! selects the active one and forwards input to it.

pub(crate) mod default;
pub(crate) mod format_chooser;
pub(crate) mod hello_world;

use std::path::Path;

use format_list::FormatList;

use crate::DisplayError;

/// The registry name of the format chooser.
pub(crate) const FORMAT_CHOOSER: &str = "format-chooser";

/// The catalog data the chooser needs for another format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FormatMeta {
    pub(crate) name: String,
    pub(crate) title: &'static str,
    pub(crate) icon_file: &'static str,
}

/// What active-format input did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FormatInput {
    Ignored,
    Consumed,
    Select(String),
}

/// A displayable format.
pub(crate) trait Format {
    /// The format's registry name.
    fn name(&self) -> &'static str;

    /// The title shown in the format chooser.
    fn title(&self) -> &'static str;

    /// The PNG icon, relative to the deploy assets root.
    fn icon_file(&self) -> &'static str;

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

    /// Handle a left-button press inside the format region.
    fn mouse_button(&mut self, region: (i32, i32, i32, i32), x: i32, y: i32) -> FormatInput {
        let _ = (region, x, y);
        FormatInput::Ignored
    }
}

/// Build the format registry in contract order. The first format is the
/// default for unassigned format windows. The chooser is built last from the
/// catalog of every other format.
pub(crate) fn registry(
    gl: &glow::Context,
    assets_root: &Path,
    state: crate::view::ViewState,
    formats: &FormatList,
) -> Result<Vec<Box<dyn Format>>, DisplayError> {
    let names = formats.names();
    let mut slots: Vec<Option<Box<dyn Format>>> = Vec::with_capacity(names.len());
    let mut catalog = Vec::new();

    for name in names {
        match name.as_str() {
            FORMAT_CHOOSER => slots.push(None),
            "hello-world" => {
                let format = hello_world::HelloWorld::new(gl, assets_root, state.clone())?;
                catalog.push(meta(name, &format));
                slots.push(Some(Box::new(format)));
            }
            "default" => {
                let format = default::Default::new(gl)?;
                catalog.push(meta(name, &format));
                slots.push(Some(Box::new(format)));
            }
            other => {
                return Err(DisplayError::Formats(format!(
                    "unknown format in formats.toml: {other}"
                )))
            }
        }
    }

    for (index, name) in names.iter().enumerate() {
        if name == FORMAT_CHOOSER {
            let chooser = format_chooser::FormatChooser::new(gl, assets_root, &catalog)?;
            slots[index] = Some(Box::new(chooser));
        }
    }

    slots
        .into_iter()
        .map(|slot| {
            slot.ok_or_else(|| DisplayError::Formats("missing format renderer".to_string()))
        })
        .collect()
}

fn meta<F: Format + ?Sized>(name: &str, format: &F) -> FormatMeta {
    FormatMeta {
        name: name.to_string(),
        title: format.title(),
        icon_file: format.icon_file(),
    }
}
