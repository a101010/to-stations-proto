# Current story: cpp-lux-pos

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add the C++ Lux Pos overlay with the frozen globe, live lighting, spotlight can, front-hemisphere mapping, Back/Done, rate-based right-stick control, R3 no-op, crosshair retention, and overlay transitions.

- **Depends on:** none.
- **Minimal test:** the Rust Lux Pos acceptance test passes with C++ `lil-d` and C++ services.

## Context

- `cpp-lux-color` is complete at `2026-11-24 13:58:20`.
- The C++ view service already owns light state, light snapshots, frozen Lux Pos attitude, and light commit/cancel actions.
- The C++ display library already has Lux Color and the shared Back/Done geometry/renderer.
- The Rust display library implements Lux Pos in `light_pos.rs` and `formats/hello_world.rs`.
- Rust `paws` is still the controller source for the C++ configuration.
- Hello view state is intentionally not persisted.
- `launch-rust.ps1` must remain unchanged.

## Decisions

1. **Port the Rust `light_pos` module to C++.**
   - Add `LightFocus`, `LightHit`, `LightLayout`, `front_light`, `light_stick_delta`, and `Spotlight`.
   - Reuse `button_row` from the completed C++ `color_picker` module.
2. **Keep Lux Pos behavior identical to Rust.**
   - Same globe layout, spotlight-can shader, front-hemisphere mapping, and Back/Done positions.
   - Same overlay transitions and frozen-attitude view.
3. **No service changes.**
   - C++ `hello-view-service` already provides light state, snapshots, frozen Lux Pos attitude, and commit/cancel actions.
4. **No Lux Color changes beyond shared helpers.**
   - Reuse the existing Back/Done row and button renderer.
5. **No C++ `paws` or deploy changes.**
   - Rust `paws` provides controller input for this story.
6. **No IDL, contract, persistence, or launch changes.**

## Scope

**In scope**

- C++ `light_pos` module.
- C++ Lux Pos rendering.
- Mouse and controller light movement.
- Back/Done focus and overlay transitions.
- Display-library tests and architecture/planning updates.

**Out of scope**

- C++ `paws`.
- Service changes.
- Persistence.
- Changes to `launch-cpp.ps1` or `launch-rust.ps1`.
- Changes to Lux Color behavior beyond shared layout helpers.

## Files

