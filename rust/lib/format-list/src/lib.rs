//! The ordered format list from `contracts/formats.toml`.
//!
//! The format-manager controller uses the list to compute next/select; the
//! display library uses the same order to build its format renderers.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// The implicit format chooser, never listed in `formats.toml`.
pub const FORMAT_CHOOSER: &str = "format-chooser";

/// `FormatManagerInputTopic.command`: advance to the next format.
pub const FORMAT_COMMAND_NEXT: i32 = 0;
/// `FormatManagerInputTopic.command`: select the named format.
pub const FORMAT_COMMAND_SELECT: i32 = 1;
/// `FormatManagerInputTopic.command`: move the virtual cursor.
pub const FORMAT_COMMAND_CURSOR_MOVE: i32 = 10;
/// `FormatManagerInputTopic.command`: move the active selection.
pub const FORMAT_COMMAND_NAVIGATE: i32 = 11;
/// `FormatManagerInputTopic.command`: pointer button event.
pub const FORMAT_COMMAND_POINTER: i32 = 12;
/// `FormatManagerInputTopic.command`: toggle the hello-world arcball.
pub const FORMAT_COMMAND_TOGGLE_ARCBALL: i32 = 13;
/// `FormatManagerInputTopic.command`: toggle the three-dot menu.
pub const FORMAT_COMMAND_MENU_TOGGLE: i32 = 14;
/// `FormatManagerInputTopic.command`: cancel/close the three-dot menu.
pub const FORMAT_COMMAND_MENU_CANCEL: i32 = 15;
/// `FormatManagerInputTopic.command`: cancel the active overlay.
pub const FORMAT_COMMAND_OVERLAY_CANCEL: i32 = 16;
/// `FormatManagerInputTopic.button`: right mouse button.
pub const FORMAT_POINTER_RIGHT: i32 = 1;

/// Whether `command` is a high-level control command relayed by the controller.
pub fn is_control_command(command: i32) -> bool {
    matches!(
        command,
        FORMAT_COMMAND_CURSOR_MOVE
            | FORMAT_COMMAND_NAVIGATE
            | FORMAT_COMMAND_POINTER
            | FORMAT_COMMAND_TOGGLE_ARCBALL
            | FORMAT_COMMAND_MENU_TOGGLE
            | FORMAT_COMMAND_MENU_CANCEL
            | FORMAT_COMMAND_OVERLAY_CANCEL
    )
}

/// `HelloWorldViewTopic.mode`: automatic model attitude.
pub const HELLO_MODE_AUTO: i32 = 0;
/// `HelloWorldViewTopic.mode`: local, arcball-controlled attitude.
pub const HELLO_MODE_INSPECT: i32 = 1;

/// `HelloWorldInputTopic.action`: begin an arcball drag.
pub const HELLO_ACTION_GRAB: i32 = 0;
/// `HelloWorldInputTopic.action`: continue an arcball drag.
pub const HELLO_ACTION_DRAG: i32 = 1;
/// `HelloWorldInputTopic.action`: end an arcball drag, applying inertia.
pub const HELLO_ACTION_RELEASE: i32 = 2;
/// `HelloWorldInputTopic.action`: toggle right-stick arcball control.
pub const HELLO_ACTION_TOGGLE_ARCBALL: i32 = 3;
/// `HelloWorldInputTopic.action`: toggle Auto/Inspect.
pub const HELLO_ACTION_MODE_TOGGLE: i32 = 4;
/// `HelloWorldInputTopic.action`: rotate from a right-stick movement.
pub const HELLO_ACTION_ROTATE: i32 = 5;
/// `HelloWorldInputTopic.action`: toggle the Lux Color placeholder overlay.
pub const HELLO_ACTION_TOGGLE_LUX_COLOR: i32 = 6;
/// `HelloWorldInputTopic.action`: toggle the Lux Pos placeholder overlay.
pub const HELLO_ACTION_TOGGLE_LUX_POS: i32 = 7;
/// `HelloWorldInputTopic.action`: set the color overlay's normalized hue.
pub const HELLO_ACTION_COLOR_HUE: i32 = 8;
/// `HelloWorldInputTopic.action`: set the color overlay's saturation/value.
pub const HELLO_ACTION_COLOR_SV: i32 = 9;
/// `HelloWorldInputTopic.action`: commit the color overlay.
pub const HELLO_ACTION_COLOR_COMMIT: i32 = 10;
/// `HelloWorldInputTopic.action`: cancel and restore the color overlay.
pub const HELLO_ACTION_COLOR_CANCEL: i32 = 11;
/// `HelloWorldInputTopic.action`: set the light overlay's front-hemisphere direction.
pub const HELLO_ACTION_LIGHT: i32 = 12;
/// `HelloWorldInputTopic.action`: commit the light overlay.
pub const HELLO_ACTION_LIGHT_COMMIT: i32 = 13;
/// `HelloWorldInputTopic.action`: cancel and restore the light overlay.
pub const HELLO_ACTION_LIGHT_CANCEL: i32 = 14;

