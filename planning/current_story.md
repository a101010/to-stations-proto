# Current story: format-menu

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add a three-dot menu to the format manager, drawn in the upper left over the active format, with one entry per format from `contracts/formats.toml`.

- **Depends on:** none.
- **Minimal test:** open the menu, choose a format, and verify the display switches to it.

## Context

- The display library owns the format manager and all format-manager DDS.
- `FormatManager` renders the active format for window 0 and publishes `Next` on clicks.
- `format-ctl-service` already handles `Select`; no controller change is required.
- `format-view-service` already persists and forwards the resulting assignments.
- `contracts/formats.toml` already provides the ordered format names.
- The display library already has HarfBuzz text rendering and GL scissor/clear, so the menu can be drawn without a new mesh or UI toolkit.

## Decisions

1. **The menu belongs to the display library**, not `lil-d` or the controller.
2. **One menu per format window.** The current single-window driver uses window 0; the state and layout take a window id so `big-d` can reuse them later.
3. **Menu entries are the ordered names from `contracts/formats.toml`**, the same order used to build the format renderers.
4. **Selection publishes a `Select` event** on `FormatManagerInputTopic`; `Next` remains the behavior for clicks outside the menu when it is closed.
5. **The menu button is always visible** in the upper-left of each rendered format window, over every format.
6. **The menu is drawn after the format renderer**, using scissor clears for opaque panel/button/active-row backgrounds and the existing text renderer for labels.
7. **No new DDS topics, ABI functions, or service changes.** The controller and view service already support `Select`.

## Files and modules

- New: `rust/lib/display-lib/src/format_menu.rs`
- `rust/lib/display-lib/src/lib.rs` — add `mod format_menu`, route mouse events.
- `rust/lib/display-lib/src/format_manager.rs` — own the menu, draw it after the active format, expose menu-aware click handling.
- `rust/lib/display-lib/src/format_source.rs` — add `send_select`.
- `rust/lib/display-lib/src/text.rs` — add top-left text drawing and text measurement; share the font path constant.
- `rust/lib/display-lib/src/formats/hello_world.rs` — use the shared font path constant.
- `planning/architecture.md` — document the menu and the `format_menu` module.
- `planning/backlog.md`, `planning/completed_stories.md`, `planning/current_story.md` — bookkeeping.

## Menu behavior

### Layout

Use physical pixels from the format region, scaled from the region height (reference height 720 px, clamped to a reasonable range):

- Button: upper-left inset, approximately 40 × 40 px at reference scale.
- Button background: dark opaque rectangle.
- Button icon: three horizontal dots rendered as the font string `"..."`, centered.
- Panel: below the button, one row per format.
- Row height: approximately 40 px at reference scale.
- Panel width: wide enough for the labels, clamped to the format region.
- Padding: approximately 8–12 px.
- Active format row: lighter background to show the current selection.
- Panel/button backgrounds are drawn with `gl.scissor` + `gl.clear(COLOR_BUFFER_BIT)`.
- No hover state: the display application forwards mouse-button events but not cursor movement.

### Interaction

Left-button press only:

- **Closed menu + button click:** open the menu; consume the click.
- **Open menu + button click:** close the menu; consume the click.
- **Open menu + format row click:** publish `Select(window, format)` and close the menu; consume the click.
- **Open menu + click anywhere else:** close the menu; consume the click so it does not also advance the format.
- **Closed menu + click in the format window but outside the button:** preserve the existing `Next(window)` behavior.
- **Right/middle clicks:** ignored.

The menu is per format window. The active format is taken from the format manager’s assignment for that window, falling back to the first format.

## Rendering and text support

### `text.rs`

Add:

- `text_width(text, font_size_px) -> f32`
- `draw_at(gl, region_width, region_height, text, color, font_size_px, left, top)`

`draw` keeps its current centered behavior for the hello-world greeting. `draw_at` uses the same glyph atlas, shader, and region viewport, but starts the text at a caller-provided top-left point. Factor the vertex-building helper so both centered and left-aligned layout share the glyph positioning code.

Move the font path into a shared constant:

```rust
pub(crate) const FONT_FILE: &str = "fonts/FiraSans-Regular.ttf";
```

Use it from both `hello_world.rs` and `format_menu.rs`.

### `format_menu.rs`

Keep the pure layout/state separate from the GL renderer:

- `MenuNames`: ordered format names.
- `MenuState`: open/closed state and per-window state.
- `MenuLayout`: button rect, panel rect, and row rects computed from the region and names.
- `MenuAction`: `Ignored`, `Consumed`, or `Select(format)`.
- `MenuRenderer`: owns a `TextRenderer`; draws the button, panel, active row, dots, and labels.

