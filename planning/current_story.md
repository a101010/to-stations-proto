# Current story: xbox-controller

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add Xbox-style controller input to `paws`, one window per display id, with per-display persistence of the last selected controller and automatic reselection when that controller appears. Keep DDS out of scope.

- **Depends on:** none.
- **Minimal test:** launch one paws per display id; select an Xbox controller for one display and verify it restores on restart only for that display. Select `Virtual` and verify the mouse works while controller input is ignored. Connect the remembered controller later and verify it is selected automatically.

## Context

- `paws` currently has mouse-only controls and a disabled `N/A` controller dropdown.
- `paws` currently launches once and has no display id.
- `gilrs` 0.11.2 provides connected-controller enumeration, stable UUIDs, connection and disconnection events, and cached axis/button state.
- `app-state` provides per-user atomic TOML state files keyed by discriminator.
- `single-instance` provides per-identity Windows named-mutex guards.
- `lil-d` already has per-display window-position persistence in `rust/ui/lil-d/src/position.rs`; this story extracts it so `paws` can share the same monitor and fallback rules.
- The current Xbox controller reports an all-zero UUID. The default gilrs WGI backend does not enumerate it; the XInput backend does.
- The first-launch regressions were caused by verification deleting the real runtime state, not by the restore or matching code.
- No joystick IDL, joystick control service, or DDS exists yet.
- `paws` is per display: each window is tied to one display id and its format-manager controller services later.

## Decisions

1. **Use `gilrs` 0.11.2 with the `xinput` backend** for controller enumeration and state. The default WGI backend does not see the current controller.
2. **One paws per display id.** `paws` takes a required `--display-id <id>`.
3. **Persist per display id** in `%LOCALAPPDATA%\to-stations\paws\<display_id>.toml`.
4. **Remembered controller identity** is `(name, UUID, last gilrs id)`. Match by UUID and name, and prefer the saved gilrs id to disambiguate identical devices.
5. **Startup:** if the remembered controller is connected, select it; otherwise select `Virtual`. Never auto-select a different controller.
6. **Appearance:** if the remembered controller appears later, select it automatically, even if `Virtual` or another controller is active.
7. **Explicit `Virtual`:** clears the remembered controller, so it will not auto-reselect.
8. **Different controller selected:** replaces the remembered controller.
9. **Disconnect:** if the selected remembered controller disconnects, fall back to `Virtual` but keep it remembered so it auto-reselects when it reconnects.
10. **Filter:** list only controllers whose name contains `Xbox` or `XInput` (case-insensitive).
11. **Modes:** `Virtual` uses mouse control; controller mode mirrors the selected device and ignores mouse control.
12. **Single instance per display:** acquire `paws-display-<id>`; a duplicate for the same display id exits.
13. **No DDS, station identity, or control service** in this story.
14. **Launch integration:** `launch-rust.ps1` starts one paws per display id; `cleanup.ps1` already stops all paws.
15. **Window position persistence (tracked by the separate `window-placement` story):** extract `lil-d`'s position logic into a shared `window-position` library and use it from both apps. `paws` restores and saves its position per display id in the same state file as the controller.
16. **Enumeration retry:** while a remembered controller exists but is not selected, re-enumerate every 500 ms until it is found or the selection clears.
17. **Zero-UUID matching:** treat an all-zero saved UUID as unknown. Match by name first, preferring the saved gilrs id. If the name changed, fall back to the saved id, or to the only controller if exactly one exists; reject ambiguous multiple matches.
18. **Verification isolation:** never delete the real `%LOCALAPPDATA%\to-stations` state; GUI smoke tests use a temporary `LOCALAPPDATA`.

## Scope

**In scope**

- `--display-id` and one paws window per display id.
- Per-display state persistence for the remembered controller.
- `gilrs` controller enumeration, selection, state polling, and Xbox filtering.
- Virtual mode and controller mode switching.
- Controller-state mirroring for sticks, face buttons, D-pad, bumpers, and triggers.
- Mouse-control gating in controller mode.
- Per-display window-position restore and save, sharing `lil-d`'s monitor and visibility rules.
- Build/deploy/launch/cleanup integration.
- Planning, architecture, and version documentation.

