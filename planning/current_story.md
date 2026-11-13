# Current story: window-placement

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Persist and restore window position and inner size for `lil-d`, `paws`, and `dds-echo` through the shared `window-position` library.

- **Depends on:** none.
- **Minimal test:** move and resize `lil-d`, `paws`, and `dds-echo`; close and relaunch each; verify both size and position restore. Legacy state without size still loads.

## Context

- `window-position` currently stores only position, monitor name, resolution, and relative offset.
- `lil-d` and `paws` restore and save position; neither saves inner size.
- `dds-echo` has no placement persistence.
- `window-position` uses winit types and already supports per-display state.
- `dds-echo` is a singleton and should use a keyed state file, not a display id.
- The `AGENTS.md` rule already requires GUI tests to use a temporary `LOCALAPPDATA` and never touch the real runtime state.
- The controller work is complete and committed; this story only covers placement.

## Decisions

1. **Store inner size**, not outer size: `width` and `height` are `window.inner_size()`.
2. Extend `SavedPosition` and `ResolvedPosition`; keep the existing monitor, visibility, primary-monitor, and cascade rules.
3. **Legacy state without size** keeps the app's default size and upgrades on the next save.
4. Add key-based state APIs so `dds-echo` uses `dds-echo\window.toml`.
5. `lil-d` restores size before position and saves after resize.
6. `paws` restores size, tracks position and size, and saves both in the existing `paws\<display_id>.toml`.
7. `dds-echo` restores and tracks placement in `dds-echo\window.toml`.
8. Do not extract a new placement tracker yet; keep app-specific tracking thin.
9. Verification uses a temporary `LOCALAPPDATA`.

## Scope

**In scope**

- Position plus inner-size persistence in `window-position`.
- Keyed state paths for singleton applications.
- Size restore and save in `lil-d` and `paws`.
- New placement persistence in `dds-echo`.
- Legacy-state compatibility and tests.
- Architecture and planning updates.

**Out of scope**

- Saving outer-window size or decorations.
- Restoring maximized, minimized, or fullscreen state.
- Monitor-layout changes beyond the existing fallback rules.
- Any DDS, controller, or format changes.

## Shared `window-position` changes

`rust/lib/window-position/src/lib.rs`:

```rust
pub struct SavedPosition {
    pub monitor: Option<String>,
    pub monitor_width: Option<u32>,
    pub monitor_height: Option<u32>,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

pub struct ResolvedPosition {
    pub position: PhysicalPosition<i32>,
    pub size: Option<PhysicalSize<u32>>,
    pub saved: Option<SavedPosition>,
}
```

- Add `SavedPosition::size() -> Option<PhysicalSize<u32>>`; reject zero dimensions.
- `saved_from_window_at` captures `window.inner_size()`.
- `choose_position` uses the saved size when present, otherwise the current inner size.
- `ResolvedPosition.size` returns the size to apply.
- Add keyed state functions:

```rust
pub fn load_key(application: &str, key: &str) -> Option<SavedPosition>;
pub fn save_key(application: &str, key: &str, position: &SavedPosition) -> io::Result<()>;
pub fn restore_or_cascade_key(
    window: &winit::window::Window,
    application: &str,
    key: &str,
    cascade_slot: u32,
) -> ResolvedPosition;
```

- Keep the display-id wrappers for `lil-d` and `paws`.
- Update the position tests for the new fields and add:
  - size round trip;
  - legacy state with no size;
  - restored size and position survive save/load.

## `lil-d`

`rust/ui/lil-d/src/app.rs`:

- After `restore_or_cascade`:
  - if `resolved.size` is present, call `window.request_inner_size(size)`;
  - then `set_outer_position(resolved.position)`.
- On `WindowEvent::Resized`:
  - update the GL surface as today;
  - mark the position dirty;
  - flush with the normal throttle.
- `flush_position` continues to call `saved_from_window`, which now includes size.
- On focus loss and close, force-save as today.

## `paws`

`rust/ui/paws/src/main.rs`:

- `SavedState.position` already stores `window_position::SavedPosition`; the size fields are automatic.
- In `restore_position`:
  - apply `resolved.size` via `winit_window.request_inner_size(size)`;
  - then `set_outer_position(resolved.position)`.
- Add `last_size: Option<(u32, u32)>`.
- In `track_position`:
  - compare both `ui.window().position()` and `ui.window().size()`;
  - mark dirty on either change;
  - save at most every 250 ms.
