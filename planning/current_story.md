# Current story: format-chooser-format

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the format-chooser format: a normal format that presents the other available formats as a 4x4 grid of icon-and-title tiles and switches to the chosen one.

- **Depends on:** none.
- **Minimal test:** open the three-dot menu, choose `Formats`, choose a format tile, and verify the display switches to it.

## Context

- Formats are renderers implementing `Format` in `rust/lib/display-lib/src/formats.rs`.
- The format manager handles the three-dot menu and forwards pointer input.
- There is no active-format input API or way to request a format selection.
- `contracts/formats.toml` is the shared ordered list used by the menu, controller, view service, and display library.
- Selecting a format already works end-to-end through the existing `Select` path and persistence.
- The display library currently generates `Next` for clicks in the format window; that behavior is removed in this story.

## Decisions

1. **The chooser is a normal format**, named `format-chooser`, added last in `contracts/formats.toml`.
2. **It lists the other formats**, excluding itself, in `formats.toml` order.
3. **It renders a 4x4 grid** of icon-and-title tiles, with **16 formats per page**. There is no chooser title.
4. **A bottom pager strip is reserved** for future next/previous controls. It is empty and inert in this story. If the grid should use the full area until pagination exists, reserve the strip only when the catalog exceeds 16.
5. **Clicking a tile publishes `Select`** for that format; clicking anything else inside the chooser is consumed.
6. **The format manager delegates input to the active format after menu handling.** Ignored and consumed input produce no action. There is no local `Next`.
7. **No new DDS topics, services, or ABI functions.** The existing `Select` path and persistence are reused.
8. **Extract shared 2D helpers** (`Rect`, clear-to-color, size scale) into a small `ui` module used by both `format_menu` and `format_chooser`.
9. **Each non-chooser format has a title and a 64x64 PNG icon.** Icons are monochromatic CRT green. `hello-world` gets a globe icon with North and South America outlines and latitude/longitude lines; its title is `Hello World`. `default` gets a square with an X through it; its title is `Default`.
10. **The three-dot menu selects the format chooser.** It has a single item, `Formats`.
11. **The three-dot menu is not drawn or active while the chooser is active.**
12. **The menu has no current-selection highlight.** Any future highlight indicates the default format, not the active one.
13. **`Next` remains supported by `format-list` and `format-ctl-service`** for external publishers, but the display library no longer generates it.

## Format metadata and icons

`rust/lib/display-lib/src/formats.rs`:

