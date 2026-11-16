# Current story: paws-format-ctl

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add DDS connectivity from `paws` to the per-display format manager: high-level cursor, navigation, and pointer commands go from `paws` to `format-ctl-service`, then to the display library. The right stick moves a virtual cursor, A/right-click opens the three-dot menu, D-pad selects the `Formats` row and wraps the chooser selection, and A activates the highlighted format.

- **Depends on:** none.
- **Minimal test:** launch paws with a display id; move the cursor to the three-dot button, open the menu and `Formats` with A, verify D-pad wraps the chooser selection with the first format selected by default, A selects the highlighted format, and `dds-echo` shows the input and command-control samples.

## Context

- `FormatManagerInputTopic` is the controller's input topic. The display library publishes `Select`; external publishers can send `Next`/`Select`.
- `FormatManagerCtlTopic` carries persisted format assignments to the view service.
- The display library has no high-level command path and no virtual cursor.
- The format chooser has no selected tile or highlight; it only responds to left clicks.
- `paws` has no DDS connectivity yet.
- `format-ctl-service` currently consumes only `Next`/`Select`.
- `paws` already has the display id and both virtual and real controller controls.

## Decisions

1. **Reuse `FormatManagerInputTopic`** for paws commands and add one output topic, `FormatManagerCommandCtlTopic`, from the controller to the display library.
2. **A/right-click activates the current target.** It activates the selected menu row when the menu is open, and the highlighted chooser tile when the chooser is active.
3. **Controller access to the chooser is through the three-dot menu.** Move the virtual cursor to the three-dot button and press A/right-click to open the menu; the `Formats` row is selected; press A again to open the chooser.
4. **The cursor is shown over the active format** after the first right-stick movement and is used to aim at the three-dot button. It is independent of the chooser tile highlight.
5. **D-pad navigation wraps spatially** within occupied tiles of the current row or column. It does not clamp.
6. **Cursor moves are pixel deltas** sent at about 30 Hz; D-pad and A are edge-triggered.
7. **All command transport is DDS.** No display-library ABI change.
8. **The view service is unchanged.** Commands are transient and are not persisted.
9. **`dds-echo` decodes and can send the new command-control topic.**
10. **Paws DDS setup is best-effort**: a missing participant, registry, or contract leaves local paws behavior working.

## Scope

**In scope**

- Extended format-manager input payload and new command-control topic.
- `paws` DDS writer, service contract, domain/config options, and command generation.
- Controller relay with display filtering and command validation.
- Display-library command reader, menu routing, and active-format command routing.
- Virtual cursor rendering and clamping.
- Controller-operated three-dot menu with a transient selected row.
- Format chooser selection highlight and spatial D-pad wrapping.
- A/right-click activation of the selected menu row or chooser tile.
- `dds-echo` decode and send support.
- Contracts, IDL, registry, and architecture updates.

**Out of scope**

- Opening the format chooser from the controller.
- Controller operation of the three-dot menu.
- Per-window cursors for `big-d`.
- Pages beyond the chooser's first 16 formats.
- A joystick control service between paws and the format controller.
- Any display-library ABI change.

## Wire protocol

Extend `DDS/format_manager.idl`:

```idl
struct FormatManagerInputTopic {
  unsigned long display_id;
  long window;
  long command;   // 0 Next, 1 Select, 10 CursorMove, 11 Navigate, 12 Pointer
  string format;
  long x;
  long y;
  long button;
  boolean pressed;
};

struct FormatManagerCommandCtlTopic {
  unsigned long display_id;
  long window;
  long command;
  long x;
  long y;
  long button;
  boolean pressed;
};
```

Add constants and command classification to `rust/lib/format-list/src/lib.rs`:

```rust
pub const FORMAT_COMMAND_CURSOR_MOVE: i32 = 10;
pub const FORMAT_COMMAND_NAVIGATE: i32 = 11;
pub const FORMAT_COMMAND_POINTER: i32 = 12;
pub const FORMAT_POINTER_RIGHT: i32 = 1;

pub fn is_control_command(command: i32) -> bool;
```

Semantics:

