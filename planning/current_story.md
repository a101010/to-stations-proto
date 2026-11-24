# Current story: cpp-lux-color

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add the C++ Lux Color overlay with the hue ring, saturation/value triangle, preview, shared Back/Done row, focus and capture, mouse and controller picking, wheel latch, and view actions.

- **Depends on:** none.
- **Minimal test:** the Rust Lux Color acceptance test passes with C++ `lil-d` and C++ services.

## Context

- `cpp-hello-view-ctl` is complete at `2026-11-24 12:37:25`.
- The C++ view service already supports Lux Color and Lux Pos state, HSV updates, commits, cancels, snapshots, and frozen Lux Pos attitude.
- The C++ display library currently renders only the placeholder overlays; Lux Color and Lux Pos are not drawn.
- The Rust display library already has the complete Lux Color implementation in `color_picker.rs` and `formats/hello_world.rs`.
- C++ `format-list` already has all hello color/light action constants.
- C++ `lil-d` uses Rust `paws`; `paws` sends the controller commands and zero-stick rest events needed by the picker.
- Hello view state is intentionally not persisted.
- `launch-rust.ps1` must remain unchanged.

## Decisions

1. **Port the Rust `color_picker` module to C++.**
   - Keep geometry and input logic separate from GL rendering so it can be tested without a GL context.
2. **Use the existing display-library architecture.**
   - `HelloWorld` owns picker focus/capture and forwards color actions.
   - `ColorPicker` owns the shader, geometry, rendering, hit testing, and shared Back/Done row.
3. **Reuse the Rust shader and layout exactly.**
   - Same hue-ring/triangle shader.
   - Same proportions, focus order, wheel latch, and capture rules.
4. **No service changes.**
   - C++ `hello-view-service` and `hello-ctl-service` already implement the color state and actions.
5. **No Lux Pos work.**
   - Keep existing Lux Pos placeholder behavior.
   - Design Back/Done geometry for reuse by `cpp-lux-pos`.
6. **No IDL, contract, persistence, launch, or deploy changes.**
   - Only the C++ display library changes.

## Scope

**In scope**

- C++ `color_picker` module.
- C++ Lux Color rendering.
- C++ mouse and controller picking.
- Picker focus/capture and wheel latch.
- Shared Back/Done row.
- Display-library tests and C++ architecture/planning updates.

**Out of scope**

- C++ Lux Pos rendering.
- C++ `paws`.
- DDS/IDL/contract changes.
- Service changes.
- Persistence.
- Changes to `launch-cpp.ps1` or `launch-rust.ps1`.

## Files

