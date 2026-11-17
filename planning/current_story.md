# Current story: hello-ctl-service

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the per-display hello_world controller and view foundation: per-display `hello-ctl-service` and `hello-view-service`, ABI v4 local input, a horizontal touch-friendly format menu, Auto/Inspect arcball with inertia, per-display light color and position, and paws right-stick toggle input.

The color-picker and light-position overlays are the next two stories (`hello-lux-color` and `hello-lux-pos`).

- **Depends on:** none.
- **Minimal test:** left-drag the globe to enter Inspect and rotate it; release quickly to impart inertia; toggle Auto/Inspect from the horizontal menu and from the paws right-stick toggle; verify a second display is unaffected.

## Context

- `hello-service` is the global model and stays unchanged.
- `hello-view-service` is currently a singleton and publishes a global view.
- The display library subscribes to the global view without filtering by display id.
- The display ABI is v3 and carries only mouse-button input; it has no motion or wheel events.
- The format menu is currently a vertical single-row `Formats` menu.
- The format-manager command path from paws to the display library already exists.
- `paws` has a display id, right-stick cursor input, D-pad, and A/right-click, but no right-thumb toggle.
- Hello-view-specific state must not be persisted.

## Decisions

1. **`hello-view-service` and `hello-ctl-service` are per display.** Both take `--display-id` and control only that display.
2. **`hello-service` remains global** and unchanged.
3. **The view service owns** Auto/Inspect mode, local attitude, arcball velocity, light color, light position, and overlay state.
4. **No hello-view persistence.** Mode, local attitude, inertia, light color/position, and overlay state reset on restart.
5. **ABI v4 adds mouse motion and wheel**, and the display application forwards all mouse buttons.
6. **Local input path:** display app -> ABI v4 -> display library -> `HelloWorldInputTopic` -> per-display `hello-ctl-service` -> `HelloWorldCtlTopic` -> per-display `hello-view-service` -> `HelloWorldViewTopic` -> display library filtered by display id.
7. **Paws commands are translated by the display library**, not by paws. Paws does not know formats.
8. **The format menu is a horizontal, touch-friendly item bar**, not vertical rows.
9. **The three-dot button and the open menu share the same top band**, flush with the format window's left edge. Opening the menu hides the button; the former button area becomes the first menu item.
10. **Menu items use equal, fixed-width slots with tight consistent gaps and centered labels.** There is no wrap, shrink-to-fit, or overflow fallback. If planned labels or buttons do not fit the slot budget, the menu is replanned.
11. **`Formats` is always the first menu item.** Format-specific items follow it.
12. **Story A menu items are `Formats`, `Inspect`/`Auto`, `Lux Color`/`World`, and `Lux Pos`/`World`.**
13. **`Inspect`/`Auto` shows the next mode**, not the current mode.
14. **The Lux menu items are selectable placeholders in Story A.** Selecting one cycles its label as if its overlay were open (`Lux Color` to `World`, and back), and opens no overlay.
15. **Auto** uses the model attitude and shows the greeting. **Inspect** uses the local attitude and hides the greeting.
16. **Arcball:** left-drag on the globe enters Inspect and rotates with trackball mapping; release velocity produces inertia damped by friction.
17. **Paws right-stick press toggles arcball**; while arcball is active, right-stick movement rotates the globe.
18. **Paws gets a local, clearly visible `R3` toggle near the right stick.** It defaults unchecked, shows `Off`/`On`, toggles on physical right-thumb press or click, and sends the same toggle command. It does not subscribe to view state or know about formats.
19. **The Xbox Start button toggles the three-dot menu, and B cancels it.** Start opens or closes the menu; B closes it if open. Both are high-level control commands relayed through the format-manager path.
20. **No format-manager assignment behavior changes** beyond accepting the new right-thumb and placeholder-overlay control commands; local input routing and menu hit-testing are part of this story.
21. **Pointer input is mouse-only in this increment.** Touch is deferred until hardware is available; no touch-specific code or ABI is added.
22. **The display library tracks the pointer position and pressed buttons.**
23. **Local mouse right-click does nothing.** Only the left button operates the menu, chooser, and hello-world drag. Controller A/right-click remains unchanged.
24. **There is one cursor over the format window: the crosshair.** The native mouse cursor is hidden while over the format region and shown over the console. The crosshair follows the last input source: local mouse or virtual controller cursor.
25. **`dds-echo` decodes and can send the new hello-world topics.**
24. **The C++ mirror is out of scope for this story.**

## Scope

**In scope**