Pure layout/hit-test functions must be unit-testable without a GL context.

### `format_manager.rs`

- Add a `format_menu::FormatMenu` field.
- Create it in `FormatManager::new` using the assets root and ordered format names.
- After rendering the active format, render the menu overlay.
- Add a menu-aware click method returning:

```rust
enum FormatAction {
    Next { window: i32 },
    Select { window: i32, format: String },
}
```

The `with_region` test constructor can leave the menu absent so existing tests continue to work.

### `format_source.rs`

- Keep `send_next`.
- Add `send_select(window, format, display_id)` using `FORMAT_COMMAND_SELECT`.
- The existing `Display::mouse_button` routes `FormatAction` to the correct send method.

## Controller and view service

No service changes are required:

- `format-ctl-service::apply` already handles `FORMAT_COMMAND_SELECT` and validates the format name.
- `format-view-service` already persists and republishes accepted assignments.

## Tests

Automated:

- Menu layout: button, panel, and row rects stay inside the format region.
- Button click toggles open/closed.
- Row click returns `Select` with the correct format name and closes the menu.
- Click outside an open menu closes it and is consumed.
- Click in the format window when the menu is closed still returns `Next`.
- Right/middle clicks are ignored.
- Menu names preserve `formats.toml` order.
- Text layout helper places text at the requested top-left origin and `text_width` scales with font size.
- Existing format manager assignment tests still pass.

Manual:

1. Launch the fleet.
2. Verify the three-dot button is visible in the upper-left over both `hello-world` and `default`.
3. Open the menu; verify both format entries appear in contract order.
4. Select `default`; verify the display changes, the menu closes, and `dds-echo` shows a `Select` input event with the correct display id.
5. Select `hello-world` the same way.
6. Click outside the menu while it is open; verify it closes without changing the format.
7. With the menu closed, click elsewhere in the format window; verify the existing `Next` behavior.
8. Repeat on the second display; verify only that display changes.
9. Restart the view service; verify the selected format is restored.

## Acceptance

- The three-dot menu is drawn over every format in the upper-left.
- The menu lists exactly one entry per format from `contracts/formats.toml`.
- Selecting an entry changes the active format for that display only.
- The menu closes on selection and on outside clicks.
- Existing click-to-Next behavior is preserved.
- The controller/view service persist the selection without changes.
- No ABI or topic changes are required.

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

- **GL state:** the menu must set its own viewport, scissor, depth, and blend state after arbitrary format renderers.
- **Layout drift:** rendering and hit-testing must share the same pure layout function.
- **Text baseline:** top-left text placement needs a small visual adjustment for font bearings; verify manually.
- **Small windows:** clamp the panel to the region; no scrolling in this story.
- **Per-window future:** only window 0 renders today; keep the menu state keyed by window where practical.
- **Click consumption:** an open-menu outside click must not also trigger `Next`.
- **Font duplication:** use one shared font path constant rather than duplicating it in format modules.

## Files touched

- `rust/lib/display-lib/src/format_menu.rs` (new)
- `rust/lib/display-lib/src/lib.rs`
- `rust/lib/display-lib/src/format_manager.rs`
- `rust/lib/display-lib/src/format_source.rs`
- `rust/lib/display-lib/src/text.rs`
- `rust/lib/display-lib/src/formats/hello_world.rs`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Result

Implemented and verified by tests/build:

- Added `format_menu` with the three-dot button, panel, format rows, active-format highlight, and pure layout/hit-test state.
- Extended `TextRenderer` with `draw_at` and `text_width`, plus a shared font path constant.
- `FormatManager` now owns the menu, draws it over the active format, and produces `Next`/`Select` actions.
- `format_source` sends `Select` events; the controller and view service persist them without changes.
- Added menu layout/state tests and text-layout tests.
- `cargo test`/`build.ps1 -Test` pass; release `build.ps1` deploys.
- A live launch opened the menu over `hello-world`, and choosing `default` wrote `format = "default"` to the display's view-service state file.
- Remaining manual checks: two-display independence, outside-click/Next behavior, and saved-format restore after restart.

## Bookkeeping

- `multi-lil-d` was completed at `2026-11-10 16:13:29` and moved to `completed_stories.md` under "Later increments".
- `format-menu` is `in progress` in `backlog.md`.
- Do not move `format-menu` to `completed_stories.md` until the user asks to plan the next story.