/// The hello-world menu item that toggles Auto/Inspect.
pub const HELLO_MENU_MODE: &str = "mode";
/// The hello-world menu item for the Lux Color placeholder overlay.
pub const HELLO_MENU_LUX_COLOR: &str = "lux-color";
/// The hello-world menu item for the Lux Pos placeholder overlay.
pub const HELLO_MENU_LUX_POS: &str = "lux-pos";
/// `HelloWorldViewTopic.overlay`: no overlay is open.
pub const HELLO_OVERLAY_NONE: i32 = 0;
/// `HelloWorldViewTopic.overlay`: the Lux Color overlay is open.
pub const HELLO_OVERLAY_COLOR: i32 = 1;
/// `HelloWorldViewTopic.overlay`: the Lux Pos overlay is open.
pub const HELLO_OVERLAY_POS: i32 = 2;

/// One saved format-window assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedAssignment {
    pub window: i32,
    pub format: String,
}

/// The persisted per-window assignments for one display.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedAssignments {
    #[serde(default)]
    pub windows: Vec<SavedAssignment>,
}

impl SavedAssignments {
    /// The effective default: window 0 is hello-world; other windows remain
    /// unassigned and fall back to hello-world in the display library.
    pub fn hello_world() -> Self {
        Self {
            windows: vec![SavedAssignment {
                window: 0,
                format: "hello-world".to_string(),
            }],
        }
    }

    /// Drop duplicate windows, sort by window, and replace unknown formats
    /// with hello-world.
    pub fn validated(self, formats: &FormatList) -> Self {
        let mut windows = Vec::new();
        let mut seen = HashSet::new();
        for mut assignment in self.windows {
            if !seen.insert(assignment.window) {
                continue;
            }
            if !formats.is_known(&assignment.format) {
                assignment.format = "hello-world".to_string();
            }
            windows.push(assignment);
        }
        windows.sort_by_key(|assignment| assignment.window);
        Self { windows }
    }
}

#[derive(Debug)]
pub enum FormatListError {
    Io(String),
    Toml(String),
    Empty,
    Duplicate(String),
    Reserved(String),
}

impl std::fmt::Display for FormatListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatListError::Io(message) => write!(f, "io error: {message}"),
            FormatListError::Toml(message) => write!(f, "toml error: {message}"),
            FormatListError::Empty => write!(f, "format list is empty"),
            FormatListError::Duplicate(name) => write!(f, "duplicate format: {name}"),
            FormatListError::Reserved(name) => {
                write!(f, "reserved implicit format: {name}")
            }
        }
    }
}

impl std::error::Error for FormatListError {}

#[derive(Debug, Deserialize)]
struct FormatsFile {
    #[serde(default)]
    formats: Vec<FormatDef>,
}

#[derive(Debug, Deserialize)]
struct FormatDef {
    name: String,
}

/// An ordered, validated format list.
#[derive(Debug, Clone, Default)]
pub struct FormatList {
    names: Vec<String>,
}

