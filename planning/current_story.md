# Current story: format-manager

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add the format manager to the display library: a registry of the default and hello-world formats and the active format. `lil-d` forwards pointer input to the library; clicking in the region delegated to the display library switches the active format.

- **Depends on:** display-lib, font-handling, earth-rendering.
- **Minimal test:** clicking in the delegated region switches between the default and hello-world formats.

## Context

* `Display` (`lib.rs:47`) holds a single `formats::hello_world::HelloWorld`; `render` delegates straight to it.
* `formats/default.rs` is dormant (not declared in `formats.rs`) and still has its own local `compile`/`link` that must be reconciled with `src/shader.rs`.
* The ABI (`include/display_lib.h`) is versioned (`ABI_VERSION` / `DISPLAY_LIB_ABI_VERSION` / `EXPECTED_ABI_VERSION` = 1) but has no input function.
* `lil-d` (`app.rs`) already computes the region as `layout.format` (`Rect`, physical pixels, top-left origin, `layout.rs:8`) and calls `egui_glow.on_window_event` first, which returns `EventResponse { consumed, repaint }`. For `MouseInput`, `consumed` is `egui_wants_pointer_input()`, so clicks over egui widgets (the console) are consumed and clicks in the empty format region are not.
* `CursorMoved` carries a `PhysicalPosition<f64>`; `MouseInput` carries only the button and state, so the application must remember the last cursor position.
* Architecture (`architecture.md:72`) already states the manager's first form lives inside the display library and switches on a click; the deferred decision (`architecture.md:178`) names a top-level `format_manager` module.

## Decisions

* **`Format` trait and registry in `formats.rs`.** `pub(crate) trait Format { fn render(&mut self, gl, framebuffer_height, x, y, w, h); }`; `formats::registry(gl, assets_root) -> Result<Vec<Box<dyn Format>>, DisplayError>` builds the list. Both formats implement the trait; their inherent `render` bodies move into the trait impl, which becomes the only `render`.
* **`format_manager.rs` top-level module.** `FormatManager { formats, active, region }`: `render` remembers the last region `(x, y, w, h)` and delegates to the active format; `mouse_button` hit-tests that region and, on a left-button press inside it, advances to the next format (wrapping).
* **Default format reactivated.** `formats.rs` declares `pub(crate) mod default;`; `default.rs` drops its local `compile`/`link`, uses `crate::shader`, and implements `Format`. `ColoredMesh` stays local: no second mesh user, so no `mesh` module yet.
* **Initial format.** Registry order `[HelloWorld, Default]` with `active = 0`, so startup is the hello-world format (as today) and a click shows the default.
* **ABI grows a focused input function.** `display_lib_mouse_button(d, x, y, button, pressed)`; `(x, y)` are physical pixels from the window's top-left, `button` is 0=left / 1=right / 2=middle / 3=back / 4=forward / 5=other, and `pressed` is nonzero on press. Only the events the feature needs are added (input handling is added per feature). The ABI version bumps to **2** in all three places; a stale DLL then fails cleanly.
* **Region hit-test in the library.** The application forwards window-space coordinates; the library compares them against the region it was last asked to render. No region crosses the ABI.
* **`lil-d` forwarding.** Track the last `CursorMoved`; on `MouseInput`, if `!consumed`, `format_on`, and the left button, forward to the library and request a redraw. The library changes the format; the redraw shows it.

## Interfaces

### `include/display_lib.h` (changed)

```c
#define DISPLAY_LIB_ABI_VERSION 2u

/* Mouse button event. (x, y) are physical pixels from the window's top-left.
 * button: 0 = left, 1 = right, 2 = middle, 3 = back, 4 = forward, 5 = other.
 * pressed: nonzero on press, zero on release. The library hit-tests the region
 * it was last asked to render and may change the active format. */
void display_lib_mouse_button(display_lib* d, int x, int y, int button, int pressed);
```

### `rust/lib/display-lib/src/formats.rs` (changed)