- Per-display hello view and control services.
- New hello-world input/control topics and extended view topic.
- ABI v4 mouse motion/wheel and all-button forwarding.
- Mouse-only pointer input; touch is deferred.
- Local left-button interaction only; local right-click is ignored.
- One crosshair over the format region; native cursor hidden there and shown over the console.
- Display-library pointer position and pressed-button tracking.
- Per-display view filtering.
- Auto/Inspect mode, local attitude, arcball and inertia.
- Per-display light/color state with defaults.
- Horizontal touch-friendly menu in the top band with four items, `Formats` first.
- Selectable Lux Color/World and Lux Pos/World placeholder items that cycle labels without opening overlays.
- Paws visible `R3` toggle and right-thumb command.
- Launch/cleanup and contract updates.
- Tests and documentation.

**Out of scope**

- Color-picker overlay (`hello-lux-color`).
- Light-position overlay (`hello-lux-pos`).
- Persistence of view state.
- Changes to the global hello model.
- C++ implementation.
- Touch input; revisit when touch hardware is available.

## Services

### `hello-service`

Unchanged global model.

### `hello-view-service`

- Required `--display-id`.
- Guard `hello-view-service-display-<id>`.
- Subscribes to global `HelloWorldTopic`.
- Subscribes to `HelloWorldCtlTopic` and filters by display id.
- Maintains per-display state:
  - mode: Auto or Inspect,
  - local attitude quaternion,
  - angular velocity for inertia,
  - light vector,
  - light color,
  - placeholder overlay state (none, Lux Color, Lux Pos) for menu label cycling.
- Merges model data and local state, then publishes `HelloWorldViewTopic` with the display id.
- No persistence.

### `hello-ctl-service`

- Required `--display-id`.
- Guard `hello-ctl-service-display-<id>`.
- Subscribes to `HelloWorldInputTopic`, filters by display id.
- Publishes `HelloWorldCtlTopic` for its display.
- Owns no long-term view state.
- Adds the service contract, build/deploy entry, launch entry, and cleanup coverage.

## IDL, topics, and contracts

Add to `DDS/math.idl`:

```idl
struct Vector3 {
  double x;
  double y;
  double z;
};

struct Color {
  double r;
  double g;
  double b;
};
```

Add to `DDS/hello_world.idl`:

```idl
struct HelloWorldInputTopic {
  unsigned long display_id;
  long action;
  double x;
  double y;
  double z;
  boolean pressed;
};

struct HelloWorldCtlTopic {
  unsigned long display_id;
  long action;
  double x;
  double y;
  double z;
  boolean pressed;
};
```

Extend `HelloWorldViewTopic` with:

- `display_id`
- `mode`
- `arcball`
- `light` (`math::Vector3`)
- `color` (`math::Color`)
- placeholder overlay state (`none`, Lux Color, Lux Pos)

`HelloWorldTopic` stays unchanged.

Add shared action/mode constants, either in a small `hello-control` library or a dedicated module in `format-list`:

- input actions: grab, drag, release, toggle arcball, mode toggle, and Lux Color/Lux Pos placeholder toggles;
- modes: Auto, Inspect;
- overlays: none, Lux Color, Lux Pos;
- menu item IDs for `Formats`, `Inspect/Auto`, `Lux Color/World`, and `Lux Pos/World`.

Contracts:

- `contracts/services/hello-ctl-service.toml`: subscribe `HelloWorldInputTopic`, publish `HelloWorldCtlTopic`.
- `contracts/services/hello-view-service.toml`: subscribe `HelloWorldCtlTopic`.
- Register the new topics in `contracts/topics.toml` with reliable QoS.
- Update `stations-dds::contract::known_types()`.
- Regenerate `build/topics.md`.

## Display library ABI v4

Update `include/display_lib.h`:

```c
#define DISPLAY_LIB_ABI_VERSION 4u

void display_lib_mouse_motion(display_lib* d, int x, int y);
void display_lib_mouse_wheel(display_lib* d, int delta);
```

- `lil-d` loads and calls the new functions.
- `lil-d` forwards `CursorMoved` and `MouseWheel`.
- `lil-d` forwards all mouse buttons, not just left.
- The display library tracks the pointer position and pressed buttons.
- Motion and wheel are routed to the active format through the format manager.
- Existing ABI v3 behavior remains otherwise unchanged; pointer input is mouse-only in this increment.

## Display library input and rendering

- Extend `Format` with mouse-motion and mouse-wheel handlers, defaulting to ignored.
- Extend `FormatCommand` with a right-thumb toggle command.
- The hello-world format:
  - receives local mouse motion/wheel and format commands;
  - publishes `HelloWorldInputTopic` through a format-specific input handle stamped with the display id;
  - maps:
    - left-drag -> enter Inspect and rotate,
    - release with velocity -> inertia,
    - right-thumb toggle -> arcball toggle,
    - right-stick movement while arcball -> rotate,
    - menu `Inspect/Auto` -> mode toggle,
    - menu `Lux Color`/`World` -> placeholder overlay toggle,
    - menu `Lux Pos`/`World` -> placeholder overlay toggle,
    - `Formats` -> existing format chooser selection.