- `CursorMove`: `x`/`y` are physical-pixel deltas.
- `Navigate`: `x`/`y` are each `-1`, `0`, or `1`; one axis is nonzero.
- `Pointer`: `button` is the mouse-button number (`1` = right) and `pressed` is the state.
- `Next`/`Select` keep their existing meaning; new fields are zero/empty for them.

QoS:

- `FormatManagerInputTopic`: reliable (unchanged).
- `FormatManagerCommandCtlTopic`: reliable, volatile.

## Controller service

`rust/service/format-ctl-service/src/main.rs`:

- Keep the input reader.
- Add a writer for `FormatManagerCommandCtlTopic`.
- In the poll loop, for each input sample addressed to this display id:
  - `Next`/`Select`: apply assignments as today.
  - `CursorMove`/`Navigate`/`Pointer`: forward the command on `FormatManagerCommandCtlTopic`.
  - Unknown commands: ignore.
- Commands for other displays are ignored.
- Commands do not affect assignments or persistence.
- Add tests for forwarding, display filtering, unknown commands, and independence from `Next`/`Select`.

Contracts:

- `contracts/services/format-ctl-service.toml` adds `[[publish]] topic = "FormatManagerCommandCtlTopic"`.
- The input subscription is unchanged.

## Paws

`rust/ui/paws`:

- Add dependencies `stations-dds` and `format-list`.
- Add `--domain` and `--config` options, matching the services. Defaults preserve current launch behavior.
- Resolve contracts through `<exe_dir>/../config`, else `./contracts`.
- Load and validate the topic registry and `contracts/services/paws.toml`.
- Create a `FormatManagerInputTopic` writer. Best-effort: log failures and continue with local-only controls.
- Add `contracts/services/paws.toml`:

```toml
[service]
name = "paws"

[[publish]]
topic = "FormatManagerInputTopic"
```

Command generation (`rust/ui/paws/src/commands.rs`, new):

- Read the current UI controls so Virtual and controller modes behave the same.
- `CommandState` keeps:
  - last cursor send time;
  - previous D-pad pressed states;
  - previous A state.
- Each ~33 ms, if the right stick magnitude is above zero:
  - send `CursorMove` with `dx/dy = round(right * 20)` pixels;
  - skip if both deltas round to zero.
- D-pad:
  - send `Navigate` on each false-to-true edge;
  - `Up/Down/Left/Right` map to `(0,-1)`, `(0,1)`, `(-1,0)`, `(1,0)`.
- A:
  - send a right-button press on the false-to-true edge;
  - send a right-button release on the true-to-false edge.
- All commands carry the paws display id and window 0.
- Unit-test the throttle and edge detection without Slint.

## Display library

### Command API

`rust/lib/display-lib/src/formats.rs`:

```rust
pub(crate) enum FormatCommand {
    CursorMove { dx: i32, dy: i32 },
    Navigate { dx: i32, dy: i32 },
    Pointer { button: i32, pressed: bool },
}

pub(crate) trait Format {
    // existing methods...

    fn command(&mut self, command: FormatCommand) -> FormatInput {
        let _ = command;
        FormatInput::Ignored
    }
}
```

### Format menu

`rust/lib/display-lib/src/format_menu.rs`:

- Add `selected_row: usize` to `MenuState`; reset to `0` when the menu opens.
- Add a `command(...)` method accepting the cursor position and a high-level command.
- Pointer right press:
  - menu closed and cursor over the three-dot button: open the menu, select row 0, consume;
  - menu open: activate the selected row (`Formats`) and close;
  - otherwise: ignored so the active format can handle it.
- Navigate while open: move `selected_row` among rows and consume. With one row this stays on row 0 but is ready for future rows.
- Keep local left-click behavior: the button toggles the menu, a row selects it, and an outside click closes it.
- Render a distinct background for the selected row while open. This is a transient controller selection, not a current-format indicator.
- Add tests for controller open, row activation, navigation, outside fallback, and local click behavior.

### Format manager

`rust/lib/display-lib/src/format_manager.rs`:

