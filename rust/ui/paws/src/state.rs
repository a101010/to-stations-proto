//! Per-display paws state.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The persisted state for one display id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedState {
    #[serde(default)]
    pub(crate) controller: Option<SavedController>,
    #[serde(default)]
    pub(crate) position: Option<window_position::SavedPosition>,
}

/// The stable identity of a remembered controller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedController {
    pub(crate) name: String,
    pub(crate) uuid: [u8; 16],
    pub(crate) id: usize,
}

impl SavedState {
    /// Load the state for `display_id`, or return defaults when it is missing.
    pub(crate) fn load(display_id: u32) -> (Option<PathBuf>, Self) {
        let path = app_state::state_path("paws", &display_id.to_string());
        let state = path
            .as_deref()
            .and_then(app_state::load::<Self>)
            .unwrap_or_default();
        (path, state)
    }

    /// Save the state atomically. Failures are reported but not fatal.
    pub(crate) fn save(&self, path: Option<&Path>) {
        let Some(path) = path else {
            return;
        };
        if let Err(error) = app_state::save(path, self) {
            eprintln!("paws: state save failed: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_saved_controller() {
        let dir = std::env::temp_dir().join(format!(
            "to-stations-paws-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("0.toml");
        let state = SavedState {
            controller: Some(SavedController {
                name: "Xbox Wireless Controller".to_string(),
                uuid: [7; 16],
                id: 2,
            }),
            position: Some(window_position::SavedPosition {
                monitor: Some("\\\\.\\DISPLAY1".to_string()),
                monitor_width: Some(1920),
                monitor_height: Some(1080),
                x: 100,
                y: 50,
            }),
        };
        state.save(Some(&path));
        assert_eq!(app_state::load::<SavedState>(&path), Some(state));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
