# Current story: hello-lux-color

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Activate the existing `Lux Color` menu item as the color-picker overlay: an opaque black overlay with a hue ring around a saturation/value triangle, Back and Done buttons, mouse and controller picking, and the menu item reading `World` while the overlay is open.

- **Depends on:** hello-ctl-service.
- **Minimal test:** open `Lux Color`, pick a color, and press Done; the light color changes. Reopen, change the color, and press Back or B; the previous color returns.

## Context

- `hello-ctl-service` is complete. The view service already owns per-display color, overlay state, and the `Lux Color`/`World` menu label.
- The view topic carries RGB `color`; it has no HSV state, so white and black would lose hue.
- The display library already has the placeholder `Lux Color` menu action, the black format background, the single crosshair cursor, and the top-band menu.
- Paws already sends A/right-click, D-pad, Start, B, and R3 commands. Back is not yet sent.
- The color overlay is not persisted.

## Decisions

1. **The Lux Color overlay is opaque black.** The Earth and greeting are hidden while it is open.
2. **The picker is a hue ring around a saturation/value triangle**, with a visible hue marker and SV marker.
3. **Back is immediately left of Done** in a row below the picker.
4. **The menu item reads `World` while Lux Color is open.** Selecting it cancels and restores the pre-overlay color.
5. **Done commits the current color and closes the overlay.**
6. **Back and B cancel and restore the pre-overlay color.** Back also cancels when captured.
7. **Opening the overlay snapshots RGB and HSV.** Canceling restores the snapshot; committing keeps the current color.
8. **No persistence.** The color, HSV, overlay state, focus, and capture reset on restart.
9. **The view topic carries `hue`, `saturation`, and `value`** in addition to RGB `color`.
10. **Mouse picking is left-button only.** Left press/drag on the ring sets hue; left press/drag on the triangle sets saturation/value. Local right-click remains a no-op.
11. **D-pad focus order is `HueRing -> Triangle -> Back -> Done`**, reverse in the opposite direction.
12. **Cursor+A picks a control or button.** Picking ring/triangle focuses and captures that control. Picking Back/Done activates the button.
13. **A when focus is Back/Done activates that button.** A when focus is ring/triangle and the cursor is not over another control/button enters capture of the focused control.
14. **The virtual cursor is captured while adjusting a ring/triangle.** It is hidden during capture and restored when capture ends.
15. **The right stick is rate-based while captured.** It adjusts hue on the ring or saturation/value in the triangle; it does not move the cursor.
16. **B releases capture first when the menu is closed.** With capture released, the next B cancels/restores the overlay. B with the menu open closes the menu first.
17. **Start releases capture and opens the menu.** Start with the menu open closes it. Opening the menu with the mouse three-dot button also releases capture.
18. **R3 keeps its existing arcball behavior outside the overlay.** Inside Lux Color it does not toggle arcball.
19. **R3 changes ring/triangle focus only when the cursor is over the ring or triangle.** While captured, R3 switches ring/triangle focus and keeps capture.
20. **D-pad focus from ring/triangle to Back/Done releases capture.**
21. **D-pad focus between ring and triangle while captured keeps capture.**
22. **A local mouse press releases controller capture first**, then handles the mouse action.
23. **Selecting another menu item cancels/restores the color overlay first**, then performs the selected action.
24. **Selecting Lux Pos while Lux Color is open cancels/restores Lux Color first**, then toggles the Lux Pos placeholder.
25. **The Lux Pos item remains a label-cycling placeholder** in this story.
26. **Pointer input remains mouse-only.** Touch is deferred.
27. **C++ remains out of scope.**

## Capture transitions

### Enter capture

1. Cursor+A over the hue ring.
2. Cursor+A over the SV triangle.
3. A while focus is hue ring/triangle and the cursor is not over another control/button.
4. Mouse drag is separate and does not enter controller capture.

### Exit capture

1. B with menu closed: release capture only; overlay stays open; focus stays.
2. B with menu open: close menu; capture remains released.
3. B with menu closed and not captured: cancel/restore and close overlay.
4. Start while captured: release capture, then open menu.
5. Start while menu open: close menu; capture stays released.
6. Back: cancel/restore and close overlay.
7. Done: commit and close overlay.
8. World menu item: cancel/restore and close overlay.
9. Formats/Inspect/Auto/Lux Pos selected: cancel/restore first, then perform that action.
10. D-pad focus moves from ring/triangle to Back/Done: capture releases automatically.
11. D-pad focus moves between ring/triangle while captured: capture remains.
12. R3 while captured: switch ring/triangle focus; capture remains.
13. Local mouse press anywhere in the overlay: release controller capture first.
14. Overlay closes or changes to none/Lux Pos: reset focus to HueRing and clear capture.
15. Window focus loss/controller disconnect: deferred; capture stays until an explicit action.

