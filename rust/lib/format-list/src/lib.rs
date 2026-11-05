//! The ordered format list from `contracts/formats.toml`.
//!
//! The format-manager controller uses the list to compute next/select; the
//! display library uses the same order to build its format renderers.

use std::collections::HashSet;
use std::path::Path;

use serde::Deserialize;

/// `FormatManagerInputTopic.command`: advance to the next format.
pub const FORMAT_COMMAND_NEXT: i32 = 0;
/// `FormatManagerInputTopic.command`: select the named format.
pub const FORMAT_COMMAND_SELECT: i32 = 1;

#[derive(Debug)]
pub enum FormatListError {
    Io(String),
    Toml(String),
    Empty,
    Duplicate(String),
}

impl std::fmt::Display for FormatListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatListError::Io(message) => write!(f, "io error: {message}"),
            FormatListError::Toml(message) => write!(f, "toml error: {message}"),
            FormatListError::Empty => write!(f, "format list is empty"),
            FormatListError::Duplicate(name) => write!(f, "duplicate format: {name}"),
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
        assert_eq!(list.next("hello-world"), Some("default"));
        assert_eq!(list.next("default"), Some("hello-world"));
        assert_eq!(list.next("missing"), None);
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
}