**Out of scope**

- DDS, joystick topics, contracts, and the joystick control service.
- Cursor movement, format-manager commands, and display control.
- Station identity.
- C++ mirror.
- Force feedback.

## Display identity and persistence

`rust/ui/paws/src/state.rs` (new):

```rust
#[derive(Serialize, Deserialize, Default)]
pub(crate) struct SavedState {
    #[serde(default)]
    pub(crate) controller: Option<SavedController>,
    #[serde(default)]
    pub(crate) position: Option<window_position::SavedPosition>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub(crate) struct SavedController {
    pub(crate) name: String,
    pub(crate) uuid: [u8; 16],
    pub(crate) id: usize,
}
```

- Load via `app_state::state_path("paws", &display_id.to_string())`.
- Save after a user selection:
  - controller selected: save its identity;
  - `Virtual` selected: save `controller: None`.
- Missing, unreadable, or malformed state falls back to `Virtual`.
- Save failures are logged and do not block the UI.
- The display id is shown in the window header, for example `paws #0`.

## Window position persistence

**Ownership:** the detailed placement plan below belongs to the separate `window-placement` story. It is kept here because the working tree already contains the first part of that story; `window-placement` also adds inner-size persistence and `dds-echo`.

Extract `rust/ui/lil-d/src/position.rs` into a new shared `rust/lib/window-position` crate:

- `SAVE_INTERVAL` (250 ms).
- `SavedPosition` and `ResolvedPosition`.
- `should_save`.
- `load(application, display_id)` and `save(application, display_id, position)`.
- `restore_or_cascade(window, application, display_id)`.
- `restore_or_cascade_with(saved, window, display_id)`.
- `saved_from_window` and `saved_from_window_at`.

Update `lil-d` to use `window-position` with application name `"lil-d"` and remove its local `position.rs`.

Paws:

- Enable Slint's `unstable-winit-030` feature.
- Load `SavedState.position` at startup.
- After `ui.show()`, attempt:
  - `ui.window().with_winit_window(...)`;
  - `window_position::restore_or_cascade_with(saved, winit_window, display_id)`;
  - `winit_window.set_outer_position(resolved.position)`;
  - store and save the resolved position.
- If the winit window is not ready, retry from the 16 ms timer.
- Track `ui.window().position()` in the same timer and save at most every `SAVE_INTERVAL`.
- Flush the position on `on_close_requested` with `force = true`.
- Always write `SavedState { controller, position }` so controller and position do not clobber each other.

## Controller selection semantics

`rust/ui/paws/src/gamepad.rs` (new):

```rust
pub(crate) struct Choice {
    pub(crate) id: gilrs::GamepadId,
    pub(crate) name: String,
    pub(crate) uuid: [u8; 16],
}

pub(crate) enum Selection {
    Virtual,
    Gamepad(gilrs::GamepadId),
}

pub(crate) fn xbox_choices(gilrs: &gilrs::Gilrs) -> Vec<Choice>;
pub(crate) fn selection_for_index(index: i32, choices: &[Choice]) -> Selection;
pub(crate) fn find_remembered(
    choices: &[Choice],
    saved: &SavedController,
) -> Option<gilrs::GamepadId>;
```

Rules:

- Dropdown index 0 is `Virtual`; index N is `choices[N - 1]`.
- On `Connected`, refresh the list. If the remembered controller is now available, select it. Otherwise leave the current selection alone.
- On `Disconnected`, refresh the list. If the selected controller disappeared, select `Virtual` and keep the remembered identity.
- Auto-reselection must never choose a controller that does not match the remembered identity.
- A user-selected controller changes the remembered identity and is saved.
- A user-selected `Virtual` clears and saves the empty identity.
- Controller labels include the controller name and id, for example `Xbox Wireless Controller (0)`.

### Controller detection hardening

