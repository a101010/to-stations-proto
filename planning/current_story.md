# Current story: cpp-display-chooser

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make the format chooser implicit and implement it in C++. Remove `format-chooser` from `formats.toml`; the chooser always exists, is assignable as an active format, and is never a catalog or chooser tile. Update the Rust format list and services to treat it as a reserved implicit format, then add the C++ icon module and chooser: 4x4 grid, icon/title tiles, pager strip, selection, D-pad navigation, mouse tile selection, and the existing `Formats` menu slot selecting it. Reuse the project's image decoder for PNG/JPG icons.

- **Depends on:** none.
- **Minimal test:** C++ `lil-d` with default and hello-world formats switches between them through the menu and chooser; D-pad selection wraps correctly; `formats.toml` lists no chooser; Rust services accept and preserve a `format-chooser` assignment; `Next` skips the chooser.

## Context

- `cleanup-paths` is complete at `2026-11-23 12:05:05`.
- The Rust display library currently expects a `format-chooser` entry in `formats.toml` and uses a placeholder slot while building the registry.
- The C++ display library currently skips `format-chooser` and does not expose a chooser renderer.
- The `Formats` menu item already publishes `Select` for `format-chooser`, but Rust format services currently validate selections against the catalog; after removing the entry they must treat the chooser as implicitly known.
- The Rust and C++ display libraries both need the chooser to remain an active selectable target even though it is absent from the ordered catalog.
- `assets/icons/default.png` and `assets/icons/hello-world.png` already exist as 64x64 RGBA PNGs.
- The C++ `stb_image` decoder and `load_image(path, desired_channels)` wrapper are already present.
- The C++ format manager already hides the menu while `format-chooser` is active and routes input to the active format.
- `paws`/`format-ctl-service` already send `Navigate`, `Pointer`, and `Select` commands.

## Decisions

1. **The format chooser is implicit.** It is not listed in `formats.toml`, is not part of the selectable catalog, and never appears as a tile inside itself.
2. **It is still assignable.** A `format-chooser` assignment is valid for the active format window.
3. **It is always constructed.** Both Rust and C++ display libraries build the chooser after building the catalog formats; no slot in the list is required.
4. **`Next` skips it.** The catalog order contains only selectable formats, so `Next` never advances to the chooser.
5. **Rust format services recognize the implicit chooser.** `FormatList` exposes a reserved `FORMAT_CHOOSER` constant and an `is_known` check used by selection and saved-assignment validation.
6. **The list parser rejects an explicit chooser entry.** A `format-chooser` entry in `formats.toml` is a contract error, not a normal format.
7. **`FormatInput` carries the selected format name.** The chooser returns a concrete catalog name for tile selection.
8. **Reuse `stb_image`.** Add the C++ `icon` module; no new dependency.
9. **No paging controls yet.** Reserve the pager strip; show at most 16 entries.
10. **No DDS/deploy changes in this story.** The existing Rust format services remain stand-ins.

## Scope

**In scope**

- Remove `format-chooser` from `contracts/formats.toml` and update its comment.
- Add the reserved implicit chooser rule to Rust `format-list`.
- Update Rust services and saved-assignment validation.
- Update the Rust display registry to always construct the chooser.
- Refactor C++ `FormatInput` to carry a selected format name.
- Update the C++ registry to always construct the chooser.
- Add C++ `icon` and `format_chooser` modules.
- Implement chooser layout, rendering, mouse selection, D-pad wrapping, and controller selection.
- Add CTest coverage for chooser geometry, navigation, and icon fitting.
- Update planning architecture and current story docs.

**Out of scope**

- Paging controls and more than 16 formats.
- Animated/hover icons.
- C++ format services or C++ `paws`.
- Lux Color/Lux Pos overlays.
- New assets or icon sizes.
- README restructuring.

## Files

