//! Per-display window position persistence and default cascade placement.

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;

/// Minimum time between position writes while the window moves.
pub const SAVE_INTERVAL: Duration = Duration::from_millis(250);

/// Minimum visible width and height on the target monitor.
const MIN_VISIBLE: i32 = 64;

/// How far a window's top-left may sit outside the target monitor and still be
/// treated as a normal edge placement.
const FRAME_MARGIN: i32 = 64;

/// Whether a position write should happen now. Forced saves and monitor changes
/// bypass the normal throttle.
pub fn should_save(force: bool, monitor_changed: bool, since_last: Duration) -> bool {
    force || monitor_changed || since_last >= SAVE_INTERVAL
}

/// Saved position relative to a monitor. `monitor_width` and `monitor_height`
/// are absent in legacy files and are filled in on the next save.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPosition {
    #[serde(default)]
    pub monitor: Option<String>,
    #[serde(default)]
    pub monitor_width: Option<u32>,
    #[serde(default)]
    pub monitor_height: Option<u32>,
    pub x: i32,
    pub y: i32,
}

/// The resolved startup position and the state that should be persisted for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPosition {
    pub position: PhysicalPosition<i32>,
    pub saved: Option<SavedPosition>,
}

#[derive(Clone, Debug)]
struct MonitorInfo {
    name: Option<String>,
    position: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
}

/// The state file for an application and display id.
pub fn state_path(application: &str, display_id: u32) -> Option<PathBuf> {
    app_state::state_path(application, &display_id.to_string())
}

/// Load the saved position for an application and display id.
pub fn load(application: &str, display_id: u32) -> Option<SavedPosition> {
    let path = state_path(application, display_id)?;
    app_state::load(&path)
}