- `flush_position` continues to use `saved_from_window_at`, which captures the actual inner size.
- The resolved state is saved on startup, so legacy files gain width/height.

## `dds-echo`

`rust/tool/dds-echo/`:

- Add `window-position` and Slint's `unstable-winit-030` feature.
- Use `window_position::load_key("dds-echo", "window")` and `save_key`.
- After the existing setup and before `run()`:
  - call `app.show()?`;
  - restore through `app.window().with_winit_window(...)`;
  - apply `resolved.size`, then `resolved.position`;
  - retry from the placement timer if the winit window is not ready.
- Add a placement timer (about 250 ms) that tracks `app.window().position()` and `app.window().size()`.
- Save at most every 250 ms while changed.
- Register `app.window().on_close_requested(...)` to force-save and return `HideWindow`.
- Duplicate launches still activate the existing window and create no new state.
- State path: `%LOCALAPPDATA%\to-stations\dds-echo\window.toml`.

## Verification isolation

- Use a temporary `LOCALAPPDATA` for every GUI smoke test.
- Remove only the temporary directory.
- Never delete or overwrite the real `%LOCALAPPDATA%\to-stations` state.
- The `AGENTS.md` rule is already in place.

## Documentation

- `planning/architecture.md`:
  - `window-position` stores position and inner size.
  - `lil-d\<display_id>.toml` and `paws\<display_id>.toml` include size.
  - Add `dds-echo\window.toml`.
  - Mention all three apps use the shared placement library.
  - Paws section: remembers position and size.
- `docs/versions.md`: no new external component.
- `planning/backlog.md` and `planning/current_story.md`: keep the story state current.

## Tests

Automated:

- `SavedPosition` size round trip.
- Legacy state without size.
- Restored size and position persistence.
- Existing monitor/cascade tests.
- `paws` `SavedState` controller + position + size round trip.
- `cargo test --manifest-path rust/Cargo.toml`.
- `cargo clippy` for changed crates.
- `.\scripts\build.ps1 -Test`.
- `.\scripts\build.ps1`.

Manual, using a temporary `LOCALAPPDATA`:

1. Move and resize `lil-d`; close/relaunch; verify both restore.
2. Move and resize `paws`; close/relaunch; verify both restore and controller selection remains.
3. Move and resize `dds-echo`; close/relaunch; verify both restore.
4. Verify legacy state files without `width`/`height` load and upgrade.
5. Verify the real `%LOCALAPPDATA%\to-stations` state is untouched.

## Acceptance

- `lil-d`, `paws`, and `dds-echo` restore both position and inner size.
- Legacy state remains compatible.
- `paws` keeps controller and placement state in one file.
- `dds-echo` uses `dds-echo\window.toml`.
- `window-position` exposes keyed state for singleton apps.
- No new external dependencies or version changes.
- Verification does not touch real runtime state.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

One manual move/resize/restart pass for all three apps using a temporary `LOCALAPPDATA`; no screenshots and no repeated UI runs.

## Risks

- **Asynchronous resize:** `request_inner_size` may apply after the position; verify no visible jump.
- **Slint size clamping:** minimum/preferred sizes may clamp restored sizes; the next save records the actual size.
- **Outer vs inner size:** mixing them would make restored sizes drift; consistently use inner size.
- **Legacy state:** missing size fields must not fail deserialization.
- **Resize saves:** `lil-d` must mark dirty on `Resized`, not only `Moved`.
- **Timer lifetime:** keep the `dds-echo` placement timer alive for the whole run.
- **Test isolation:** never delete the real state directory.

## Files touched

- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`
- `rust/lib/window-position/src/lib.rs`
- `rust/ui/lil-d/src/app.rs`
- `rust/ui/paws/src/main.rs`
- `rust/ui/paws/src/state.rs` (only if the round-trip test changes)
- `rust/tool/dds-echo/Cargo.toml`
- `rust/tool/dds-echo/src/main.rs`
- `rust/Cargo.lock` (only if dependencies change)
- `docs/versions.md` (only if needed; no expected change)

## Bookkeeping

- `xbox-controller` was completed at `2026-11-13 11:15:47` and moved to `completed_stories.md` under "Later increments".
- `window-placement` is `in progress` in `backlog.md`.
- Do not move `window-placement` to `completed_stories.md` until the user asks to plan the next story.