- `remembered_index` treats an all-zero saved UUID as unknown.
- Matching order:
  1. exact name + UUID + preferred gilrs id;
  2. exact name + UUID;
  3. zero UUID: name match, preferring the saved id;
  4. zero UUID with a changed name: saved id, or the only controller if exactly one exists;
  5. non-zero UUID with a changed name: unique UUID match;
  6. otherwise no match.
- If multiple controllers remain ambiguous, do not auto-select.
- While a remembered controller exists but is not selected, re-enumerate about every 500 ms and retry matching.
- Only refresh the dropdown model when the controller list actually changes.
- Stop retrying once the remembered controller is selected or the remembered identity is cleared.
- When a match is found under a changed backend name, update the remembered identity and save it on startup so the state self-heals.
- Keep the existing `Connected`/`Disconnected` refresh behavior.

### Verification isolation

- Add an `AGENTS.md` rule: never delete or overwrite the real `%LOCALAPPDATA%\to-stations` state.
- GUI smoke tests launch processes with a temporary `LOCALAPPDATA` and remove only that directory.
- The first-launch `lil-d` monitor change and the `paws` `Virtual` fallback were caused by deleting the real state directories during verification.

## Input mapping

`rust/ui/paws/src/controls.rs` (new):

```rust
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) struct Controls {
    pub(crate) left: (f32, f32),
    pub(crate) right: (f32, f32),
    pub(crate) a: bool,
    pub(crate) b: bool,
    pub(crate) x: bool,
    pub(crate) y: bool,
    pub(crate) lb: bool,
    pub(crate) rb: bool,
    pub(crate) lt: bool,
    pub(crate) rt: bool,
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left_dpad: bool,
    pub(crate) right_dpad: bool,
    pub(crate) back: bool,
    pub(crate) start: bool,
}

pub(crate) trait PadState {
    fn axis(&self, axis: gilrs::Axis) -> f32;
    fn pressed(&self, button: gilrs::Button) -> bool;
}

impl PadState for gilrs::ev::state::GamepadState { ... }

impl Controls {
    pub(crate) fn read(pad: &impl PadState) -> Self;
    pub(crate) fn neutral() -> Self;
    pub(crate) fn apply(&self, ui: &MainWindow);
}
```

Mappings:

| Controller | UI |
|---|---|
| `Axis::LeftStickX/Y` | left stick, with Y inverted |
| `Axis::RightStickX/Y` | right stick, with Y inverted |
| `Button::South/East/West/North` | A/B/X/Y |
| `Button::LeftTrigger/RightTrigger` | LB/RB |
| `Axis::LeftZ/RightZ` above threshold | LT/RT |
| `Button::DPadUp/Down/Left/Right` | D-pad |
| `Button::Select` | Back |
| `Button::Start` | Start |

Use an 0.08 stick deadzone and clamp to `[-1, 1]`.

## UI changes

`rust/ui/paws/ui/paws.slint`:

- Header shows `paws #<display-id>`.
- Replace the disabled `N/A` ComboBox:
  - `in-out property <[string]> controllers: ["Virtual"];`
  - `in-out property <int> controller-index: 0;`
  - enabled always;
  - `selected(index) => { root.controller-selected(index); }`.
- Add `in-out property <bool> virtual-mode: true;`.
- Add button properties: `a-down`, `b-down`, `x-down`, `y-down`, `lb-down`, `rb-down`, `lt-down`, `rt-down`, `up-down`, `down-down`, `left-down`, `right-down`, `back-down`, `start-down`.
- Refactor `ControllerButton`:
  - `down` is supplied by the parent;
  - `TouchArea` emits `pressed-changed(bool)`;
  - each instance forwards to `control-pressed("<name>", value)`.
- Add `in property <bool> interactive` to `Stick`, bound to `root.virtual-mode`.
- Add callbacks:
  - `controller-selected(int)`
  - `control-pressed(string, bool)`.
- Footer reflects mode:
  - Virtual: `Virtual mode: use the mouse or select a connected controller.`
  - Controller: `Controller mode: the window mirrors the selected controller.`

## Rust wiring

`rust/ui/paws/src/main.rs`:

