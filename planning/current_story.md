# Current story: hello-ctl-service

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the per-display hello_world controller and view foundation: per-display `hello-ctl-service` and `hello-view-service`, ABI v4 local input, a horizontal touch-friendly format menu, Auto/Inspect arcball with inertia, per-display light color and position, and paws right-stick toggle input.

The color-picker and light-position overlays are the next two stories (`hello-lux-color` and `hello-lux-pos`).

- **Depends on:** none.
- **Minimal test:** right-drag the globe to enter Inspect and rotate it; release quickly to impart inertia; toggle Auto/Inspect from the horizontal menu and from the paws right-stick toggle; verify a second display is unaffected.

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
6. **Local input path:** display app → ABI v4 → display library → `HelloWorldInputTopic` → per-display `hello-ctl-service` → `HelloWorldCtlTopic` → per-display `hello-view-service` → `HelloWorldViewTopic` → display library filtered by display id.
7. **Paws commands are translated by the display library**, not by paws. Paws does not know formats.
8. **The format menu is a horizontal, touch-friendly item bar**, not vertical rows.
9. **`Formats` is always the first menu item.** Format-specific items follow it.
10. **Story A menu items are `Formats` and `Inspect`/`Auto`.**
11. **`Inspect`/`Auto` shows the next mode**, not the current mode.
12. **Auto** uses the model attitude and shows the greeting. **Inspect** uses the local attitude and hides the greeting.
13. **Arcball:** right-drag on the globe enters Inspect and rotates with trackball mapping; release velocity produces inertia damped by friction.
14. **Paws right-stick press toggles arcball**; while arcball is active, right-stick movement rotates the globe.
15. **Paws gets a local `R3` checkbox.** It defaults unchecked, toggles on physical right-thumb press or click, and sends the same toggle command. It does not subscribe to view state or know about formats.
16. **No format-manager behavior changes** beyond accepting the new right-thumb control command.
17. **`dds-echo` decodes and can send the new hello-world topics.**
18. **The C++ mirror must implement ABI v4** after the Rust increment.

## Scope

**In scope**

- Per-display hello view and control services.
- New hello-world input/control topics and extended view topic.
- ABI v4 mouse motion/wheel and all-button forwarding.
- Per-display view filtering.
- Auto/Inspect mode, local attitude, arcball and inertia.
- Per-display light/color state with defaults.
- Horizontal touch-friendly menu with `Formats` first and `Inspect`/`Auto` second.
- Paws `R3` checkbox and right-thumb command.
- Launch/cleanup and contract updates.
- Tests and documentation.

**Out of scope**

- Color-picker overlay (`hello-lux-color`).
- Light-position overlay (`hello-lux-pos`).
- Persistence of view state.
- Changes to the global hello model.
- C++ implementation.

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
  - pre-overlay state reserved for the next stories.
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
- `light` (`math::Vector3`)
- `color` (`math::Color`)
- overlay/menu state reserved for the next stories

`HelloWorldTopic` stays unchanged.

Add shared action/mode constants, either in a small `hello-control` library or a dedicated module in `format-list`:

- input actions: grab, drag, release, toggle arcball, menu mode toggle, and future overlay actions;
- modes: Auto, Inspect;
- menu item IDs for `Formats`, `Inspect/Auto`, and future format-specific items.

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
- Existing ABI v3 behavior remains otherwise unchanged.

## Display library input and rendering

- Extend `Format` with mouse-motion and mouse-wheel handlers, defaulting to ignored.
- Extend `FormatCommand` with a right-thumb toggle command.
- The hello-world format:
  - receives local mouse motion/wheel and format commands;
  - publishes `HelloWorldInputTopic` through a format-specific input handle stamped with the display id;
  - maps:
    - right-drag → enter Inspect and rotate,
    - release with velocity → inertia,
    - right-thumb toggle → arcball toggle,
    - right-stick movement while arcball → rotate,
    - menu `Inspect/Auto` → mode toggle,
    - `Formats` → existing format chooser selection.