- Add per-window cursor state, or a single window-0 cursor for the first increment.
- `command(window, command) -> Option<FormatAction>`:
  - `CursorMove`: initialize at region center on the first command, add the delta, clamp to the region, make the cursor visible;
  - if the active format is not the chooser, route menu commands first:
    - `Pointer` right press opens the menu when the cursor is over the button, or activates the selected row when open;
    - `Navigate` moves the menu selection when open;
    - consumed menu commands do not reach the active format;
  - when the menu is closed, `Navigate` and other pointer commands go to the active format;
  - when the chooser is active, bypass the menu entirely;
  - map `FormatInput::Select` to `FormatAction::Select`.
- Render the cursor after the format and menu:
  - a crosshair made from two `ui::clear_rect` bars;
  - contrasting color;
  - hidden until the first cursor move.
- Add tests for cursor initialization, clamping, menu-first routing, active-format command routing, and Select propagation.

### Format source

`rust/lib/display-lib/src/format_source.rs`:

- Add an optional reader for `FormatManagerCommandCtlTopic`.
- On each command matching this display id:
  - convert it to `FormatCommand`;
  - call `FormatManager::command`;
  - publish any resulting `Select` through the existing input writer.
- A missing command topic must not break assignment handling.
- Add tests for display-id filtering and Select publication.

### Format chooser

`rust/lib/display-lib/src/formats/format_chooser.rs`:

- Add `selected: usize`, default `0`.
- Render a distinct background/border for the selected tile.
- `command`:
  - `Navigate` moves the selection with spatial wrap:
    - left/right cycle through occupied tiles in the current row;
    - up/down cycle through occupied tiles in the current column;
    - a row or column with one occupied tile is a no-op for that axis;
  - right-button press returns `Select(entries[selected].name.clone())`;
  - releases and other buttons are consumed without selection.
- Mouse left-click updates `selected` and selects as today.
- With the current two formats, left/right alternate between the two tiles and up/down are no-ops.
- Add tests for default selection, horizontal/vertical wrap, sparse grids, full-grid edge wrap, activation, and empty catalogs.

## `dds-echo`

`rust/tool/dds-echo`:

- Decode `FormatManagerCommandCtlTopic`.
- Update `FormatManagerInputTopic` tests and field metadata for the new fields.
- Add a `TopicWriter` variant and field-schema arm for the command-control topic.
- Add decode and send tests.

## Contracts, IDL, and build

- `DDS/format_manager.idl`: extend the input struct and add the command-control topic.
- `contracts/topics.toml`: add `FormatManagerCommandCtlTopic` with reliable QoS.
- `contracts/services/format-ctl-service.toml`: publish the command-control topic.
- `contracts/services/paws.toml`: new, publish the input topic.
- `rust/lib/stations-dds/src/contract.rs`: add the new type to `known_types()`.
- `rust/lib/stations-dds/src/lib.rs`: add a type-name assertion.
- `rust/lib/stations-dds/tests/registry.rs`: completeness passes with the new IDL/registry.
- `build/topics.md`: regenerated by `build.ps1`.

## Architecture updates

`planning/architecture.md`:

- Paws now publishes high-level commands on `FormatManagerInputTopic` using its display id.
- `format-ctl-service` relays control commands on `FormatManagerCommandCtlTopic`.
- The display library routes commands to the active format and draws the virtual cursor.
- The chooser highlights a selected tile; D-pad navigates and A/right-click activates.
- `paws` is a control source in the format-manager MVC.
- No ABI changes; commands are DDS-only.

## Tests

Automated:

- `format-list` command classification.
- `FormatChooser`:
  - default selection;
  - horizontal and vertical spatial wrap;
  - sparse-grid no-ops and full-grid edge wrap;
  - right-click activation;
  - mouse selection.
- `FormatMenu`:
  - controller open when the cursor is over the button;
  - selected-row activation selects `format-chooser`;
  - menu navigation with one and multiple rows;
  - outside commands fall through;
  - local click behavior is unchanged.
- `FormatManager`:
  - cursor initialization and clamping;
  - menu-first command routing when not in the chooser;
  - chooser bypasses the menu;
  - active-format command routing;
  - Select propagation.
- `FormatSource`:
  - display-id filtering;
  - command reader optional;
  - Select publication.
- `format-ctl-service`:
  - command forwarding;
  - display filtering;
  - unknown commands ignored;
  - Next/Select independence.