```text
cpp/libs/display-lib/src/light_pos.hpp
cpp/libs/display-lib/src/light_pos.cpp
cpp/libs/display-lib/src/formats/hello_world_impl.hpp
cpp/libs/display-lib/src/formats/hello_world.cpp
cpp/libs/display-lib/tests/main.cpp
cpp/libs/display-lib/CMakeLists.txt
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No new deployable files, assets, contracts, or scripts.

## Implementation

### 1. Add `light_pos` geometry and helpers

Create:

- `LightFocus { Back, Done }`
- `LightHit { Back, Done, None }`
- `LightLayout`
- `next_light_focus(LightFocus, int direction)`
- `front_light(float x, float y)`
- `light_stick_delta(int dx, int dy, const Rect& region)`
- `Spotlight`

`LightFocus` rules:

- default focus is Back;
- `Back -> Done` forward;
- `Done -> Back` backward;
- forward from Done stays Done.

`LightLayout`:

- use `button_row(region)` for shared Back/Done positions and font size;
- globe margin `12 * scale`;
- globe sits above the Back/Done row;
- globe side is `min(width, max(1, buttons.back.y - margin - top))`;
- globe centered horizontally;
- radius from `earth::silhouette_radius(side, side)`;
- default colors and fallback state remain view-driven.

`front_light`:

- clamp normalized x-y to the unit disk;
- derive `z = sqrt(max(0, 1 - x² - y²))`.

`LightLayout` helpers:

- `hit(x, y)` checks Back/Done;
- `pointer_xy(x, y)` maps globe pointer coordinates to normalized front-hemisphere x-y;
- `marker(x, y)` projects normalized light x-y using a `1.05 * radius` marker radius;
- `can_angle(marker)` points the spotlight toward the globe center;
- `spotlight_size()` is `max(34 * scale, 12)`.

`light_stick_delta`:

- `half = min(region.w, region.h) / 2`;
- scale is `0.25 / half`;
- return `(dx * scale, -dy * scale)`, matching Rust.

### 2. Add the spotlight renderer

Port the Rust spotlight shader and renderer:

- Quad vertices `[-1,1]`.
- Vertex shader:
  - rotate the quad by `u_angle`;
  - translate by `u_marker` in region coordinates;
  - project using `u_region_size`.
- Fragment shader:
  - body, nose, and lens formulas from Rust;
  - lens uses the current light color;
  - body uses neutral gray;
  - alpha is the shape mask.
- `Spotlight`:
  - own program, VAO, VBO;
  - cache `u_marker`, `u_angle`, `u_size`, `u_region_size`, `u_color`;
  - render with blending enabled;
  - set viewport to the format region;
  - restore GL program/VAO/blend state.

### 3. Integrate with `HelloWorld`

Add state:

- `Spotlight spotlight_`
- `LightFocus light_focus_ = LightFocus::Back`
- `bool light_dragging_ = false`
- `std::optional<std::pair<float, float>> light_cursor_`

Add helpers:

- `light_state()` returning the current view light or default `{0.6, 0.8, 0.7}`;
- `send_light(x, y, z)`;
- `send_light_commit()`;
- `send_light_cancel()`;
- `move_light(dx, dy)`.

Update `sync_overlay(int overlay)`:

- continue resetting picker state when the overlay changes;
- reset `light_focus_ = Back`;
- set `light_dragging_ = false`;
- when opening Lux Pos, set `light_cursor_ = front_light(light.x, light.y)` from the current light;
- otherwise clear `light_cursor_`.

Rendering:

- keep the existing background clear;
- if overlay is Lux Color, render the picker and return;
- if overlay is Lux Pos:
  - create `LightLayout`;
  - set the OpenGL viewport to the globe area;
  - render Earth with the frozen view attitude, live light, and color;
  - compute the spotlight marker from the live light;
  - render the spotlight can at the marker angle/size;
  - draw Back/Done with the shared button renderer;
  - return without drawing the greeting.
- otherwise preserve existing hello-world rendering.
- Do not add Lux Pos rendering to Lux Color.

Mouse input:

- Lux Pos handles only left-button input.
- Press:
  - Back: focus Back and send light cancel;
  - Done: focus Done and send light commit;
  - globe area: begin dragging, map pointer to front-hemisphere light, store cursor, send light.
- Release: clear dragging.
- Mouse motion:
  - if dragging, update light from pointer and send it;
  - otherwise ignored.
- Right-click remains ignored.

Controller commands:

- `Navigate`: cycle Back/Done focus.
- Right pointer/A:
  - Back cancels;
  - Done commits;
  - empty area activates the focused Back/Done.
- `ToggleArcball`/R3: consumed and does nothing.
- `CursorMove`:
  - nonzero movement moves the light using `light_stick_delta`;
  - zero movement is consumed with no light change;
  - the virtual crosshair stays at its last position.
- `CancelMenu` and `OverlayCancel`: cancel and restore the light.
- `ReleaseCapture`: stop light dragging.
- `ToggleMenu`: leave to the menu layer.

Menu behavior:

- Mode toggle cancels active Lux Pos first, then sends mode toggle.
- Lux Color cancels Lux Pos first.
- Lux Pos:
  - cancels Lux Color first when open;
  - sends the Lux Pos toggle;
  - clears picker capture/picking;
  - stops light dragging.
- World menu item cancels Lux Pos through `OverlayCancel`.

Crosshair:

- `cursor_hidden()` remains true only for captured Lux Color.
- Lux Pos never captures the cursor.

### 4. CMake and tests

- Add `src/light_pos.cpp` to `display_lib`.
- Add `src/light_pos.cpp` to `display-lib-tests`.
- Include `light_pos.hpp` in the display-library tests.
- Add tests:
  - Back/Done match `button_row` from Lux Color;
  - globe sits above Back/Done;
  - `front_light` clamps to the disk and gives `z = 1` at the center;
  - marker is inside the globe and points inward;
  - focus cycles between Back and Done correctly.
- CTest count remains 13; `display-lib-tests` is expanded.

### 5. Architecture

Update the C++ display-library split to include `color_picker` and `light_pos`, which now match the Rust display-library modules.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

Manual acceptance with a temporary `LOCALAPPDATA` and `launch-cpp.ps1`:

1. Open `Lux Pos` from the menu.
2. Verify:
   - frozen globe at the opening attitude;
   - live lighting;
   - spotlight-can marker pointing toward the globe center;
   - Back/Done shared positions and Back default focus.
3. Move the light with the mouse:
   - lighting changes;
   - Done commits;
   - Back/B/World cancels and restores the previous light.
4. Reopen and verify the previous attitude remains frozen while the light moves.
5. Verify controller behavior with Rust `paws`:
   - D-pad Back/Done focus;
   - A/right-click activation;
   - right stick moves the light while the crosshair stays put;
   - R3 does nothing;
   - zero right-stick movement does nothing.
6. Verify overlay transitions:
   - Lux Color cancels Lux Pos;
   - Lux Pos cancels Lux Color;
   - mode change cancels Lux Pos;
   - menu selection cancels Lux Pos.
7. Confirm two displays remain independent and `launch-rust.ps1` still works.

## Risks

- The spotlight shader and globe viewport coordinate conversion must match the Rust rendering.
- `Earth::render` expects the caller to set the globe viewport.
- Lux Pos must not capture the cursor or hide it.
- R3 must be consumed without toggling arcball.
- Cursor-move commands must not move the virtual crosshair in Lux Pos.
- Do not persist light or overlay state.
- Avoid changing Lux Color behavior while adding shared `button_row` usage.

## Bookkeeping

- `cpp-lux-color` is completed at `2026-11-24 13:58:20`.
- `cpp-lux-pos` is `in progress` in `backlog.md`.
- `cpp-lux-pos` has no remaining story prerequisites.
- Do not move `cpp-lux-pos` to `completed_stories.md` until the user asks to plan the next story.