## Scope

**In scope**

- Real Lux Color overlay rendering.
- Hue ring, SV triangle, hue/SV markers, focus highlight.
- Back and Done buttons with Back left of Done.
- HSV state in the view service and view topic.
- Mouse picking and dragging on ring/triangle.
- Controller focus, capture, rate-based adjustment, and R3 behavior.
- Start/B/Back/World/Done behavior.
- Cancel/restore and commit semantics.
- Paws Back command.
- Menu cancel-first behavior for other items.
- Tests and documentation.

**Out of scope**

- Lux Pos overlay.
- Persistence.
- Touch input.
- C++ implementation.
- Changes to the global hello model.

## View service

### State

- RGB `color`.
- `hue`, `saturation`, `value`.
- Overlay state: none, Lux Color, Lux Pos placeholder.
- Pre-overlay snapshot: RGB and HSV.
- No focus or capture state; those are display-library UI state.

### Actions

Add `HelloWorldInputTopic` actions:

- `HELLO_ACTION_COLOR_HUE` - set normalized hue.
- `HELLO_ACTION_COLOR_SV` - set saturation and value.
- `HELLO_ACTION_COLOR_COMMIT` - commit and close.
- `HELLO_ACTION_COLOR_CANCEL` - restore and close.

Extend existing behavior:

- `HELLO_ACTION_TOGGLE_LUX_COLOR` opens Lux Color or, when already open, cancels/restores.
- `HELLO_ACTION_TOGGLE_LUX_POS` while Lux Color is open cancels/restores Lux Color first, then toggles the Lux Pos placeholder.
- `HELLO_ACTION_MODE_TOGGLE` while Lux Color is open cancels/restores Lux Color first, then toggles Auto/Inspect.
- Ignore arcball grab/drag/release/rotate and R3 arcball toggles while Lux Color is open.

### Color math

- Convert HSV to RGB for `color`.
- Clamp hue/saturation/value to `[0, 1]`.
- Keep hue stable when saturation or value reaches zero.
- Default color is white: hue 0, saturation 0, value 1.

## IDL, topics, and contracts

Extend `HelloWorldViewTopic` with:

```idl
double hue;
double saturation;
double value;
```

Keep the existing RGB `color`.

- No new topics.
- No contract changes.
- Regenerate Rust types and `build/topics.md`.
- Update `dds-echo` view decode/send schemas and field tests.

## Display library

### Color picker module

Add `rust/lib/display-lib/src/color_picker.rs`.

- `PickerLayout`:
  - center,
  - outer and inner ring radii,
  - triangle vertices,
  - Back and Done rectangles.
- `PickerHit`:
  - HueRing,
  - Triangle,
  - Back,
  - Done,
  - None.
- Geometry:
  - hue angle starts at top and advances clockwise,
  - triangle top is white,
  - triangle bottom-right is the current hue,
  - triangle bottom-left is black,
  - barycentric coordinates map to saturation/value.
- Rendering:
  - opaque black background over the whole format region,
  - hue ring shader/geometry,
  - SV triangle shader/geometry,
  - hue marker,
  - SV marker,
  - focus highlight,
  - Back and Done labels.
- Tests for hit testing, hue mapping, barycentric mapping, and marker positions.

### HelloWorld state

Add local overlay UI state:

- focus: HueRing, Triangle, Back, Done,
- captured: bool,
- mouse picking: none, ring, triangle,
- last virtual cursor position before capture if needed.

Render:

- If view overlay is Lux Color: draw the opaque picker instead of Earth/greeting.
- Otherwise render Earth/greeting as today.
- Draw focus highlight for the focused control.
- Hide the virtual crosshair while captured.

Input:

- Mouse left press/drag on ring/triangle picks.
- Mouse left click on Back/Done activates.
- Mouse press anywhere releases controller capture.
- Controller CursorMove while captured adjusts the focused control rate-based; otherwise it remains a cursor move.
- D-pad changes focus in the confirmed order.
- A/right-click uses cursor hit first, then focused Back/Done activation, then focused ring/triangle capture.
- R3 follows the overlay rules.
- B follows menu/capture/cancel layering.
- Back sends color cancel.
- Start releases capture before menu open.
- Overlay transitions reset focus/capture.

### Format manager

- Pass the virtual cursor to format commands that need it.
- Add a way for an active format to hide the virtual cursor while capturing.
- When the menu opens (Start or three-dot), release active capture.
- When Formats or another menu item is selected while Lux Color is open, cancel/restore the overlay first.
- Preserve existing menu, chooser, and cursor behavior.

### Format source

