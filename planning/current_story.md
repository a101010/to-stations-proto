# Current story: paws

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the game-controller window application (`paws`): a Slint window with an Xbox-style virtual controller, local stick and button interaction, and a controller-selection dropdown that shows `N/A` until binding exists. Add it to the Rust build/deploy and launch/cleanup scripts.

- **Depends on:** none.
- **Minimal test:** `launch-rust.ps1` brings up the `paws` window showing the virtual controller; sticks move and buttons highlight.

## Context

- `paws` is the controller GUI named in `architecture.md` and lives at `rust/ui/paws`.
- `dds-echo` already proves the Slint 1.18 toolchain, build script, and Windows windowing backend.
- No joystick IDL, joystick control service, station identity, or game-controller binding exists yet.
- Real game-controller binding and the joystick control service are explicitly later, not-yet-broken-out work.
- `paws` is a per-station application, so multiple instances are legitimate; do not use the `single-instance` guard.
- The Rust launch script now starts one `paws` window as part of the configuration.
- `paws` needs no DDS participant, contracts, IDL, or assets for this story.

## Decisions

1. **Slint window, no DDS.** `paws` is a standalone Slint UI with local state only.
2. **Xbox-style virtual controller drawn with Slint primitives.** Rectangles, rounded rectangles, and circles; no image assets.
3. **Local interactivity.** Sticks drag and return to center; buttons, bumpers, and triggers highlight while pressed.
4. **Controller selection is a dropdown, not a bind button.** Reserve header space for a controller `ComboBox` whose only entry is `N/A`; it is disabled until binding exists.
5. **Binding is deferred.** No gamepad detection, no real binding, no station identity, and no DDS in this story.
6. **No single-instance guard.** Multiple `paws` windows are allowed.
7. **Launch integration is in scope.** One `paws.exe` is started by `launch-rust.ps1` after the display windows are raised, and `cleanup.ps1` stops it.
8. **Release is a GUI application.** Keep the `windows_subsystem` attribute so the release build has no console window.
9. **No shared Slint component extraction yet.** The controller layout stays inside `paws.slint` until a second consumer appears.

## Scope

**In scope**

- New `rust/ui/paws` crate and `paws.slint`.
- Window layout and virtual-controller visuals.
- Local stick drag and button press states.
- Rust stick-normalization helper with unit tests.
- Build, deploy, launch, and cleanup integration.
- Planning and architecture updates.

**Out of scope**

- Real game-controller detection or binding.
- Joystick IDL, topics, contracts, DDS, or the joystick control service.
- Persisting controller state.
- Station identity or per-station routing.
- C++ mirror.

## UI design

Window:

- Title `paws`.
- Preferred size about 960x640; minimum about 720x480.
- Dark background with a header, a large centered controller body, and a footer.

Header:

- `paws` title.
- A `Controller` label and a disabled `ComboBox` with the single entry `N/A`.
- No bind button.

Controller body:

- Rounded central body with two grips.
- Left and right analog sticks, each an outer ring plus a movable inner knob.
- D-pad on the left and four face buttons on the right.
- LB/RB bumpers and LT/RT triggers at the top.
- Start/back in the center.
- Labels: `A`, `B`, `X`, `Y`, `LB`, `RB`, `LT`, `RT`, `Back`, `Start`.

Interaction:

- Each stick's `TouchArea` reports the pointer position relative to the stick center.
- Rust clamps the vector to the stick radius and normalizes each axis to `[-1, 1]`.
- The knob position is derived from the normalized values.
- Releasing a stick resets it to zero.
- `TouchArea.pressed` drives the visual highlight for buttons, bumpers, and triggers.

Footer:

- `Local virtual controller. Game-controller binding and DDS are not connected yet.`

## Crate and code

`rust/ui/paws/`:

- `Cargo.toml` with the `paws` binary, `slint = "1.18"`, and `build-dependencies: slint-build = "1.18"`.
- `build.rs` compiles `ui/paws.slint`.
- `src/main.rs`:
  - `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`.
  - `slint::include_modules!()`.
  - Create `MainWindow`, wire stick callbacks, run.