- Parse `--display-id` with `clap`.
- Acquire the `paws-display-<id>` guard before loading state or creating the UI.
- Initialize `Gilrs` best-effort; if it fails, run with only `Virtual`.
- Load `SavedState` for the display id.
- Create the UI, populate the controller model, and restore the remembered controller if present.
- Wire `controller-selected`, `control-pressed`, `stick-drag`, and `stick-release`.
  - Stick and button callbacks update the UI only in `Virtual` mode.
- Add a 16 ms `slint::Timer` that:
  - drains gilrs events;
  - refreshes the controller list on connect/disconnect;
  - auto-selects the remembered controller when it appears;
  - falls back to `Virtual` on disconnect;
  - applies the selected controller state to the UI;
  - leaves controls untouched in `Virtual` mode.
- Keep the timer alive until `ui.run()` returns.

## Launch integration

`scripts/launch-rust.ps1`:

- After raising the display windows, start one paws per display id:
  - `paws.exe --display-id <id>`.
- Raise each paws window.
- Update the launch message.
- `scripts/cleanup.ps1` already stops all paws processes; no change.
- `scripts/build.ps1` already scans and deploys paws; no change.

## Documentation updates

- `docs/versions.md`: record `gilrs` 0.11.2.
- `planning/architecture.md`:
  - Paws section: one window per display id, persisted controller selection, window position, auto-reselection rules, no DDS.
  - Service and display instances: add `paws-display-<id>`.
  - Runtime state examples: add `paws\<display_id>.toml` with controller and position.
  - Shared Rust libraries: add `window-position`, used by `lil-d` and `paws`.
- `planning/backlog.md`, `planning/completed_stories.md`, and `planning/current_story.md`: keep the story state current.

## Tests

Automated:

- `Controls::neutral` is all zero and false.
- Axis clamp, Y inversion, and deadzone behavior.
- Face buttons, bumpers, triggers, D-pad, back, and start map correctly.
- `selection_for_index`: 0 -> `Virtual`, 1 -> first controller, out-of-range -> `Virtual`.
- `find_remembered`:
  - exact UUID and name match;
  - saved id preferred when names and UUIDs collide;
  - zero UUID matches by name and prefers the saved id;
  - zero UUID with a changed name matches the saved id;
  - zero UUID with a changed name and one controller matches it;
  - zero UUID with multiple ambiguous matches -> none;
  - non-zero UUID with a changed name and a unique UUID -> match;
  - no match -> none;
  - different controller -> none.
- Startup retry re-enumerates every 500 ms while the remembered controller is pending.
- The XInput backend enumerates the current controller; the WGI backend does not.
- Selection-state rules:
  - user selects a controller -> remembered and saved;
  - user selects `Virtual` -> remembered cleared and saved;
  - disconnect -> `Virtual` but remembered kept;
  - remembered appears -> selected;
  - other controller appears -> not selected.
- Window-position tests move to `window-position` and cover monitor-relative round trip, preferred-monitor restore, resolution mismatch, off-screen fallback, and cascade.
- `SavedState` round trip includes both controller and position.
- Existing `stick::clamp_stick` tests still pass.

Manual (use a temporary `LOCALAPPDATA`; never delete the real state):

1. Launch the fleet with two display ids; verify two paws windows, each labelled with its id.
2. Select an Xbox controller for display 0; verify only that window mirrors it.
3. Restart; verify display 0 restores that controller and display 1 stays `Virtual`.
4. Select `Virtual` for display 0; restart; verify it stays `Virtual`.
5. Disconnect the selected controller; verify fallback to `Virtual`.
6. Reconnect it; verify automatic reselection.
7. Connect a different Xbox controller; verify it is listed but not auto-selected.
8. Verify the dropdown contains only Xbox/XInput controllers.
9. With an all-zero-UUID controller, verify startup auto-selection still works.
10. Move each paws window to a different position, close it, and relaunch; verify each restores its own position.
11. Move a paws window to another monitor, close, and relaunch; verify it returns there when that monitor is present.
12. Remove or change the saved monitor/resolution; verify fallback to the main monitor or cascade instead of off-screen.
13. Verify controller selection is still restored after position saves.
14. Run `cleanup.ps1`; verify all paws windows stop.