```text
contracts/formats.toml
rust/lib/format-list/src/lib.rs
rust/lib/display-lib/src/formats.rs
rust/lib/display-lib/Cargo.toml
rust/service/format-ctl-service/src/main.rs
rust/service/format-view-service/src/main.rs
cpp/libs/display-lib/CMakeLists.txt
cpp/libs/display-lib/src/formats.hpp
cpp/libs/display-lib/src/formats.cpp
cpp/libs/display-lib/src/format_manager_impl.cpp
cpp/libs/display-lib/src/formats/hello_world.cpp
cpp/libs/display-lib/src/icon.hpp/.cpp
cpp/libs/display-lib/src/format_chooser.hpp/.cpp
cpp/libs/display-lib/tests/main.cpp
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No changes are expected in:

- `scripts/build.ps1`
- `scripts/launch-*.ps1`
- `scripts/cleanup.ps1`
- `assets/`
- `docs/versions.md`

## Implementation

### 1. Contract and Rust implicit chooser

`contracts/formats.toml`:

- Keep only the selectable catalog:
  - `hello-world`
  - `default`
- Update the comment to state that `format-chooser` is implicit and must not be listed.

`rust/lib/format-list/src/lib.rs`:

- Add `pub const FORMAT_CHOOSER: &str = "format-chooser";`.
- Reject `format-chooser` in `FormatList::from_toml` with a clear error.
- Keep `names()` and `next()` limited to the catalog.
- Add:

```rust
pub fn is_known(&self, name: &str) -> bool {
    name == FORMAT_CHOOSER || self.contains(name)
}
```

- Keep `contains()` meaning “is in the ordered catalog.”
- Use `is_known` in `SavedAssignments::validated`, so a persisted chooser assignment is preserved instead of falling back to `hello-world`.

`rust/service/format-ctl-service/src/main.rs`:

- Use `formats.is_known(&event.format)` for `Select` validation.
- Add tests that `format-chooser` is accepted and `Next` never selects it.

`rust/service/format-view-service/src/main.rs`:

- Rely on the updated `SavedAssignments::validated` behavior.
- Add a test that a `format-chooser` control assignment survives validation.

### 2. Rust display registry

`rust/lib/display-lib/src/formats.rs`:

- Use the shared `format_list::FORMAT_CHOOSER` constant.
- Remove the chooser slot/placeholder logic.
- Build the catalog from the listed formats only.
- Always construct `FormatChooser` after the catalog is complete.
- Append the chooser to the registry so assignments to `format-chooser` resolve.
- Keep the chooser out of its own catalog.
- Unknown listed names remain errors.

### 3. C++ selection plumbing

Change `FormatInput` from a bare enum to:

```cpp
struct FormatInput {
    enum class Type { Ignored, Consumed, Select };
    Type type = Type::Ignored;
    std::string format;
};
```

Update:

- Base `Format` defaults.
- `HelloWorld` ignored/consumed returns.
- `FormatManager` checks and selections.
- `FormatManager::mouse_button`/`command` use `input.format` for selection.
- `FormatManager::menu_action` selection checks.

The chooser returns `Select` with `default` or `hello-world`; it does not return `Select` for the chooser itself.

### 4. C++ implicit registry and catalog

In `formats.hpp`:

- Add `FormatMeta { name, title, icon_file }`.

In `formats.cpp`:

- Build listed formats and collect their metadata.
- Reject `format-chooser` in the C++ format list parser.
- Always construct `FormatChooser(assets_root, catalog)` after the catalog is complete.
- Append it to the registry.
- Remove the obsolete `chooser_available` flag; the chooser is unconditional.
- Ensure `first_name()` remains the first catalog format.

Assignments to `format-chooser` now resolve through the registry, and `Formats` selection continues to publish that name.

### 5. C++ icon module

Add `icon.hpp/.cpp`:

- `Icon` owns a 2D RGBA texture and dimensions.
- Load via `load_image(path, 4)`.
- Upload as `GL_RGBA8` with linear filtering and clamp-to-edge.
- `fit_rect(rect, image_w, image_h)` centers an aspect-preserving fit.
- `Icon::fit(rect)` uses the image dimensions.
- `IconRenderer` owns the textured-quad shader, VAO, and VBO.
- Draw maps the icon rect from format-region coordinates into clip space.
- Enable source-alpha blending and restore state.

No new assets or dependency.

### 6. C++ chooser

Add `format_chooser.hpp/.cpp`:

- 4 columns × 4 rows, capacity 16.
- `ChooserTile` with tile/icon/title rects and index.
- `ChooserLayout` with tiles and reserved pager rect.
- `tile_at(x, y)` hit-testing.
- `wrap_index(selected, count, dx, dy)` matching Rust:
  - horizontal movement wraps within the current row;
  - vertical movement wraps within the current column;
  - sparse grids skip empty cells;
  - no movement when no other occupied tile exists.
- `selectable` excludes the implicit chooser by construction (catalog contains only listed formats).
- Background, tile, selected-tile, and title colors matching Rust.
- Render:
  - clear the full format region;
  - set viewport and disable depth;
  - draw tile backgrounds;
  - draw fitted icons and centered titles.
- Input:
  - left-click on a tile selects it and returns `Select` with the format name;
  - left-click elsewhere in the region consumes;
  - right-click is ignored;
  - `Navigate` updates selection and consumes;
  - controller right-pointer selects the highlighted tile;
  - other pointer/move/menu/overlay commands consume.
- The chooser never contributes menu items.

The existing manager already closes/hides the menu while the chooser is active and routes input to it.

### 7. Tests

Rust:

- `format-list` rejects an explicit `format-chooser` entry.
- `is_known("format-chooser")` is true while `contains("format-chooser")` is false.
- `next` never returns the chooser.
- Saved assignment validation preserves `format-chooser`.
- `format-ctl-service` accepts a chooser `Select`.
- Display registry tests, if practical, verify chooser availability without relying on a list slot.

C++:

- `fit_rect` centers in wide/tall/invalid/zero-size rects.
- `ChooserLayout` has 16 cells in 4 columns for 16 entries.
- Tiles stay inside the region and above the pager.
- Two entries use only the first two cells.
- `tile_at` maps centers and ignores empty space.
- Empty catalog/invalid region has no tiles.
- `wrap_index` covers two-entry, full-grid, sparse-grid, and empty cases.
- Format-list parser rejects `format-chooser`.
- Existing image test continues to verify both icon assets are 64x64 RGBA.

### 8. Documentation

- Update `architecture.md`:
  - `formats.toml` is the ordered catalog only; `format-chooser` is implicit.
  - The chooser is always available and assignable but never a catalog item.
  - Rust `format-list` owns the reserved chooser rule.
  - C++ display-lib module list includes `icon`.
- Keep README changes out of scope.

## Manual verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-cpp.ps1 -DisplayId 0
```

