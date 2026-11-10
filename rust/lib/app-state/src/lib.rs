//! Per-user runtime state files.
//!
//! State lives under `%LOCALAPPDATA%\to-stations\<application>\<key>.toml`.
//! Each instance discriminator gets its own file, and writes are atomic so a
//! reader never observes a partial update.

use std::io;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

/// The state file for `application`/`key`, or `None` when `LOCALAPPDATA` is
/// unavailable.
pub fn state_path(application: &str, key: &str) -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(base)
            .join("to-stations")
            .join(application)
            .join(format!("{key}.toml")),
    )
}

/// Load a TOML state file. Missing or malformed files return `None`.
pub fn load<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

/// Write a TOML state file atomically, creating its directory if needed.
pub fn save<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string(value).map_err(io::Error::other)?;
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Sample {
        value: u32,
    }

    #[test]
    fn saves_and_loads_round_trip() {
        let dir = std::env::temp_dir().join(format!(
            "to-stations-app-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("state.toml");
        let sample = Sample { value: 42 };
        save(&path, &sample).unwrap();
        assert_eq!(load::<Sample>(&path), Some(sample));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_file_is_none() {
        let path = std::env::temp_dir().join("to-stations-app-state-missing.toml");
        let _ = std::fs::remove_file(&path);
        assert!(load::<Sample>(&path).is_none());
    }
}
