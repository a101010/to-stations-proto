# Current story: hello-lux-pos

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Activate the existing `Lux Pos` menu item as the light-position overlay: show the globe frozen at the attitude it had when the overlay opened, with a live lighting preview and an iconic spotlight-can marker. Mouse dragging and controller trackball input move the light over the user-facing hemisphere. Back and Done use the same positions as in Lux Color, and the menu item reads `World` while the overlay is open.

- **Depends on:** none.
- **Minimal test:** open `Lux Pos`, move the light, and press Done; the lighting changes. Reopen, move it again, and press Back or B; the previous light position returns.

## Context

- `hello-lux-color` is complete. The view service already owns per-display light, overlay state, and the `Lux Pos`/`World` menu label.
- The view topic already carries the light vector and overlay state; no new IDL fields are expected.
- The Earth renderer already uses the light vector and light color.
- The display library already has shared picker layout/button positions, the single crosshair cursor, and the top-band menu.
- `Lux Pos` currently opens a label-cycling placeholder.
- Paws already sends right-stick movement, D-pad, A/right-click, Start, B, and Back commands. R3 toggles arcball outside overlays.
- The light overlay is not persisted.

## Decisions

1. **The Lux Pos overlay has a black background.** The Earth and spotlight marker are the visual focus.
2. **The globe is frozen at the attitude shown when Lux Pos opens.** It stays fixed while the light moves.
3. **The globe is rendered in a square area above the button row.** The greeting is hidden.
4. **Back and Done use the same positions as Lux Color.** Back is left of Done.
5. **Default D-pad focus is Back.** D-pad left/right switches between Back and Done.
6. **A/right-click activates the focused button.** Mouse left-click activates Back or Done directly.
7. **There is no color preview rectangle in Lux Pos.**
8. **The menu item reads `World` while Lux Pos is open.** Selecting it cancels and restores the previous light.
9. **Done commits the current light and closes the overlay.**
10. **Back and B cancel and restore the previous light.**
11. **Opening the overlay snapshots the current light and effective attitude.** The attitude is model attitude in Auto and local attitude in Inspect.
12. **No persistence.** Light, frozen attitude, overlay state, focus, and drag state reset on restart.
13. **Light is a view-space direction constrained to the user-facing hemisphere.** `z >= 0`.
14. **Light control is always active for the controller.** There is no capture state in Lux Pos.
15. **The virtual crosshair remains visible.** Controller right-stick movement moves the light, not the crosshair.
16. **R3 does nothing inside Lux Pos.** It does not toggle arcball or light control.
17. **The spotlight marker is an iconic 2D spotlight can pointing toward the globe center.** The lens/glow uses the current light color; the body is neutral.
18. **Mouse left-drag on the globe moves the light.** Local right-click remains a no-op.
19. **Controller right stick moves the light rate-based in screen x/y.** Radius is clamped to 1; `z = sqrt(1 - x^2 - y^2)`.
20. **The menu remains reachable with Start and the three-dot button.**
21. **Selecting another menu item cancels/restores Lux Pos first**, then performs the selected action.
22. **Selecting Lux Color while Lux Pos is open cancels Lux Pos first**, then opens Lux Color.
23. **Selecting Lux Pos while Lux Color is open cancels Lux Color first**, then opens Lux Pos.
24. **The C++ mirror remains out of scope.**

## Scope

**In scope**

- Real Lux Pos overlay rendering.
- Frozen globe attitude and live light preview.
- Front-hemisphere light mapping and marker.
- Iconic spotlight-can rendering.
- Back and Done at the same positions as Lux Color.
- D-pad focus for Back/Done and A activation.
- Mouse and controller light movement.
- Menu `World` behavior and cross-overlay transitions.
- Tests and documentation.

**Out of scope**

- Persistence of light state.
- R3 changes.
- C++ implementation.
- Changes to the global hello model.
- Touch input.

## Overlay layout