With a temporary `LOCALAPPDATA`:

- `contracts/formats.toml` contains only `hello-world` and `default`.
- Open the menu and choose `Formats`; the chooser appears.
- The chooser offers only `Hello World` and `Default` tiles with their icons.
- The first tile is selected initially.
- Mouse tile click switches the format.
- Clicking empty chooser space consumes without changing the format.
- D-pad left/right wraps between the two tiles.
- D-pad up/down is a no-op with two entries.
- Controller A/right-click selects the highlighted tile.
- The menu is hidden while the chooser is active.
- `format-chooser` can be assigned and is restored across restart.
- `Next` does not select the chooser.
- Rust and C++ configurations both still run.
- The relocated deploy still works.
- Remove only temporary test state.

## Risks

- `FormatInput` must carry the format name and all call sites must use `.type`; otherwise chooser selection can select the current active format instead of the clicked tile.
- An explicit `format-chooser` in `formats.toml` must be rejected; silently accepting it would put the chooser back in the catalog and reintroduce the old slot logic.
- Rust selection and saved-assignment validation must use `is_known`, not `contains`, or the `Formats` menu path will be rejected after the contract change.
- The chooser must be constructed after the catalog is complete, then appended; it must never be included in its own catalog.
- Icon texture/blending state must not leak into the next format or menu render.
- D-pad wrapping must match Rust exactly, especially sparse grids.
- Do not touch real `%LOCALAPPDATA%`; GUI tests use temporary state.

## Bookkeeping

- `cleanup-paths` is completed at `2026-11-23 12:05:05`.
- `cpp-display-chooser` is `in progress` in `backlog.md`.
- `cpp-display-chooser` has no remaining story prerequisites.
- Do not move `cpp-display-chooser` to `completed_stories.md` until the user asks to plan the next story.