- `ViewSource` filters `HelloWorldViewTopic` by display id.
- Extend the view state with mode, overlay, light, and color.
- Earth rendering applies the light vector and light color.
- Hide the greeting in Inspect mode.
- No Lux Color or Lux Pos overlay is rendered in this story; only the menu labels cycle.

## Horizontal touch menu

Replace the vertical menu layout with a horizontal top-band item bar:

- The three-dot button and the open menu occupy the same top band at the format window's left edge.
- The button is `56 * ui_scale` high, larger than before for a touchscreen look.
- Opening the menu hides the three-dot button; the former button area is the first menu item.
- The menu starts at `region.x`, `region.y`; the first slot is flush left with no outer left padding.
- Items use equal, fixed-width slots, centered labels, and tight consistent gaps.
- Slot width is budgeted from the widest planned label (`Lux Color`) at the 720x720 reference size.
- There is no wrap, shrink-to-fit, or overflow fallback. If planned labels or buttons no longer fit, replan the menu.
- Large touch targets; no hover dependency.
- `Formats` is always first.
- Story A items:
  1. `Formats`
  2. `Inspect` / `Auto`
  3. `Lux Color` / `World`
  4. `Lux Pos` / `World`
- `Inspect`/`Auto`, `Lux Color`/`World`, and the Lux mutual exclusion come from the per-display view state.
- The Lux items are selectable placeholders: selecting one cycles its label and opens no overlay.
- D-pad left/right cycles through all four items.
- A/right-click (controller) or the virtual cursor activates the selected item; local mouse left-click activates items, and local right-click does nothing.
- The Xbox Start button toggles the menu open/closed; B cancels/closes it.
- The menu is hidden while the chooser is active.
- Selecting another item while an overlay is open will cancel the current overlay first (used by Stories B/C).

## Arcball and inertia

- Trackball/arcball mapping from pointer position to sphere point.
- Rotation between consecutive points is applied to the local attitude.
- Left-drag enters Inspect.
- Release velocity becomes angular velocity.
- A friction constant damps the rotation over time.
- Auto mode ignores local attitude and uses the model attitude.
- Inspect mode uses local attitude and hides the greeting.
- The view service integrates inertia at its relay rate.
- Tests cover mapping, composition, damping, and mode transitions.

## Paws

- Add a local, clearly visible custom `R3` checkbox-style toggle near the right stick.
- The toggle shows `Off`/`On`; do not rely on the standard `CheckBox` widget, which was not discoverable.
- Default unchecked.
- Clicking/tapping the toggle changes it.
- Physical right-stick press toggles it.
- Every toggle sends the same right-thumb command.
- Paws does not subscribe to hello-world view state and does not know about formats.
- The toggle may not reflect menu/mouse mode changes; that is accepted.
- Add right-thumb press detection to the gamepad `Controls`.
- Send a menu-toggle command on the Start press edge and a menu-cancel command on the B press edge.
- Keep the existing right-stick cursor commands; the display library interprets them as arcball rotation when hello-world is in arcball mode.

## Tests

Automated:

- Per-display input/control/view filtering.
- Auto/Inspect merge and greeting visibility.
- Arcball mapping, inertia, and friction decay.
- Four-item horizontal menu: left alignment, equal slots, labels, no button while open, and fit at 720x720.
- Local left-click routing for menu/chooser and local right-click no-op.
- Cursor-source selection and native-cursor hiding over the format region.
- Lux Color/World and Lux Pos/World placeholder cycling and mutual exclusion.
- Display-library pointer position and pressed-button tracking.
- ABI v4 loading and event routing.
- Right-thumb toggle from paws and the R3 toggle.
- Start/B menu toggle and cancel commands from paws.
- Contract, registry, and known-type completeness.
- `dds-echo` decode/send for new topics.

Manual, using a temporary `LOCALAPPDATA`:

1. Launch the fleet with two display ids.
2. Left-drag the globe on display 0; verify it enters Inspect and rotates.
3. Right-click the globe and menu button; verify nothing happens.
4. Release quickly; verify inertia and damping.
5. Open the menu with left-click; verify four items, left alignment, tight equal spacing, and that the three-dot button is hidden.
6. Toggle `Inspect`/`Auto` from the menu.
7. Select `Lux Color`; reopen the menu and verify it reads `World`. Repeat for `Lux Pos`, and verify only one placeholder overlay is active.
8. Toggle the paws `R3` control by clicking it and, if a controller is available, by physical right-stick press; verify the same arcball state on display 0.
9. Verify the native arrow is hidden over the format region and one crosshair follows the mouse; use the controller and verify the same crosshair moves instead.
10. Verify display 1 is unaffected.
11. Confirm the greeting is hidden in Inspect and restored in Auto.
12. Verify light color/position defaults are visible per display.
13. Verify no hello-view state file is written.
14. Verify `cleanup.ps1` stops both per-display services.