- `paws` command generation:
  - cursor throttle;
  - D-pad press edges;
  - A press/release edges.
- `dds-echo` decode/send.
- `cargo test`, clippy, `build.ps1 -Test`, `build.ps1`.

Manual, using a temporary `LOCALAPPDATA`:

1. Launch the fleet and paws for display 0.
2. Select the format chooser locally.
3. Move the right stick; verify the virtual cursor moves and stays inside the format window.
4. Press D-pad right; verify the highlight moves from `Hello World` to `Default`.
5. Press A; verify the display switches to `Default`.
6. Verify `dds-echo` shows the input and command-control samples.
7. Verify the second display is unaffected.
8. Repeat from Virtual mode with mouse-driven sticks/buttons.
9. Verify no real `%LOCALAPPDATA%\to-stations` state is touched.

## Acceptance

- `paws` publishes high-level commands to `format-ctl-service`.
- The controller relays them on `FormatManagerCommandCtlTopic`.
- The display library draws a virtual cursor and routes commands to the active format.
- The chooser highlights the first format by default and moves the selection with D-pad.
- A/right-click opens the menu when aimed at the three-dot button, activates the selected menu row, and selects the highlighted chooser tile.
- D-pad navigation wraps spatially within rows and columns.
- `Next`/`Select` and format persistence remain intact.
- `dds-echo` decodes and can send the new command-control topic.
- Contracts, registry, and `build/topics.md` stay consistent.
- No display-library ABI change.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-rust.ps1
.\scripts\cleanup.ps1
```

One manual controller pass including opening the menu and chooser from paws; no screenshots and no repeated UI runs.

## Risks

- **Command latency:** the controller relay adds a hop; keep messages small and reliable.
- **Cursor rate:** throttle cursor sends to about 30 Hz to avoid flooding DDS.
- **Window scope:** only window 0 is supported; commands for other windows are ignored.
- **Menu aiming:** opening the menu requires pointing the cursor at the small three-dot button.
- **Menu activation:** A/right-click on an open menu activates the selected row regardless of cursor position.
- **Spatial wrap:** partially filled rows/columns must skip empty cells and no-op when only one occupied tile exists.
- **Cursor semantics:** the cursor is independent of the chooser highlight.
- **Selection persistence:** the chooser keeps its selected tile while it exists.
- **Best-effort paws DDS:** missing contracts or a missing participant must leave local controls working.
- **Input compatibility:** all input-topic publishers must fill the new fields; `Select` sends zeros.
- **Contract changes:** all apps must be rebuilt together because the input topic payload changed.
- **Test isolation:** use a temporary `LOCALAPPDATA`.

## Files touched

- `DDS/format_manager.idl`
- `contracts/topics.toml`
- `contracts/services/format-ctl-service.toml`
- `contracts/services/paws.toml` (new)
- `build/topics.md` (generated)
- `rust/lib/format-list/src/lib.rs`
- `rust/lib/stations-dds/src/contract.rs`
- `rust/lib/stations-dds/src/lib.rs`
- `rust/lib/stations-dds/tests/registry.rs`
- `rust/lib/display-lib/src/formats.rs`
- `rust/lib/display-lib/src/formats/format_chooser.rs`
- `rust/lib/display-lib/src/format_manager.rs`
- `rust/lib/display-lib/src/format_menu.rs`
- `rust/lib/display-lib/src/format_source.rs`
- `rust/service/format-ctl-service/src/main.rs`
- `rust/service/format-ctl-service/Cargo.toml` (only if needed)
- `rust/ui/paws/Cargo.toml`
- `rust/ui/paws/src/main.rs`
- `rust/ui/paws/src/commands.rs` (new)
- `rust/tool/dds-echo/src/decode.rs`
- `rust/tool/dds-echo/src/send.rs`
- `rust/Cargo.lock`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `window-placement` was completed at `2026-11-13 16:10:13` and moved to `completed_stories.md` under "Later increments".
- `paws-format-ctl` is `in progress` in `backlog.md`.
- Do not move `paws-format-ctl` to `completed_stories.md` until the user asks to plan the next story.
