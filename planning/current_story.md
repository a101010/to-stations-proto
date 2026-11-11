# Current story: format-chooser-format

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the format-chooser format: a normal display format that presents the other available formats and switches to the chosen one.

- **Depends on:** none.
- **Minimal test:** open the format chooser, choose a format, and verify the display switches to it.

## Context

- The display libraries have a three-dot menu over every format.
- Formats are renderers implementing `Format` in `rust/lib/display-lib/src/formats.rs`.
- The format manager handles menu input and publishes `Next` for clicks inside the format window.
- There is no way for an active format to receive pointer input or request a format selection.
- `contracts/formats.toml` is the shared ordered list used by the menu, controller, and view service.
- Selecting a format already works end to end: the display library publishes `Select`, the controller applies it, and the view service persists it.

## Decisions

1. **The chooser is a normal format**, named `format-chooser`, added last in `contracts/formats.toml`.
2. **It lists the other formats**, excluding itself, in `formats.toml` order.
3. **It renders a title and a grid of large format tiles.**
4. **Clicking a tile publishes `Select`** for that format; clicking the background is consumed so it does not fall through to `Next`.
5. **The format manager delegates input to the active format after handling the three-dot menu.**
6. **No new DDS topics, services, or ABI functions.** The existing `Select` path and format persistence are reused.
7. **Extract shared 2D helpers** (`Rect`, clear-to-color, size scale) into a small `ui` module used by both `format_menu` and `format_chooser`.

## New active-format input API

In `rust/lib/display-lib/src/formats.rs`:

```rust
pub(crate) enum FormatInput {
    Ignored,
    Consumed,
    Select(String),
}

pub(crate) trait Format {
    fn name(&self) -> &'static str;
    fn render(...);

    fn mouse_button(
        &mut self,
        region: (i32, i32, i32, i32),
        x: i32,
        y: i32,
    ) -> FormatInput {
        let _ = (region, x, y);
        FormatInput::Ignored
    }
}
```

Existing formats keep the default `Ignored` behavior, so click-to-`Next` is unchanged for them.

## Format manager routing

`rust/lib/display-lib/src/format_manager.rs`:

1. Menu handling runs first, exactly as now.
2. If the menu ignores the click and the point is inside the format window:
   - find the active format for that window;
   - call `active_format.mouse_button(region, x, y)`;
   - map the result:
     - `Ignored` → existing `FormatAction::Next`
     - `Consumed` → no DDS action
     - `Select(name)` → `FormatAction::Select`
3. If no active format is found, retain the existing `Next` fallback.

Add a small active-format-name helper used by both `render` and `mouse_button`.

## Shared `ui` module

New `rust/lib/display-lib/src/ui.rs`:

- `Rect { x, y, w, h }` with `contains`.
- `clear_rect(gl, framebuffer_height, rect, color)` using scissor + color clear.
- `scale_for_height(height) -> f32` using the 720 px reference and clamped range.

Update `format_menu.rs` to use these helpers instead of its local copies. This keeps the chooser and menu geometry consistent.

## Format chooser

New `rust/lib/display-lib/src/formats/format_chooser.rs`.

### Construction

```rust
FormatChooser::new(gl, assets_root, all_format_names)
```

- Filter out `"format-chooser"` from the choices.
- Load the shared `TextRenderer` font (`text::FONT_FILE`).

### Layout

Pure `ChooserLayout` based on the region and choice count:

- Reserve the upper-left menu-button area; start the chooser content below it.
- Title: `Choose a format`.
- Grid:
  - 1 choice: one tile.
  - 2 choices: two columns.
  - 3+ choices: two columns, wrapping into rows.
- Tiles fill the available content area with margins and gaps; clamp to the format region.
- Tile labels are the format names, centered.
- Layout and hit-testing share the same pure layout function.

### Render

- Clear the whole region to the chooser background.
- Set the region viewport and disable depth testing.
- Draw the title.
- Draw each tile background with `ui::clear_rect`.
- Draw each tile label with `TextRenderer::draw_at`.
- The normal three-dot menu is drawn after the chooser by `FormatManager`.

### Input

- A left press on a tile returns `FormatInput::Select(name)`.
- A left press anywhere else inside the chooser returns `FormatInput::Consumed`.
- The menu still takes precedence because `FormatManager` handles it first.

## Registry and contracts

`contracts/formats.toml`:

```toml
[[formats]]
name = "hello-world"

[[formats]]
name = "default"

[[formats]]
name = "format-chooser"
```

`rust/lib/display-lib/src/formats.rs`:

- Add `pub(crate) mod format_chooser;`.
- Add registry arm:
  - `"format-chooser" => Box::new(FormatChooser::new(gl, assets_root, formats.names())?)`.

The menu and view-service validation pick up the new format automatically.

## Tests

Automated:

- `ui::Rect::contains` boundary tests.
- Chooser choices exclude `format-chooser` and preserve contract order.
- Chooser tile layout stays inside the region.
- Tile click returns `Select` with the correct format name.
- Background click returns `Consumed`.
- `FormatManager` delegates:
  - active format `Ignored` → `Next`;
  - active format `Consumed` → no action;
  - active format `Select` → `Select` action.
- Existing menu/text/format-manager tests still pass.

Manual:

1. Launch the fleet.
2. Open the three-dot menu; verify `format-chooser` is listed.
3. Select `format-chooser`; verify the chooser appears with tiles for `hello-world` and `default`.
4. Click `default`; verify the display switches and `dds-echo` shows the `Select` input and resulting control/view samples.
5. Open the menu while the chooser is active; verify the menu still works.
6. Click the chooser background; verify it does not advance to `Next`.
7. Repeat on the second display; verify only that display changes.
8. Restart `format-view-service`; verify the chosen format is restored.

## Acceptance

- `format-chooser` appears in `contracts/formats.toml` and the format menu.
- The chooser lists every other format in contract order and excludes itself.
- Clicking a tile switches that display to the chosen format.
- Background clicks do not fall through to `Next`.
- The menu still takes precedence over the chooser’s input.
- No DDS, service, or ABI changes are needed.
- The chosen format persists through the existing view-service state file.

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

- **Input ordering:** menu input must run before active-format input.
- **Layout drift:** render and hit-test must use the same `ChooserLayout`.
- **Menu overlap:** reserve the upper-left menu-button area in the chooser layout.
- **Self-selection:** filter `format-chooser` from its own choices.
- **Empty choices:** handle a format list containing only the chooser without panicking.
- **GL state:** the chooser clears the whole region and must set its viewport/depth state before text.
- **Borrowing:** `FormatManager::mouse_button` must clone the active name before mutably borrowing the format list.
- **Persistence:** the view service must accept the new format name from `formats.toml`; no code change is expected.

## Files touched

- `contracts/formats.toml`
- `rust/lib/display-lib/src/lib.rs`
- `rust/lib/display-lib/src/ui.rs` (new)
- `rust/lib/display-lib/src/format_menu.rs`
- `rust/lib/display-lib/src/format_manager.rs`
- `rust/lib/display-lib/src/formats.rs`
- `rust/lib/display-lib/src/formats/format_chooser.rs` (new)
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `format-menu` was completed at `2026-11-10 16:55:09` and moved to `completed_stories.md` under "Later increments".
- `format-chooser-format` is `in progress` in `backlog.md`; its dependency on the completed `format-menu` was removed.
- Do not move `format-chooser-format` to `completed_stories.md` until the user asks to plan the next story.