```rust
pub(crate) mod default;
pub(crate) mod hello_world;

pub(crate) trait Format {
    fn render(&mut self, gl: &glow::Context, framebuffer_height: i32,
              x: i32, y: i32, width: i32, height: i32);
}

pub(crate) fn registry(
    gl: &glow::Context,
    assets_root: &std::path::Path,
) -> Result<Vec<Box<dyn Format>>, DisplayError>;
```

### `rust/lib/display-lib/src/format_manager.rs` (new)

```rust
pub(crate) struct FormatManager {
    formats: Vec<Box<dyn crate::formats::Format>>,
    active: usize,
    region: (i32, i32, i32, i32),
}
impl FormatManager {
    pub(crate) fn new(gl: &glow::Context, assets_root: &std::path::Path)
        -> Result<Self, DisplayError>;
    pub(crate) fn render(&mut self, gl: &glow::Context, framebuffer_height: i32,
                         x: i32, y: i32, width: i32, height: i32);
    pub(crate) fn mouse_button(&mut self, x: i32, y: i32, button: i32, pressed: bool);
}

fn contains(region: (i32, i32, i32, i32), x: i32, y: i32) -> bool;
fn advance(active: usize, len: usize) -> usize;

#[cfg(test)]
impl FormatManager {
    fn with_region(formats: Vec<Box<dyn crate::formats::Format>>,
                   region: (i32, i32, i32, i32)) -> Self;
    fn active(&self) -> usize;
}
```

### `rust/lib/display-lib/src/lib.rs` (changed)

```rust
mod earth; mod format_manager; mod formats; mod shader; mod text;

const ABI_VERSION: u32 = 2;

struct Display { gl: glow::Context, format_manager: format_manager::FormatManager }
impl Display {
    fn new(loader: impl FnMut(&str) -> *const c_void) -> Result<Self, DisplayError>;
    fn render(&mut self, framebuffer_height: i32, x: i32, y: i32, width: i32, height: i32);
    fn mouse_button(&mut self, x: i32, y: i32, button: i32, pressed: bool);
}

#[no_mangle]
pub extern "C" fn display_lib_mouse_button(
    display: *mut c_void, x: c_int, y: c_int, button: c_int, pressed: c_int);
```

### `rust/lib/display-lib/src/formats/default.rs` (changed)

```rust
impl crate::formats::Format for Default {
    fn render(&mut self, gl: &glow::Context, framebuffer_height: i32,
              x: i32, y: i32, width: i32, height: i32) { /* existing body */ }
}
// local compile/link removed; uses crate::shader::{compile, link}; dormant doc comment updated.
```

### `rust/lib/display-lib/src/formats/hello_world.rs` (changed)

```rust
impl crate::formats::Format for HelloWorld {
    fn render(&mut self, gl: &glow::Context, framebuffer_height: i32,
              x: i32, y: i32, width: i32, height: i32) { /* existing body */ }
}
```

### `rust/ui/lil-d/src/display.rs` (changed)

```rust
const EXPECTED_ABI_VERSION: u32 = 2;
type MouseButtonFn = unsafe extern "C" fn(*mut c_void, c_int, c_int, c_int, c_int);
// loads b"display_lib_mouse_button\0"; adds:
pub fn mouse_button(&mut self, x: i32, y: i32, button: i32, pressed: bool);
```

### `rust/ui/lil-d/src/app.rs` (changed)

```rust
struct App { /* ... */ cursor: Option<winit::dpi::PhysicalPosition<f64>> }
// window_event:
//   let response = egui_glow.on_window_event(...);   // keep response.consumed
//   CursorMoved { position, .. } => self.cursor = Some(position)
//   MouseInput { state, button, .. } =>
//       if !response.consumed && self.state.format_on {
//           if let (Some(pos), MouseButton::Left) = (self.cursor, button) {
//               display.mouse_button(pos.x as i32, pos.y as i32, 0, state == Pressed);
//               window.request_redraw();
//           }
//       }
```

## Pseudocode

### `FormatManager::render` / `mouse_button`