Touch input is out of scope for this increment; the menu keeps its touch-friendly sizing.

## Acceptance

- Per-display `hello-view-service` and `hello-ctl-service` start and filter correctly.
- Local mouse input reaches the display library through ABI v4 and then the per-display controller.
- Paws right-thumb/R3 toggle input reaches the same path.
- Start toggles the menu and B cancels it.
- Auto/Inspect mode, local arcball, inertia, and light/color work per display.
- The menu is horizontal, touch-friendly, left-aligned in the top band, has four items, and hides the three-dot button while open.
- Lux Color/World and Lux Pos/World cycle as selectable placeholders without rendering overlays.
- No hello-view state is persisted.
- The global model remains unchanged.
- Contracts, registry, `dds-echo`, and `build/topics.md` stay consistent.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-rust.ps1
.\scripts\cleanup.ps1
```

One manual two-display run; no screenshots and no repeated UI runs.

## Risks

- **Per-display refactor:** view-service filtering and launch/cleanup changes affect every display.
- **ABI v4:** header, Rust library, and `lil-d` must change together; the C++ mirror is deferred.
- **Menu budget:** four fixed slots must fit at 720x720; adding labels or items later requires replanning, not wrapping.
- **Menu hit-testing:** when open, the former three-dot area is the first item, not a close toggle.
- **Placeholder overlays:** Lux labels cycle, but no overlay is rendered; this must not be mistaken for a finished overlay.
- **Input ownership:** the display library must publish input with the correct display id.
- **Arcball math:** trackball mapping, inertia, and friction decay need focused tests.
- **Touch:** touch input is deferred until hardware is available; the menu keeps its touch-friendly layout.
- **Cursor hiding:** native cursor visibility is region-based; console controls must keep the native arrow.
- **Local buttons:** only the left button acts locally; controller A/right-click is intentionally separate.
- **Paws toggle:** it is intentionally local-only and may not mirror menu/mouse mode changes.
- **Start/B:** Start and B act on the menu before the active format; B only cancels an open menu.
- **No persistence:** restart resets mode, attitude, light, color, and placeholder overlay state by design.
- **Test isolation:** use a temporary `LOCALAPPDATA`.

## Files touched

- `DDS/math.idl`
- `DDS/hello_world.idl`
- `contracts/topics.toml`
- `contracts/services/hello-ctl-service.toml` (new)
- `contracts/services/hello-view-service.toml`
- `include/display_lib.h`
- `rust/lib/format-list/src/lib.rs` (or new control constants module)
- `rust/lib/stations-dds/src/contract.rs`
- `rust/lib/stations-dds/src/lib.rs`
- `rust/lib/stations-dds/tests/registry.rs`
- `rust/service/hello-ctl-service/` (new)
- `rust/service/hello-view-service/src/main.rs`
- `rust/service/hello-view-service/src/relay.rs`
- `rust/lib/display-lib/src/lib.rs`
- `rust/lib/display-lib/src/view.rs`
- `rust/lib/display-lib/src/earth.rs`
- `rust/lib/display-lib/src/formats.rs`
- `rust/lib/display-lib/src/format_manager.rs`
- `rust/lib/display-lib/src/format_source.rs`
- `rust/lib/display-lib/src/format_menu.rs`
- `rust/lib/display-lib/src/formats/hello_world.rs`
- `rust/ui/lil-d/src/app.rs`
- `rust/ui/lil-d/src/display.rs`
- `rust/ui/paws/src/commands.rs`
- `rust/ui/paws/src/controls.rs`
- `rust/ui/paws/ui/paws.slint`
- `rust/tool/dds-echo/src/decode.rs`
- `rust/tool/dds-echo/src/send.rs`
- `scripts/build.ps1`
- `scripts/launch-rust.ps1`
- `scripts/cleanup.ps1`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `paws-format-ctl` was completed at `2026-11-16 15:47:43` and moved to `completed_stories.md` under "Later increments".
- `hello-ctl-service` is `in progress` in `backlog.md`.
- `hello-lux-color` is `todo`, depending on `hello-ctl-service`.
- `hello-lux-pos` is `todo`, depending on `hello-lux-color`.
- Do not move `hello-ctl-service` to `completed_stories.md` until the user asks to plan the next story.