```text
cpp/libs/display-lib/src/color_picker.hpp
cpp/libs/display-lib/src/color_picker.cpp
cpp/libs/display-lib/src/formats/hello_world_impl.hpp
cpp/libs/display-lib/src/formats/hello_world.cpp
cpp/libs/display-lib/tests/main.cpp
cpp/libs/display-lib/CMakeLists.txt
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No `planning/architecture.md` change is needed; it already documents the C++ `color_picker` module.

## Implementation

### 1. Add `color_picker` geometry and logic

Create these public display-library types:

- `PickerControl { HueRing, Triangle, Back, Done }`
- `PickerHit { HueRing, Triangle, Back, Done, None }`
- `WheelLatch`
- `PickerLayout`
- `ButtonRow`
- `ColorPicker`

Port the Rust behavior exactly:

- `PickerControl::next(direction)` cycles `HueRing -> Triangle -> Back -> Done`.
- `WheelLatch`:
  - zero movement clears the latch;
  - first movement chooses direction from `dx - dy`, then tie-breakers;
  - magnitude is `sqrt(dx² + dy²)` capped at `20`;
  - direction remains latched through circular movement.
- `PickerLayout`:
  - scale from `scale_for_height(height)`;
  - center at `x + width/2`, `y + height * 0.44`;
  - ring outer `0.32 * min(width, height)`;
  - ring inner `0.78 * outer`;
  - triangle radius `0.86 * inner`;
  - button width `120 * scale`, height `44 * scale`;
  - button gap `12 * scale`, margin `12 * scale`;
  - preview height `36 * scale`;
  - Back immediately left of Done;
  - preview between ring and buttons;
  - clamp the button row inside the region.
- Hit testing:
  - Back/Done first;
  - ring annulus;
  - then triangle.
- Hue:
  - angle from `(x - cx, -(y - cy))`;
  - normalized to `[0, 1)`.
- Saturation/value:
  - barycentric triangle weights;
  - saturation `w1`;
  - value `w0 + w1`;
  - clamp saturation to value.
- Markers:
  - hue marker at mid-ring;
  - SV marker from barycentric weights;
  - triangle clamp projects outside points back inside.
- Shared `ButtonRow` returning Back, Done, and font size for Lux Pos reuse.

### 2. Add the GL renderer

Implement `ColorPicker` using existing `shader` and `ui` helpers:

- Constructor:
  - link the Rust vertex/fragment shader;
  - create a VAO/VBO with the six quad vertices;
  - cache uniform locations;
  - construct a `TextRenderer` from the deploy font.
- Destructor:
  - delete program, VAO, and VBO.
- Render:
  - clear the whole format region black;
  - draw the ring and triangle with the shader;
  - draw focus outline around ring/triangle;
  - draw hue and SV markers, with a larger marker while captured;
  - draw RGB preview rectangle and border;
  - draw Back/Done using the shared button renderer;
  - restore normal GL program/VAO state after drawing.

Export `draw_button` from the picker module so Lux Pos can reuse the same Back/Done rendering.

### 3. Integrate with `HelloWorld`

Add state to `HelloWorld`:

- `ColorPicker picker_`
- `PickerControl picker_focus_`
- `bool picker_captured_`
- `std::optional<PickerHit> mouse_picking_`
- `std::optional<std::pair<float, float>> triangle_cursor_`
- `WheelLatch wheel_`
- `int last_overlay_`

Add helpers:

- `sync_overlay(int overlay)`
- `clear_capture()`
- `send_commit()`
- `send_cancel()`
- `focus_picker(control, capture)`
- `picker_hit_at(cursor)`
- `toggle_picker_focus()`
- `move_triangle_cursor(dx, dy)`
- `move_hue_wheel(dx, dy)`
- `color_state()` returning hue, saturation, value

Overlay behavior:

- On overlay change:
  - reset focus to HueRing;
  - capture only when entering Lux Color;
  - clear mouse picking, triangle cursor, and wheel;
  - reset drag state.
- `cursor_hidden()` returns `picker_captured_`.

Rendering:

- In `render`, call `sync_overlay(view->overlay)`.
- If the overlay is Lux Color:
  - render only the picker;
  - hide Earth and greeting;
  - pass hue/saturation/value, output color, triangle marker, focus, and capture.
- Otherwise preserve existing hello-world rendering.
- Leave Lux Pos rendering unchanged for the next story.

Mouse input:

- In Lux Color, any local press first releases capture.
- Only left button is handled.
- Press:
  - Back cancels;
  - Done commits;
  - hue ring focuses and captures picking, then sends hue;
  - triangle focuses and captures picking, then sends saturation/value;
  - empty area does nothing.
- Release clears mouse picking.
- Return consumed for left-button events in the overlay.
- Mouse motion:
  - releases capture;
  - if hue/triangle picking is active, sends the updated value;
  - otherwise ignored.

Controller commands in Lux Color:

- `Navigate`: cycle focus in the requested direction, retaining capture.
- Right pointer/A press:
  - Back cancels;
  - Done commits;
  - otherwise use cursor hit to focus and capture ring/triangle;
  - fall back to focused Back/Done;
  - otherwise capture the currently focused ring/triangle.
- `ToggleArcball`/R3:
  - if captured, switch ring/triangle focus;
  - otherwise focus ring/triangle under cursor;
  - never toggle arcball inside the overlay.
- `CursorMove` while captured:
  - zero movement clears the wheel latch;
  - ring focus moves hue with the latched wheel;
  - triangle focus moves the triangle cursor in screen x-y;
  - Back/Done do nothing.
- `CancelMenu`:
  - release capture when captured;
  - otherwise cancel and restore the color.
- `OverlayCancel`: cancel and restore.
- `ReleaseCapture`: clear capture.
- Uncaptured cursor movement falls through so the format-manager cursor can move.
- `ToggleMenu` remains for the menu layer.

Menu behavior:

- Mode toggle cancels Lux Color first, then sends the mode toggle.
- Lux Color item:
  - detect whether it is opening or closing;
  - send the toggle action;
  - reset focus, capture, picking, triangle cursor, and wheel.
- Lux Pos remains the existing placeholder action.

### 4. CMake and tests

- Add `src/color_picker.cpp` to `display_lib`.
- Add `src/color_picker.cpp` to `display-lib-tests`.
- Include `color_picker.hpp` in the test main.
- Extend `display-lib-tests` with the Rust picker tests:
  - wheel latch keeps direction through a circle;
  - wheel latch changes direction only after rest;
  - wheel speed is capped;
  - Back is immediately left of Done;
  - preview sits between ring and buttons;
  - triangle clamping;
  - focus cycle order and reverse;
  - ring/triangle/Back/Done hit tests;
  - hue/SV marker round trips.
- CTest count remains 13; `display-lib-tests` is expanded.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

Manual acceptance with a temporary `LOCALAPPDATA` and `launch-cpp.ps1`:

1. Open `Lux Color` from the menu.
2. Verify:
   - opaque black overlay;
   - hue ring, SV triangle, RGB preview, and Back/Done;
   - Back is left of Done and shared positions are used.
3. Pick a hue and saturation/value with the mouse:
   - preview and Earth light color update;
   - Done commits the new color.
4. Reopen, change the color, then cancel with Back, B, or the World menu item:
   - previous color is restored.
5. Verify controller behavior with Rust `paws`:
   - initial hue-ring focus and captured cursor;
   - D-pad focus order retains capture;
   - right stick hue wheel with direction latch;
   - triangle x-y movement;
   - R3 switches ring/triangle focus;
   - A/right-click activates ring, triangle, Back, or Done;
   - mouse motion releases capture.
6. Confirm Lux Color state remains per display and is not persisted.
7. Confirm `launch-rust.ps1` still works unchanged.

## Risks

- GL shader/layout must match the Rust picker closely enough for consistent hit testing and appearance.
- Coordinate conversion between top-left picking and OpenGL bottom-left viewport is error-prone.
- Capture must be released by local mouse press/motion but retained by D-pad focus changes.
- R3 must not toggle arcball inside Lux Color.
- Color cancel must restore both RGB and HSV snapshots through the view service.
- Do not implement Lux Pos rendering in this story.

## Bookkeeping

- `cpp-hello-view-ctl` is completed at `2026-11-24 12:37:25`.
- `cpp-lux-color` is `in progress` in `backlog.md`.
- `cpp-lux-color` has no remaining story prerequisites.
- Do not move `cpp-lux-color` to `completed_stories.md` until the user asks to plan the next story.