```rust
pub(crate) struct FormatMeta {
    pub(crate) name: String,
    pub(crate) title: &'static str,
    pub(crate) icon_file: &'static str,
}

pub(crate) enum FormatInput {
    Ignored,
    Consumed,
    Select(String),
}

pub(crate) trait Format {
    fn name(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn icon_file(&self) -> &'static str;
    fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    );

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

Registry construction:

1. Iterate `formats.names()`.
2. Build every non-chooser format and collect `FormatMeta` in contract order.
3. Build `FormatChooser` with that catalog.
4. Return all formats in original contract order.

Per-format modules:

- `formats/default.rs`: title `Default`, icon file `icons/default.png`.
- `formats/hello_world.rs`: title `Hello World`, icon file `icons/hello-world.png`.
- `formats/format_chooser.rs`: no meaningful icon or title; it is excluded from its own catalog.

New `rust/lib/display-lib/src/icon.rs`:

- `Icon { texture, width, height }`.
- `Icon::load(gl, path)` uses `image::open(path).to_rgba8()`, uploads as `RGBA8`, with `LINEAR` filtering and `CLAMP_TO_EDGE`.
- `IconRenderer` is a textured-quad shader; blend with `SRC_ALPHA, ONE_MINUS_SRC_ALPHA`.
- Icons draw at `64 * ui::scale_for_height(region_height)`, capped to the tile icon area.
- `fit_rect`/`icon_rect` is a pure helper for aspect-preserving placement and tests.
- PNGs are drawn as authored, so they carry the CRT-green color and transparency.

Assets:

```text
assets/icons/hello-world.png   64x64 RGBA
assets/icons/default.png       64x64 RGBA
```

`scripts/build.ps1` already copies `assets/` recursively, so no build-script change is needed.

## Shared `ui` module

New `rust/lib/display-lib/src/ui.rs`:

- `Rect { x, y, w, h }` with `contains`.
- `clear_rect(gl, framebuffer_height, rect, color)` using scissor + color clear.
- `scale_for_height(height) -> f32` using the 720 px reference and clamped range.

Update `format_menu.rs` and `format_chooser.rs` to use these helpers instead of local copies.

## Active-format input API

- `FormatInput` has `Ignored`, `Consumed`, and `Select(String)`.
- `Format` gains `title`, `icon_file`, and a default `mouse_button` that returns `Ignored`.
- Formats no longer click to `Next`. This behavior is removed in this story. The three-dot menu and format chooser are the only ways to choose a format.

## Format manager routing

`rust/lib/display-lib/src/format_manager.rs`:

1. Add `active_format_name(window) -> Option<String>` used by both render and input.
2. Render the active format.
3. If the active format is not `format-chooser`, render the menu; otherwise skip it and close any open menu state for that window.
4. On input:
   - if the active format is not the chooser, let the menu handle the click first;
   - if the menu ignores it or is hidden, and the point is inside the format region, call the active format's `mouse_button`;
   - map `Ignored` and `Consumed` to no action, and `Select(name)` to `FormatAction::Select`;
   - if no active format exists, do nothing.
5. Delete `FormatAction::Next` and the `Next` fallback.
6. Delete `FormatSource::send_next`.
7. Keep `FORMAT_COMMAND_NEXT` and `format-ctl-service` handling unchanged so external publishers can still send `Next`.

## Single-item format menu

`rust/lib/display-lib/src/format_menu.rs`:

- `FormatMenu::new(gl, assets_root)` no longer takes the format list.
- One fixed row: label `Formats`, target `format-chooser`.
- `MenuLayout::new(region, 1)`.
- A row click returns `Select("format-chooser")` and closes the menu.
- Remove active-format highlighting and the `ACTIVE_BG` path.
- Add `FormatMenu::close(window)` so entering the chooser clears stale open state.
- Keep outside-click closing and open/closed state.

## Format chooser

New `rust/lib/display-lib/src/formats/format_chooser.rs`.

### Construction

```rust
FormatChooser::new(gl, assets_root, catalog: &[FormatMeta])
```

- Filter out `format-chooser` from the catalog.
- Load one 64x64 icon texture per entry.
- Keep entries `(name, title, Icon)` in contract order.

### Layout

Constants: `GRID_COLUMNS = 4`, `GRID_ROWS = 4`, `PAGE_CAPACITY = 16`.

- Scale margins, gaps, and pager height with `ui::scale_for_height`.
- Reserve a bottom pager strip: height `48 * scale`, gap `8 * scale`.
- The grid fills the remaining content area with uniform cell sizes.
- Render only the first `min(catalog.len(), 16)` entries, row-major.
- Each tile has a dark background, a square icon area, and a centered title below.
- `ChooserLayout` exposes the tiles and the pager rectangle; render and hit-testing share it.

### Render

1. Clear the whole region to the chooser background.
2. Set the region viewport and disable depth testing.
3. Draw tile backgrounds with `ui::clear_rect`.
4. Draw icons with `IconRenderer`.
5. Draw titles centered with `TextRenderer`.
6. Do not draw a chooser title.
7. The menu is not drawn because the chooser is active.

### Input

- A left press on a used tile returns `Select(name)`.
- Any other press inside the region, including empty grid cells and the pager strip, returns `Consumed`.
- There is no pagination behavior yet.

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
- Add the `format-chooser` registry arm and pass the collected catalog.

The menu, controller, and view service pick up the new name from the contract. Persistence needs no code change.

## Tests

Automated:

- `ui::Rect::contains` boundary tests.
- `icon::fit_rect` preserves aspect and stays inside the tile.
- PNG assets exist, are 64x64, and have an alpha channel.
- Chooser catalog excludes `format-chooser` and preserves contract order.
- Chooser layout produces 16 cells in 4x4 order and stays inside the content area above the pager strip.
- Two entries occupy only the first two cells.
- A used-tile click returns the correct `Select`; an empty cell, the pager, and the background return `Consumed`.
- The menu has one row: `Formats` returns `Select("format-chooser")`.
- The menu is not rendered or consulted when the chooser is active.
- `FormatManager` delegates:
  - menu `Consumed` -> no action;
  - menu `Select` -> `Select`;
  - active format `Ignored` -> no action;
  - active format `Consumed` -> no action;
  - active format `Select` -> `Select`.
- Existing menu/text/format-manager/registry tests still pass.

Manual:

1. Launch the fleet.
2. Open the three-dot menu; verify it has only `Formats`.
3. Choose `Formats`; verify the chooser shows a 4x4 grid with `Hello World` and `Default` tiles and icons, and no chooser title.
4. Verify the three-dot menu is not visible or clickable while the chooser is active.
5. Choose `Default`; verify the display switches and `dds-echo` shows the `Select` input and resulting control/view samples.
6. Click the background of `Default` and `Hello World`; verify nothing changes.
7. Reopen the menu from a non-chooser format and verify it still works.
8. Repeat on the second display; verify only that display changes.
9. Restart `format-view-service`; verify the chosen format is restored.

## Acceptance

- `format-chooser` appears in `contracts/formats.toml`, is the menu target, and is in the registry.
- The chooser lists every other format in contract order and excludes itself.
- It renders a 4x4 grid of 64x64 CRT-green icon tiles with `Hello World` and `Default` titles.
- Clicking a tile switches that display to the chosen format.
- Background clicks do not advance or change the format.
- The three-dot menu has one `Formats` row and is hidden while the chooser is active.
- No local `Next` is generated; external `Next` still works through the controller.
- The chosen format persists through the existing view-service state file.
- No ABI, IDL, topic, or service-contract changes are needed.

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

- **Icon assets:** two 64x64 PNGs must be created; a different size or missing alpha affects loading or appearance.
- **Upscaling:** 64x64 icons are scaled by display height; linear filtering is required, and very large displays may look soft.
- **Grid density:** 4x4 is fixed; fewer formats leave empty cells intentionally, and more than 16 requires the future paging story.
- **Pager reservation:** the bottom strip is inert; it must never be hit-tested as a tile.
- **Menu visibility:** entering the chooser through DDS must clear any open menu state, or it may reappear later.
- **Borrowing:** clone the active format name before mutably borrowing the format list.
- **GL state:** restore viewport/blend/program state after icons and text.
- **Text fit:** use `text_width`/`font_size_for` so `Hello World` fits its tile.
- **Persistence:** the view service already validates against `formats.toml`; adding the chooser requires no code change.
- **Empty catalog:** render an empty grid without panicking.

## Files touched

- `contracts/formats.toml`
- `assets/icons/hello-world.png` (new)
- `assets/icons/default.png` (new)
- `rust/lib/display-lib/src/lib.rs`
- `rust/lib/display-lib/src/ui.rs` (new)
- `rust/lib/display-lib/src/icon.rs` (new)
- `rust/lib/display-lib/src/format_menu.rs`
- `rust/lib/display-lib/src/format_manager.rs`
- `rust/lib/display-lib/src/format_source.rs`
- `rust/lib/display-lib/src/formats.rs`
- `rust/lib/display-lib/src/formats/default.rs`
- `rust/lib/display-lib/src/formats/hello_world.rs`
- `rust/lib/display-lib/src/formats/format_chooser.rs` (new)
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/current_story.md`

## Bookkeeping

- `format-menu` was completed at `2026-11-10 16:55:09` and moved to `completed_stories.md` under "Later increments".
- `format-chooser-format` is `in progress` in `backlog.md`; its dependency on the completed `format-menu` was removed.
- Do not move `format-chooser-format` to `completed_stories.md` until the user asks to plan the next story.