- Black background over the format region.
- Globe square area above the Back/Done row.
- Back and Done use the same rectangles as Lux Color.
- Back is left of Done.
- Spotlight can is placed at the projected light direction.
- Spotlight can rotates to point toward the globe center.
- No color preview rectangle.
- Greeting hidden.
- Menu button remains over the overlay.

## View service

### State

- Light vector.
- Pre-Lux-Pos light snapshot.
- Frozen attitude snapshot.
- Overlay state: none, Lux Color, Lux Pos.

### Actions

Add `HelloWorldInputTopic` actions:

- `HELLO_ACTION_LIGHT` - set the light from normalized front-hemisphere x/y/z.
- `HELLO_ACTION_LIGHT_COMMIT` - commit the current light and close.
- `HELLO_ACTION_LIGHT_CANCEL` - restore the previous light and close.

Extend existing behavior:

- `HELLO_ACTION_TOGGLE_LUX_POS` opens Lux Pos or, when already open, cancels/restores.
- `HELLO_ACTION_TOGGLE_LUX_COLOR` while Lux Pos is open cancels/restores Lux Pos first, then opens Lux Color.
- `HELLO_ACTION_TOGGLE_LUX_POS` while Lux Color is open cancels/restores Lux Color first, then opens Lux Pos.
- `HELLO_ACTION_MODE_TOGGLE` while Lux Pos is open cancels/restores Lux Pos first, then toggles Auto/Inspect.
- Ignore arcball grab/drag/release/rotate and R3 arcball toggles while Lux Pos is open.

### Light rules

- Clamp incoming light to the front hemisphere.
- Normalize the light direction.
- Publish the frozen attitude while Lux Pos is open.
- Publish the current light for the live preview.
- Cancel restores the pre-overlay light.
- Commit keeps the current light.

## Display library

### New `light_pos` module

Add `rust/lib/display-lib/src/light_pos.rs`.

- `LightLayout`:
  - globe square area,
  - globe center and screen radius,
  - Back/Done rectangles using the same position helper as Lux Color,
  - spotlight marker position and orientation.
- Shared button-row helper with `color_picker.rs` so Back/Done positions stay identical.
- Front-hemisphere helpers:
  - normalized x-y -> clamped disk -> z,
  - light direction -> marker screen position,
  - marker position -> can rotation angle.
- Spotlight-can renderer:
  - iconic 2D can shape,
  - body neutral, lens/glow in the current light color,
  - rotated to point toward the globe center.
- Hit tests for Back/Done.
- Tests for marker projection, can orientation, front-hemisphere clamping, and button positions.

### Earth screen radius

Expose a helper from `earth.rs` for the globe's projected screen radius so the spotlight marker sits on/just outside the rendered globe silhouette.

### HelloWorld

Add Lux Pos local state:

- light cursor x-y for smooth controller movement,
- mouse light-drag state,
- focused button: Back or Done, default Back.

Render:

- If overlay is Lux Pos:
  - black background,
  - Earth in the globe square with frozen attitude and current light,
  - spotlight can,
  - Back/Done with focus highlight,
  - greeting hidden,
  - crosshair still drawn.
- Otherwise render as today.

Input:

- Mouse left press on Back/Done activates.
- Mouse left press on globe starts light drag.
- Mouse motion updates the light.
- Mouse release ends drag.
- Local right-click is a no-op.
- Controller right stick always moves the light rate-based; it does not move the virtual cursor.
- D-pad focus switches Back/Done.
- A/right-click activates the focused button.
- R3 is consumed as a no-op in Lux Pos.
- Start toggles the menu.
- B closes the menu or cancels/restores Lux Pos.
- Back cancels/restores.
- World menu item cancels/restores.
- Overlay transitions reset light cursor, focus to Back, and drag state.
- `cursor_hidden` stays false for Lux Pos.

## Menu behavior