```
render(gl, fh, x, y, w, h):
    region = (x, y, w, h)
    if let Some(f) = formats.get_mut(active): f.render(gl, fh, x, y, w, h)

mouse_button(x, y, button, pressed):
    if button != 0 || !pressed: return          // left press only
    if contains(region, x, y) && !formats.is_empty():
        active = advance(active, formats.len())

contains((rx, ry, rw, rh), x, y) = x >= rx && x < rx + rw && y >= ry && y < ry + rh
advance(active, len) = (active + 1) % len
```

### `formats::registry`

```
registry(gl, assets_root):
    vec![ Box::new(HelloWorld::new(gl, assets_root)?),
          Box::new(Default::new(gl)?) ]
```

## Tests

No GL context:

* `format_manager::contains`: inside, outside (all four sides), and the boundaries (`x == rx`, `x == rx + rw`).
* `format_manager::advance`: increments and wraps at `len`.
* `format_manager` with fake `Format`s via `with_region`: a left press inside advances `active`; a left press outside does not; a right press does not; a release does not.
* `formats/default.rs` existing tests unchanged (x geometry).
* No GL test for `registry` (it constructs GL objects).

## Minimal test

```
.\scripts\build.ps1
.\scripts\launch-rust.ps1
```

Clicking in the delegated region switches between the hello-world format (earth and greeting) and the default format (amber background, green x); the power button still toggles the region; `.\scripts\cleanup.ps1` stops it.

Negative check: an old `display_lib.dll` (ABI 1) against the new `lil-d` fails at load with the ABI-version mismatch message.

## Risks and follow-ups

* **egui event consumption.** The click must reach the library only when egui did not consume it; verify clicks in the empty central region report `consumed == false` and console clicks report `true`.
* **Coordinate space.** `CursorMoved` is physical pixels, matching the physical `layout.format` region; no scale conversion is needed. A `MouseInput` with no prior `CursorMoved` is ignored.
* **Stale region.** The region is only updated by `render`; forwarding is gated on `format_on`, so a hidden region never changes the format.
* **ABI growth.** The C++ display library and application (later stories) must implement and consume `display_lib_mouse_button` and match ABI 2.
* **Format count.** Cycling assumes at least one format; guarded.
* **Module boundary.** `mesh` stays deferred (the default's `ColoredMesh` and the earth's sphere share no code).

## Module layout

Post-2018 layout (`foo.rs` plus a `foo/` directory; no `mod.rs`):

```
src/lib.rs                    ABI, Display, DisplayError, assets_root
src/shader.rs                 shared shader compile/link
src/text.rs                   text rendering
src/earth.rs                  earth rendering
src/format_manager.rs         registry, active format, region hit-test
src/formats.rs                the Format trait and the registry
src/formats/default.rs        the default format (reactivated)
src/formats/hello_world.rs    the hello-world format
```

## Result

Implemented. `display-lib` now has a `format_manager` module holding the format registry (`formats::registry`: hello-world then default) and the active format; `render` remembers the region and delegates to the active format, and `mouse_button` advances to the next format on a left-button press inside that region. The `Format` trait lives in `formats.rs`; the default format is reactivated and its local `compile`/`link` moved onto `crate::shader`. The ABI grew `display_lib_mouse_button` and bumped to version 2 across `include/display_lib.h`, `lib.rs`, and `lil-d`'s `display.rs`; `lil-d` tracks the cursor and forwards unconsumed (`EventResponse::consumed == false`) left-button events in the format region, then repaints. `scripts/build.ps1 -Test` passes (`display-lib` has 18 tests: 5 new format-manager tests plus the reactivated default-format tests), and the release deploy build succeeds with no warnings.

Deviation from the plan: `forward_mouse` lives on `impl App` (a plain helper), and the manager's test-only helpers are `with_region`/`active` behind `#[cfg(test)]`. The visual minimal test (clicking in the region switches formats) remains for the user to confirm with `scripts/launch-rust.ps1`.

## Files

* Authored: `rust/lib/display-lib/src/format_manager.rs`.
* Changed: `include/display_lib.h`, `rust/lib/display-lib/src/{lib.rs,formats.rs}`, `rust/lib/display-lib/src/formats/{default.rs,hello_world.rs}`, `rust/ui/lil-d/src/{display.rs,app.rs}`, `planning/{architecture,backlog,current_story}.md`, optionally `README.md`.