- Map the new Paws Back command to an overlay-cancel format command.
- Keep existing Start/B/pointer/R3 mappings.

## Paws

- Add a Back press-edge command.
- Send `FORMAT_COMMAND_OVERLAY_CANCEL` on Back.
- Keep Start, B, A/right-click, D-pad, and R3 behavior.
- No new Paws buttons.

## Menu behavior

- `Lux Color` label is `World` while the overlay is open.
- Selecting `World` cancels/restores.
- Selecting `Lux Pos` cancels/restores Lux Color first, then toggles the Lux Pos placeholder.
- Selecting `Formats` or `Inspect/Auto` cancels/restores Lux Color first.
- The three-dot button remains visible over the black overlay.
- Back/Done are overlay controls, not menu items.

## Tests

Automated:

- HSV/RGB conversion and clamping.
- Hue angle, ring hit testing, and hue marker position.
- SV barycentric mapping, triangle hit testing, and marker position.
- Back/Done hit testing.
- Focus cycle order.
- Capture entry/exit transition table.
- R3 focus behavior.
- Mouse ring/triangle/Back/Done handling.
- View-service open/cancel/commit/World/Lux-Pos/mode transitions.
- Menu labels for Lux Color/World and Lux Pos placeholder.
- Paws Back command edge.
- View-topic HSV field round-trip and dds-echo schema completeness.

Manual, using a temporary `LOCALAPPDATA`:

1. Open Lux Color from the menu; verify the opaque black overlay, ring, triangle, Back, and Done.
2. Verify Back is immediately left of Done.
3. Mouse-drag the hue ring; verify the hue marker and current color state.
4. Mouse-drag the SV triangle; verify saturation/value.
5. Mouse Back restores the previous color.
6. Reopen, mouse Done commits the picked color.
7. Controller D-pad focus cycles Ring -> Triangle -> Back -> Done.
8. Cursor+A on ring/triangle captures; right stick adjusts rate-based; B releases capture.
9. A on focused Back/Done activates.
10. R3 outside the overlay toggles arcball; inside the overlay it follows the ring/triangle focus rules.
11. Start releases capture and opens the menu; Start closes it.
12. World cancels/restores; Lux Pos, Formats, and Inspect/Auto cancel Lux Color first.
13. Display 1 is unaffected.
14. No color state is persisted.
15. Existing left globe drag, right-click no-op, single crosshair, and Start/B behavior remain.

## Acceptance

- Lux Color renders as an opaque picker with ring, triangle, Back, and Done.
- Back is left of Done.
- Mouse and controller can pick hue and saturation/value.
- Controller capture, B release, R3 focus, D-pad order, and Start menu release work as specified.
- Done commits; Back/B/World cancel and restore.
- Other menu items cancel the overlay first.
- No persistence.
- Contracts, registry, `dds-echo`, and view-topic fields stay consistent.
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

- **Color math:** HSV/RGB conversion and hue stability at black/white.
- **Picking geometry:** ring/triangle hit testing and barycentric mapping.
- **Capture state:** many entry/exit paths; keep the state machine explicit and tested.
- **Cursor ownership:** display library owns the crosshair; formats must be able to hide it while capturing.
- **Menu priority:** Start/B/menu actions must not conflict with capture.
- **Format changes:** changing format must cancel the overlay and restore color.
- **No persistence:** restart resets color and overlay state by design.
- **Test isolation:** use a temporary `LOCALAPPDATA`.

## Files touched

- `DDS/hello_world.idl`
- `rust/lib/format-list/src/lib.rs`
- `rust/lib/stations-dds/src/lib.rs`
- `rust/lib/stations-dds/tests/field_metadata.rs`
- `rust/service/hello-view-service/src/relay.rs`
- `rust/lib/display-lib/src/color_picker.rs` (new)
- `rust/lib/display-lib/src/formats/hello_world.rs`
- `rust/lib/display-lib/src/formats.rs`
- `rust/lib/display-lib/src/format_manager.rs`
- `rust/lib/display-lib/src/format_source.rs`
- `rust/lib/display-lib/src/formats/format_chooser.rs`
- `rust/lib/display-lib/src/view.rs`
- `rust/lib/display-lib/src/lib.rs`
- `rust/ui/paws/src/commands.rs`
- `rust/ui/paws/src/controls.rs`
- `rust/tool/dds-echo/src/decode.rs`
- `rust/tool/dds-echo/src/send.rs`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `hello-ctl-service` was completed at `2026-11-17 14:44:11` and moved to `completed_stories.md` under "Later increments".
- `hello-lux-color` is `in progress` in `backlog.md`.
- `hello-lux-pos` remains `todo`, depending on `hello-lux-color`.
- Do not move `hello-lux-color` to `completed_stories.md` until the user asks to plan the next story.