- `Lux Pos` label is `World` while the overlay is open.
- Selecting `World` cancels/restores.
- Selecting `Lux Color`, `Formats`, or `Inspect`/`Auto` cancels Lux Pos first.
- The three-dot button remains visible.
- Start/B menu behavior remains.

## Tests

Automated:

- Front-hemisphere mapping, radius clamping, and light normalization.
- Spotlight marker projection and can orientation.
- Back/Done positions match Lux Color.
- Default focus is Back; D-pad switches Back/Done.
- A/right-click activates focused Back/Done.
- R3 is a no-op in Lux Pos.
- Right stick moves light and leaves the crosshair position unchanged.
- Mouse drag moves light; local right-click is a no-op.
- View-service open snapshots light and attitude.
- Frozen attitude remains fixed while the model updates.
- Light updates produce the expected view light.
- Cancel restores the previous light; commit keeps it.
- Lux Color/Lux Pos transitions cancel the current overlay first.
- Mode toggle cancels Lux Pos first.
- Menu labels for Lux Color/World and Lux Pos/World.
- No persistence.

Manual, using a temporary `LOCALAPPDATA`:

1. Open Lux Pos; verify the frozen globe, spotlight can, Back/Done, and default Back focus.
2. Mouse-drag the globe; verify marker movement and live lighting.
3. Controller right stick moves the light; verify the crosshair stays put.
4. R3 does nothing.
5. D-pad focuses Back/Done; A activates.
6. Done commits; reopen, change the light, and Back/B restore.
7. World cancels/restores.
8. Lux Color while Pos is open cancels Pos first.
9. Start/three-dot menu remains usable.
10. Display 1 is unaffected.
11. Existing Lux Color, globe drag, cursor, and menu behavior remain green.

## Acceptance

- Lux Pos renders the frozen globe, live lighting, and spotlight-can marker.
- Back/Done use the same positions as Lux Color; default focus is Back.
- Mouse and controller can move the light over the front hemisphere.
- R3 does nothing in Lux Pos; the crosshair remains visible.
- Done commits; Back/B/World cancel and restore.
- Other menu items cancel Lux Pos first.
- No persistence.
- Display 1 is unaffected.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-rust.ps1
.\scripts\cleanup.ps1
```

One manual two-display run with a temporary `LOCALAPPDATA`.

## Risks

- **Front-hemisphere mapping:** marker and light must stay on the user-facing side.
- **Frozen attitude:** model updates must not change the Lux Pos globe.
- **Marker projection:** the spotlight must sit on the rendered globe silhouette and point inward.
- **Crosshair ownership:** Lux Pos keeps the crosshair visible while the stick moves the light.
- **Menu priority:** Start/B/World must not conflict with light control.
- **Overlay transitions:** Color/Pos transitions must cancel/restore correctly.
- **No persistence:** restart resets light and overlay state by design.
- **Test isolation:** use a temporary `LOCALAPPDATA`.

## Files touched

- `rust/lib/format-list/src/lib.rs`
- `rust/service/hello-view-service/src/relay.rs`
- `rust/lib/display-lib/src/light_pos.rs` (new)
- `rust/lib/display-lib/src/color_picker.rs` (shared button-row layout)
- `rust/lib/display-lib/src/formats/hello_world.rs`
- `rust/lib/display-lib/src/earth.rs`
- `rust/lib/display-lib/src/formats.rs` (only if interfaces change)
- `rust/lib/display-lib/src/format_manager.rs` (only if interfaces change)
- `rust/lib/display-lib/src/lib.rs`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

Paws is expected to need no changes. C++ remains out of scope.

## Bookkeeping

- `hello-lux-color` was completed at `2026-11-18 10:58:52` and moved to `completed_stories.md` under "Later increments".
- `hello-lux-pos` is `in progress` in `backlog.md`.
- `hello-lux-pos` has no remaining story prerequisites.
- Do not move `hello-lux-pos` to `completed_stories.md` until the user asks to plan the next story.