impl FormatList {
    /// Parse a format list from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, FormatListError> {
        let file: FormatsFile =
            toml::from_str(text).map_err(|e| FormatListError::Toml(e.to_string()))?;
        let mut names = Vec::with_capacity(file.formats.len());
        let mut seen = HashSet::new();
        for format in file.formats {
            if format.name.is_empty() {
                return Err(FormatListError::Empty);
            }
            if format.name == FORMAT_CHOOSER {
                return Err(FormatListError::Reserved(format.name));
            }
            if !seen.insert(format.name.clone()) {
                return Err(FormatListError::Duplicate(format.name));
            }
            names.push(format.name);
        }
        if names.is_empty() {
            return Err(FormatListError::Empty);
        }
        Ok(Self { names })
    }

    /// Load a format list from `path`.
    pub fn load(path: &Path) -> Result<Self, FormatListError> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| FormatListError::Io(format!("read {}: {e}", path.display())))?;
        Self::from_toml(&text)
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn first(&self) -> Option<&str> {
        self.names.first().map(String::as_str)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.iter().any(|candidate| candidate == name)
    }

    /// Whether `name` can be assigned as an active format. The chooser is
    /// implicit and is known even though it is not in the catalog.
    pub fn is_known(&self, name: &str) -> bool {
        name == FORMAT_CHOOSER || self.contains(name)
    }

    /// The next format name, wrapping. Returns `None` for an unknown format.
    pub fn next(&self, name: &str) -> Option<&str> {
        let index = self.names.iter().position(|candidate| candidate == name)?;
        Some(&self.names[(index + 1) % self.names.len()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMATS: &str = r#"
[[formats]]
name = "hello-world"

[[formats]]
name = "default"
"#;

    #[test]
    fn loads_and_wraps() {
        let list = FormatList::from_toml(FORMATS).unwrap();
        assert_eq!(list.names(), ["hello-world", "default"]);
        assert_eq!(list.first(), Some("hello-world"));
        assert!(list.contains("default"));
        assert!(!list.contains(FORMAT_CHOOSER));
        assert!(list.is_known(FORMAT_CHOOSER));
        assert_eq!(list.next("hello-world"), Some("default"));
        assert_eq!(list.next("default"), Some("hello-world"));
        assert_eq!(list.next(FORMAT_CHOOSER), None);
        assert_eq!(list.next("missing"), None);
    }

    #[test]
    fn rejects_explicit_chooser() {
        let text = r#"
[[formats]]
name = "hello-world"

[[formats]]
name = "format-chooser"
"#;
        assert!(matches!(
            FormatList::from_toml(text),
            Err(FormatListError::Reserved(name)) if name == FORMAT_CHOOSER
        ));
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            FormatList::from_toml(""),
            Err(FormatListError::Empty)
        ));
        assert!(matches!(
            FormatList::from_toml("[[formats]]\nname = \"\""),
            Err(FormatListError::Empty)
        ));
    }

    #[test]
    fn rejects_duplicates() {
        let text = r#"
[[formats]]
name = "a"
[[formats]]
name = "a"
"#;
        assert!(matches!(
            FormatList::from_toml(text),
            Err(FormatListError::Duplicate(name)) if name == "a"
        ));
    }

    #[test]
    fn saved_assignments_default_and_validate() {
        let list = FormatList::from_toml(FORMATS).unwrap();
        assert_eq!(
            SavedAssignments::hello_world(),
            SavedAssignments {
                windows: vec![SavedAssignment {
                    window: 0,
                    format: "hello-world".to_string(),
                }],
            }
        );

        let saved = SavedAssignments {
            windows: vec![
                SavedAssignment {
                    window: 1,
                    format: "missing".to_string(),
                },
                SavedAssignment {
                    window: 0,
                    format: "default".to_string(),
                },
                SavedAssignment {
                    window: 0,
                    format: "hello-world".to_string(),
                },
                SavedAssignment {
                    window: 2,
                    format: FORMAT_CHOOSER.to_string(),
                },
            ],
        };
        assert_eq!(
            saved.validated(&list),
            SavedAssignments {
                windows: vec![
                    SavedAssignment {
                        window: 0,
                        format: "default".to_string(),
                    },
                    SavedAssignment {
                        window: 1,
                        format: "hello-world".to_string(),
                    },
                    SavedAssignment {
                        window: 2,
                        format: FORMAT_CHOOSER.to_string(),
                    },
                ],
            }
        );
    }
    #[test]
    fn control_commands_are_classified() {
        assert!(!is_control_command(FORMAT_COMMAND_NEXT));
        assert!(!is_control_command(FORMAT_COMMAND_SELECT));
        assert!(is_control_command(FORMAT_COMMAND_CURSOR_MOVE));
        assert!(is_control_command(FORMAT_COMMAND_NAVIGATE));
        assert!(is_control_command(FORMAT_COMMAND_POINTER));
        assert!(is_control_command(FORMAT_COMMAND_TOGGLE_ARCBALL));
        assert!(is_control_command(FORMAT_COMMAND_MENU_TOGGLE));
        assert!(is_control_command(FORMAT_COMMAND_MENU_CANCEL));
        assert!(is_control_command(FORMAT_COMMAND_OVERLAY_CANCEL));
        assert!(!is_control_command(99));
    }
}