## Acceptance

- `paws` accepts required `--display-id`, and one window launches per display id.
- The dropdown lists `Virtual` plus connected Xbox-style controllers with ids.
- The remembered controller restores per display id; no other controller auto-selects.
- A remembered controller appearing later is auto-selected.
- Explicit `Virtual` clears the remembered controller.
- Controller mode mirrors the controller and ignores mouse input.
- `Virtual` mode uses mouse input and ignores controller input.
- State persists in `paws\<display_id>.toml`.
- Each paws window restores its own window position per display id.
- Controller selection and window position coexist in the same state file.
- Startup retries controller enumeration and zero-UUID controllers match by name/id.
- Verification uses a temporary `LOCALAPPDATA`; real runtime state is never deleted.
- No DDS, IDL, contracts, or control-service changes.
- `gilrs` 0.11.2 is recorded in `docs/versions.md`.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-rust.ps1
.\scripts\cleanup.ps1
```

One manual two-display controller run, including restart and reconnect checks; no screenshots and no repeated UI runs.

## Risks

- **Persistence identity:** UUID may be shared by identical controllers; use the saved gilrs id as a tiebreaker.
- **Stable ids:** gilrs ids are session-scoped, so UUID and name are the primary key.
- **Auto-reselection:** only the remembered controller may reselect; guard against choosing a different device.
- **Explicit Virtual:** clear the remembered controller or it reselects immediately.
- **Per-display state:** two paws instances must never share a display id; use the guard.
- **Launch order:** raise each paws window after its display; multiple windows may overlap.
- **Button refactor:** moving state from component-local to MainWindow properties touches every control.
- **Mouse gating:** sticks and buttons must not visually react to mouse in controller mode.
- **Gilrs backend:** the Windows default WGI backend does not enumerate the current controller; use the XInput backend.
- **Slint winit accessor:** requires the `unstable-winit-030` feature and a matching winit 0.30 version.
- **Restore timing:** the winit window may not exist immediately after `show()`; retry from the timer if needed.
- **Shared-file writes:** position and controller saves must always write the full `SavedState`.
- **Existing state compatibility:** files without `[position]` must still load.
- **lil-d refactor:** moving `position.rs` must preserve lil-d's monitor and fallback behavior.
- **Zero-UUID ambiguity:** multiple same-name controllers with no matching saved id must not be guessed.
- **Retry loop:** only the remembered controller may be selected during the retry; stop when it is selected or the remembered identity is cleared.
- **Test isolation:** deleting real runtime state caused the false first-launch regressions; never repeat that.
- **No automated UI test:** manual position and controller checks remain necessary.

## Files touched

- `AGENTS.md`
- `docs/versions.md`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`
- `rust/Cargo.lock`
- `rust/Cargo.toml`
- `rust/lib/window-position/Cargo.toml` (new)
- `rust/lib/window-position/src/lib.rs` (new)
- `rust/ui/lil-d/Cargo.toml`
- `rust/ui/lil-d/src/app.rs`
- `rust/ui/lil-d/src/main.rs`
- `rust/ui/lil-d/src/position.rs` (deleted, moved into `window-position`)
- `rust/ui/paws/Cargo.toml`
- `rust/ui/paws/src/main.rs`
- `rust/ui/paws/src/gamepad.rs` (new)
- `rust/ui/paws/src/controls.rs` (new)
- `rust/ui/paws/src/state.rs` (new)
- `rust/ui/paws/ui/paws.slint`
- `scripts/launch-rust.ps1`

## Bookkeeping

- `paws` was completed at `2026-11-12 15:33:37` and moved to `completed_stories.md` under "Later increments".
- `xbox-controller` is `in progress` in `backlog.md`.
- `window-placement` was added to `backlog.md` as `todo`, depending on `xbox-controller`; it covers position and inner-size persistence for `lil-d`, `paws`, and `dds-echo`.
- Do not move `xbox-controller` to `completed_stories.md` until the user asks to plan the next story.