- `ViewSource` filters `HelloWorldViewTopic` by display id.
- Extend the view state with mode, light, and color.
- Earth rendering applies the light vector and light color.
- Hide the greeting in Inspect mode.

## Horizontal touch menu

Replace the vertical menu layout with a horizontal item bar:

- Opens from the three-dot button.
- Large touch targets; no hover dependency.
- Anchored at the button and extending right, or spanning the top of the format window if space is tight.
- Items are laid out left to right.
- `Formats` is always first.
- Story A items:
  1. `Formats`
  2. `Inspect` / `Auto`
- Story B adds `Lux Color` / `World`.
- Story C adds `Lux Pos` / `World`.
- Labels come from the per-display view state.
- D-pad left/right selects items.
- A/right-click, touch, or the virtual cursor activates the selected item.
- The menu is hidden while the chooser is active.
- Selecting another item while an overlay is open will cancel the current overlay first (used by Stories B/C).

## Arcball and inertia

- Trackball/arcball mapping from pointer position to sphere point.
- Rotation between consecutive points is applied to the local attitude.
- Right-drag enters Inspect.
- Release velocity becomes angular velocity.
- A friction constant damps the rotation over time.
- Auto mode ignores local attitude and uses the model attitude.
- Inspect mode uses local attitude and hides the greeting.
- The view service integrates inertia at its relay rate.
- Tests cover mapping, composition, damping, and mode transitions.

## Paws

- Add a local `R3` checkbox to the paws UI.
- Default unchecked.
- Clicking the checkbox toggles it.
- Physical right-stick press toggles it.
- Every toggle sends the same right-thumb command.
- Paws does not subscribe to hello-world view state and does not know about formats.
- The checkbox may not reflect menu/mouse mode changes; that is accepted.
- Add right-thumb press detection to the gamepad `Controls`.
- Keep the existing right-stick cursor commands; the display library interprets them as arcball rotation when hello-world is in arcball mode.

## Tests

Automated:

- Per-display input/control/view filtering.
- Auto/Inspect merge and greeting visibility.
- Arcball mapping, inertia, and friction.
- Horizontal menu layout, labels, and activation.
- ABI v4 loading and event routing.
- Right-thumb toggle from paws and the checkbox.
- Contract, registry, and known-type completeness.
- `dds-echo` decode/send for new topics.

Manual, using a temporary `LOCALAPPDATA`:

1. Launch the fleet with two display ids.
2. Right-drag the globe on display 0; verify it enters Inspect and rotates.
3. Release quickly; verify inertia and damping.
4. Toggle `Inspect`/`Auto` from the horizontal menu.
5. Toggle the paws `R3` checkbox and the physical right-stick press; verify the same arcball state on display 0.
6. Verify display 1 is unaffected.
7. Confirm the greeting is hidden in Inspect and restored in Auto.
8. Verify light color/position defaults are visible per display.
9. Verify no hello-view state file is written.
10. Verify `cleanup.ps1` stops both per-display services.

## Acceptance

- Per-display `hello-view-service` and `hello-ctl-service` start and filter correctly.
- Local input reaches the display library through ABI v4 and then the per-display controller.
- Paws right-thumb/checkbox input reaches the same path.
- Auto/Inspect mode, local arcball, inertia, and light/color work per display.
- The menu is horizontal, touch-friendly, and has `Formats` first.
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
- **ABI v4:** header, Rust library, `lil-d`, and the future C++ mirror must change together.
- **Menu rewrite:** horizontal touch layout and dynamic labels touch existing menu tests.
- **Input ownership:** the display library must publish input with the correct display id.
- **Arcball math:** trackball mapping and inertia need focused tests.
- **Paws checkbox:** it is intentionally local-only and may not mirror menu/mouse mode changes.
- **No persistence:** restart resets mode, attitude, light, and color by design.
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