/// Save the position for an application and display id.
pub fn save(application: &str, display_id: u32, position: &SavedPosition) -> io::Result<()> {
    let path = state_path(application, display_id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is unavailable"))?;
    app_state::save(&path, position)
}

/// Build the saved position for the current window and monitor.
pub fn saved_from_window(
    window: &Window,
    position: PhysicalPosition<i32>,
) -> Option<SavedPosition> {
    saved_from_window_at(window, position.x, position.y)
}

/// Build the saved position for the current window and monitor from x/y.
pub fn saved_from_window_at(window: &Window, x: i32, y: i32) -> Option<SavedPosition> {
    let monitor = monitor_info(window.current_monitor()?);
    Some(saved_from_monitor(&monitor, PhysicalPosition::new(x, y)))
}

/// Restore the saved position, fall back to the main monitor, or cascade by
/// display id.
pub fn restore_or_cascade(window: &Window, application: &str, display_id: u32) -> ResolvedPosition {
    let saved = load(application, display_id);
    restore_or_cascade_with(saved.as_ref(), window, display_id)
}

/// Restore `saved`, fall back to the main monitor, or cascade by display id.
pub fn restore_or_cascade_with(
    saved: Option<&SavedPosition>,
    window: &Window,
    display_id: u32,
) -> ResolvedPosition {
    let monitors: Vec<MonitorInfo> = window.available_monitors().map(monitor_info).collect();
    let primary = window
        .primary_monitor()
        .map(monitor_info)
        .or_else(|| monitors.first().cloned());
    choose_position(
        saved,
        &monitors,
        primary.as_ref(),
        window_size(window),
        display_id,
    )
}

fn monitor_info(monitor: winit::monitor::MonitorHandle) -> MonitorInfo {
    MonitorInfo {
        name: monitor.name(),
        position: monitor.position(),
        size: monitor.size(),
    }
}

fn window_size(window: &Window) -> PhysicalSize<u32> {
    let outer = window.outer_size();
    if outer.width > 0 && outer.height > 0 {
        outer
    } else {
        let inner = window.inner_size();
        PhysicalSize::new(inner.width.max(1), inner.height.max(1))
    }
}

fn choose_position(
    saved: Option<&SavedPosition>,
    monitors: &[MonitorInfo],
    primary: Option<&MonitorInfo>,
    window_size: PhysicalSize<u32>,
    display_id: u32,
) -> ResolvedPosition {
    if let Some(saved) = saved {
        if let Some(name) = saved.monitor.as_deref() {
            if let Some(monitor) = monitors
                .iter()
                .find(|monitor| monitor.name.as_deref() == Some(name))
            {
                if resolution_matches(saved, monitor) {
                    let position = offset(monitor, saved.x, saved.y);
                    if visible_enough(monitor, position, window_size) {
                        return resolved(monitor, position);
                    }
                }
            }
        }
        if let Some(primary) = primary {
            let position = offset(primary, saved.x, saved.y);
            if visible_enough(primary, position, window_size) {
                return resolved(primary, position);
            }
        }
    }

    let (origin_x, origin_y, monitor_width) = primary
        .map(|monitor| (monitor.position.x, monitor.position.y, monitor.size.width))
        .unwrap_or((0, 0, 0));
    let (x, y) = cascade_position(
        origin_x,
        origin_y,
        monitor_width,
        window_size.width,
        display_id,
    );
    let mut position = PhysicalPosition::new(x, y);
    if let Some(monitor) = primary {
        if !visible_enough(monitor, position, window_size) {
            position = monitor.position;
        }
        return resolved(monitor, position);
    }
    ResolvedPosition {
        position,
        saved: None,
    }
}

fn resolution_matches(saved: &SavedPosition, monitor: &MonitorInfo) -> bool {
    match (saved.monitor_width, saved.monitor_height) {
        (Some(width), Some(height)) => monitor.size.width == width && monitor.size.height == height,
        // Legacy state: the name is all we have.
        _ => true,
    }
}

fn resolved(monitor: &MonitorInfo, position: PhysicalPosition<i32>) -> ResolvedPosition {
    ResolvedPosition {
        position,
        saved: Some(saved_from_monitor(monitor, position)),
    }
}

fn saved_from_monitor(monitor: &MonitorInfo, position: PhysicalPosition<i32>) -> SavedPosition {
    SavedPosition {
        monitor: monitor.name.clone(),
        monitor_width: Some(monitor.size.width),
        monitor_height: Some(monitor.size.height),
        x: position.x - monitor.position.x,
        y: position.y - monitor.position.y,
    }
}

fn offset(monitor: &MonitorInfo, x: i32, y: i32) -> PhysicalPosition<i32> {
    PhysicalPosition::new(monitor.position.x + x, monitor.position.y + y)
}

/// Whether the window has enough visible area on `monitor` to be recovered, and
/// its top-left is close enough to the monitor to keep the title bar reachable.
fn visible_enough(
    monitor: &MonitorInfo,
    position: PhysicalPosition<i32>,
    window_size: PhysicalSize<u32>,
) -> bool {
    let monitor_left = monitor.position.x;
    let monitor_top = monitor.position.y;
    let monitor_right = monitor_left + monitor.size.width as i32;
    let monitor_bottom = monitor_top + monitor.size.height as i32;

    let left = position.x.max(monitor_left);
    let top = position.y.max(monitor_top);
    let right = (position.x + window_size.width as i32).min(monitor_right);
    let bottom = (position.y + window_size.height as i32).min(monitor_bottom);
    let visible_width = right - left;
    let visible_height = bottom - top;

    let top_left_near = position.x >= monitor_left - FRAME_MARGIN
        && position.x < monitor_right
        && position.y >= monitor_top - FRAME_MARGIN
        && position.y < monitor_bottom;

    visible_width >= MIN_VISIBLE && visible_height >= MIN_VISIBLE && top_left_near
}

/// Cascade slot layout on the main monitor: row `r` starts at `(10r, 5r)`,
/// and windows are one window width apart within a row.
fn cascade_position(
    origin_x: i32,
    origin_y: i32,
    monitor_width: u32,
    window_width: u32,
    slot: u32,
) -> (i32, i32) {
    let width = i64::from(window_width.max(1));
    let monitor_width = i64::from(monitor_width);
    let mut remaining = i64::from(slot);
    let mut row: i64 = 0;
    loop {
        let available = (monitor_width - 10 * row).max(0);
        let capacity = (available / width).max(1);
        if remaining < capacity {
            let column = remaining;
            return (
                (i64::from(origin_x) + 10 * row + column * width) as i32,
                (i64::from(origin_y) + 5 * row) as i32,
            );
        }
        remaining -= capacity;
        row += 1;
        if row > 100_000 {
            return (origin_x, origin_y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(name: &str, x: i32, y: i32, width: u32, height: u32) -> MonitorInfo {
        MonitorInfo {
            name: Some(name.to_string()),
            position: PhysicalPosition::new(x, y),
            size: PhysicalSize::new(width, height),
        }
    }

    fn saved(
        monitor: &str,
        width: Option<u32>,
        height: Option<u32>,
        x: i32,
        y: i32,
    ) -> SavedPosition {
        SavedPosition {
            monitor: Some(monitor.to_string()),
            monitor_width: width,
            monitor_height: height,
            x,
            y,
        }
    }

    const WINDOW: PhysicalSize<u32> = PhysicalSize::new(736, 759);

    #[test]
    fn save_triggers_follow_force_monitor_and_interval() {
        assert!(should_save(true, false, Duration::ZERO));
        assert!(should_save(false, true, Duration::ZERO));
        assert!(!should_save(
            false,
            false,
            SAVE_INTERVAL - Duration::from_millis(1)
        ));
        assert!(should_save(false, false, SAVE_INTERVAL));
    }

    #[test]
    fn round_trips_monitor_resolution_and_offset() {
        let dir = std::env::temp_dir().join(format!(
            "to-stations-position-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("0.toml");
        let saved = saved("\\\\.\\DISPLAY5", Some(3840), Some(2160), 200, 150);
        app_state::save(&path, &saved).unwrap();
        assert_eq!(app_state::load::<SavedPosition>(&path), Some(saved));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn restores_on_the_preferred_monitor() {
        let monitors = vec![
            monitor("A", 0, 0, 1920, 1080),
            monitor("B", 1920, 0, 1920, 1080),
        ];
        let saved = saved("B", Some(1920), Some(1080), 100, 50);
        assert_eq!(
            choose_position(Some(&saved), &monitors, Some(&monitors[0]), WINDOW, 0).position,
            PhysicalPosition::new(2020, 50)
        );
    }

    #[test]
    fn negative_edge_offset_on_matching_monitor_is_restored() {
        let monitors = vec![
            monitor("A", 0, 0, 3840, 2160),
            monitor("B", -2560, 0, 2560, 1440),
        ];
        let saved = saved("B", Some(2560), Some(1440), -5, 2);
        assert_eq!(
            choose_position(Some(&saved), &monitors, Some(&monitors[0]), WINDOW, 0).position,
            PhysicalPosition::new(-2565, 2)
        );
    }

    #[test]
    fn legacy_state_without_resolution_restores_and_upgrades() {
        let monitors = vec![
            monitor("A", 0, 0, 3840, 2160),
            monitor("B", -2560, 0, 2560, 1440),
        ];
        let saved = saved("B", None, None, -5, 2);
        let resolved = choose_position(Some(&saved), &monitors, Some(&monitors[0]), WINDOW, 0);
        assert_eq!(resolved.position, PhysicalPosition::new(-2565, 2));
        let upgraded = resolved.saved.expect("upgraded state");
        assert_eq!(upgraded.monitor_width, Some(2560));
        assert_eq!(upgraded.monitor_height, Some(1440));
    }

    #[test]
    fn resolution_mismatch_falls_back_to_main() {
        let monitors = vec![
            monitor("A", 0, 0, 1920, 1080),
            monitor("B", 1920, 0, 1280, 720),
        ];
        let saved = saved("B", Some(1920), Some(1080), 100, 50);
        assert_eq!(
            choose_position(Some(&saved), &monitors, Some(&monitors[0]), WINDOW, 0).position,
            PhysicalPosition::new(100, 50)
        );
    }

    #[test]
    fn missing_monitor_uses_the_main_monitor() {
        let monitors = vec![monitor("A", 0, 0, 1920, 1080)];
        let saved = saved("Z", Some(1920), Some(1080), 100, 50);
        assert_eq!(
            choose_position(Some(&saved), &monitors, Some(&monitors[0]), WINDOW, 0).position,
            PhysicalPosition::new(100, 50)
        );
    }

    #[test]
    fn outside_main_monitor_cascades() {
        let monitors = vec![monitor("A", 0, 0, 1920, 1080)];
        let saved = saved("Z", Some(1920), Some(1080), 5000, 50);
        assert_eq!(
            choose_position(Some(&saved), &monitors, Some(&monitors[0]), WINDOW, 0).position,
            PhysicalPosition::new(0, 0)
        );
    }

    #[test]
    fn off_screen_on_matching_monitor_cascades() {
        let monitors = vec![monitor("A", 0, 0, 1920, 1080)];
        let saved = saved("A", Some(1920), Some(1080), 5000, 50);
        assert_eq!(
            choose_position(Some(&saved), &monitors, Some(&monitors[0]), WINDOW, 0).position,
            PhysicalPosition::new(0, 0)
        );
    }

    #[test]
    fn cascade_uses_window_width_and_wraps_rows() {
        assert_eq!(cascade_position(0, 0, 2000, 720, 0), (0, 0));
        assert_eq!(cascade_position(0, 0, 2000, 720, 1), (720, 0));
        assert_eq!(cascade_position(0, 0, 2000, 720, 2), (10, 5));
        assert_eq!(cascade_position(0, 0, 2000, 720, 3), (730, 5));
        assert_eq!(cascade_position(0, 0, 2000, 720, 4), (20, 10));
    }
}