- `src/stick.rs`:
  - Pure `clamp_stick(dx: f32, dy: f32, radius: f32) -> (f32, f32)`.
  - Clamp the vector length to the radius, normalize to `[-1, 1]`, and preserve direction.
  - Unit tests for center, edge, outside, and diagonal input.
- `ui/paws.slint`:
  - Properties for both stick values, button/trigger pressed states, and the controller selection model.
  - Reusable `Stick` and `ControllerButton` components inside the file.
  - No external assets.

## Build and launch integration

- Add `ui/paws` to the `rust/Cargo.toml` workspace members.
- Add `rust\ui\paws\src` to the scan list in `scripts/build.ps1`.
- Copy `build\rust\release\paws.exe` to `build\deploy\bin` and update the deployed-apps message.
- In `scripts/launch-rust.ps1`, start one `paws.exe` from `build\deploy\bin` after the display windows are raised, so the controller window opens on top.
- In `scripts/cleanup.ps1`, add `paws` to the stopped image-name list.
- No changes to `docs/versions.md`: Slint is already recorded.

## Tests

Automated:

- `stick::clamp_stick` unit tests.
- `cargo test --manifest-path rust/Cargo.toml`.
- `.\scripts\build.ps1 -Test`.
- `.\scripts\build.ps1`.

Manual:

1. Run `.\scripts\launch-rust.ps1`; verify the `paws` window opens with the virtual controller.
2. Verify the controller dropdown shows `N/A` and is reserved in the header.
3. Drag both sticks; verify the knobs move and return to center.
4. Press the face buttons, D-pad, bumpers, and triggers; verify the highlights.
5. Verify no console window appears for the release build.
6. Launch a second `paws.exe` manually; verify a second window opens.
7. Run `.\scripts\cleanup.ps1`; verify `paws` stops.
8. Launch `build\deploy\bin\paws.exe` from an unrelated working directory; verify it still runs.

## Acceptance

- `paws.exe` builds and deploys to `build/deploy/bin`.
- `launch-rust.ps1` starts one `paws` window and raises it after the displays.
- `cleanup.ps1` stops `paws`.
- The window shows a recognizable Xbox-style virtual controller.
- Sticks and buttons respond locally.
- The header reserves a controller dropdown showing `N/A`.
- No DDS participant, topics, contracts, station identity, or gamepad binding are added.
- Multiple instances are allowed.
- The release build has no console window.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-rust.ps1
.\scripts\cleanup.ps1
```

One manual launch of the fleet plus one manual second-instance check; no screenshots and no repeated UI runs.

## Risks

- **Slint layout:** percentage positioning and aspect preservation need care; test at minimum and preferred sizes.
- **Binding loops:** keep knob geometry derived from normalized values without circular bindings.
- **Launch ordering:** start `paws` after the display windows are raised so the controller is visible.
- **Cleanup:** add `paws` to the image-name stop list or a stale window survives a cleanup.
- **Disabled dropdown:** the reserved `N/A` selector must read as intentionally unavailable, not as a broken control.
- **Scope creep:** gamepad binding, DDS, station ids, and per-station routing are explicitly deferred.
- **No automated UI test:** the visual/interaction check remains manual, consistent with the story’s minimal test.

## Files touched

- `rust/Cargo.toml`
- `rust/ui/paws/Cargo.toml` (new)
- `rust/ui/paws/build.rs` (new)
- `rust/ui/paws/src/main.rs` (new)
- `rust/ui/paws/src/stick.rs` (new)
- `rust/ui/paws/ui/paws.slint` (new)
- `scripts/build.ps1`
- `scripts/launch-rust.ps1`
- `scripts/cleanup.ps1`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `format-chooser-format` was completed at `2026-11-12 10:46:13` and moved to `completed_stories.md` under "Later increments".
- `paws` is `in progress` in `backlog.md`.
- Do not move `paws` to `completed_stories.md` until the user asks to plan the next story.
