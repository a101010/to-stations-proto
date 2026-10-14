# Current story: lil-d

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the small display application (`lil-d`): a window and an egui console area, with the region delegated to the display library painted a dark unsaturated green placeholder. The console has a single power button that toggles the format window. Takes `--station N` on the command line. Extends `scripts/build.ps1` to build and deploy the executable to `build/deploy`, and adds `scripts/launch-rust.ps1` and `scripts/cleanup.ps1`; later stories that add executables or libraries extend these scripts as needed.

- **Depends on:** none.
- **Minimal test:** the window opens showing the console and the green region; the power button toggles the format window; `launch-rust.ps1` starts it and `cleanup.ps1` stops it.

## Context

`rust-render` proved the windowing/rendering stack (winit 0.30.13, glutin 0.32.3, glutin-winit 0.5.0, glow 0.17, egui/egui-winit/egui_glow 0.36.2) with a temporary smoke test under `build/rust-render-smoke`; that directory is gitignored and is the reference for the window/GL/egui setup. No committed display crate exists: `rust/` holds only `lib/stations-dds` and `tool/equirect-to-cubemap`, and there is no `rust/ui/` yet.

`scripts/build.ps1` currently generates IDL into `build/gen/rust` and builds the Rust workspace with the Cargo target directory `build/rust`; it has no deploy step, and `scripts/launch-rust.ps1` and `scripts/cleanup.ps1` do not exist.

`lil-d` is the display application that will host the display library. In this story the display library does not exist yet, so the region it will own is painted a placeholder color.

The third-party APIs below were verified against the installed `egui_glow-0.36.2`, `egui-0.36.2`, and `egui-winit-0.36.2` crates.

## Decisions

* **Crate.** `lil-d` lives at `rust/ui/lil-d`, a binary crate (`architecture.md` puts GUI applications under `ui/`).
* **Stack reuse.** Follow the smoke test's winit + glutin/glow + egui_glow setup; the windowing/rendering dependencies come from `[workspace.dependencies]` in `rust/Cargo.toml`.
* **Window.** One OS window: a square format region (the region that will be delegated to the display library) painted a named dark unsaturated green constant, with the console area around it. The power button lives in the console and toggles the format region on/off.
* **Arguments.** `--station N` via `clap`, default `0`. `--topics` is not added yet; it arrives with `dds-and-service-contracts`.
* **No DDS, no `InputEvent`, no display library.** `lil-d` does not forward input yet; the display application forwards input to the display library when `format-manager` needs it.
* **Build and deploy are script-owned and grow with the stories.** `scripts/build.ps1` gains a step that assembles `build/deploy/bin` with `lil-d.exe`; the debug binary is deployed for now (a release switch can come later). `scripts/launch-rust.ps1` and `scripts/cleanup.ps1` are new, location-agnostic (resolve the repository root from `$PSScriptRoot`; no drive letter), and PowerShell 5.1 compatible.
* **Launch and cleanup operate only on `build/deploy`.** `launch-rust.ps1` starts `build/deploy/bin/lil-d.exe` with its station argument; the model and view services join it in later stories. `cleanup.ps1` stops all configuration processes.

## Crate layout

```
rust/ui/lil-d/
  Cargo.toml
  src/
    main.rs        entry point
    cli.rs         command-line arguments
    layout.rs      format-region/console geometry
    gl_window.rs   winit window + glutin/glow context
    console.rs     egui console (power button)
    app.rs         ApplicationHandler, state, render loop
```

`rust/Cargo.toml` adds `"ui/lil-d"` to `members`.

## Cargo.toml

```toml
[package]
name = "lil-d"
version = "0.0.0"
edition = "2021"
publish = false

[[bin]]
name = "lil-d"
path = "src/main.rs"

[dependencies]
winit = { workspace = true }
glutin = { workspace = true }
glutin-winit = { workspace = true }
glow = { workspace = true }
egui = { workspace = true }
egui_glow = { workspace = true }
clap = { workspace = true }
```

`egui_glow` is pulled with its `winit` feature from `[workspace.dependencies]`; it re-exports `egui_winit` and `EventResponse`, so `egui-winit` is not a direct dependency. All versions are already pinned in `rust/Cargo.toml`.

## Interfaces

### `cli.rs`

```rust
#[derive(clap::Parser, Debug)]
#[command(name = "lil-d", about = "to-stations small display")]
pub struct Args {
    /// Station number.
    #[arg(long, default_value_t = 0)]
    pub station: u32,
}
```

### `layout.rs`

```rust
/// Console strip height in logical pixels.
pub const CONSOLE_HEIGHT_LOGICAL: f64 = 64.0;

/// A rectangle in physical pixels, origin top-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }

/// The window split into the region delegated to the display library and the console.
#[derive(Clone, Copy, Debug)]
pub struct Layout { pub format: Rect, pub console: Rect }

/// Compute the layout for a window of `size` (physical pixels) at `scale`.
pub fn compute(size: winit::dpi::PhysicalSize<u32>, scale: f64) -> Layout;
```

Pseudocode:

```
fn compute(size, scale):
    w = size.width as i32
    h = size.height as i32
    console_h = clamp(round(CONSOLE_HEIGHT_LOGICAL * scale), 0, h)
    console = Rect { x: 0, y: h - console_h, w, h: console_h }
    avail_h = h - console_h
    side = min(w, avail_h)
    format = Rect { x: (w - side)/2, y: (avail_h - side)/2, w: side, h: side }
    return Layout { format, console }
```

### `gl_window.rs`

```rust
pub struct GlWindow {
    window: winit::window::Window,
    gl_context: glutin::context::PossiblyCurrentContext,
    gl_display: glutin::display::Display,
    gl_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
}

impl GlWindow {
    /// Safety: must be called on the event-loop thread.
    pub unsafe fn new(
        event_loop: &winit::event_loop::ActiveEventLoop,
        title: &str,
        size: winit::dpi::LogicalSize<f64>,
    ) -> Self;

    pub fn window(&self) -> &winit::window::Window;
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32>;
    pub fn scale_factor(&self) -> f64;
    pub fn resize(&self, width: core::num::NonZeroU32, height: core::num::NonZeroU32);
    pub fn swap_buffers(&self);
    pub fn get_proc_address(&self, name: &core::ffi::CStr) -> *const core::ffi::c_void;
}
```

`new` is the smoke test's `GlWindow::new` verbatim (glutin_winit `DisplayBuilder`, context creation with a GLES fallback, window surface, `make_current`, swap interval `Wait(1)`). `size` returns `window.inner_size()`; `scale_factor` returns `window.scale_factor()`.

### `console.rs`

```rust
/// What the console requested this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct ConsoleAction { pub toggle_power: bool }

/// Draw the console into the root egui Ui and report the user's action.
pub fn draw(ui: &mut egui::Ui, station: u32, format_on: bool) -> ConsoleAction;
```

Pseudocode (egui 0.36 panel API, verified):

```
fn draw(ui, station, format_on):
    action = ConsoleAction::default()
    egui::Panel::bottom("lil-d-console")
        .exact_size(CONSOLE_HEIGHT_LOGICAL as f32)
        .frame(egui::Frame::new().fill(CONSOLE_FILL))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label(format!("station {station}"))
                ui.separator()
                label = if format_on { "Power: on" } else { "Power: off" }
                if ui.button(label).clicked() { action.toggle_power = true }
            })
        })
    return action
```

### `app.rs`

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct AppState { pub format_on: bool }

pub struct App {
    args: cli::Args,
    state: AppState,
    gl_window: Option<gl_window::GlWindow>,
    gl: Option<std::sync::Arc<glow::Context>>,
    egui_glow: Option<egui_glow::EguiGlow>,
}

impl App {
    pub fn new(args: cli::Args) -> Self;
    fn setup(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn render(&mut self);
    fn clear(&self, color: [f32; 4]);
    fn paint_format(&self, rect: layout::Rect, color: [f32; 4]);
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn window_event(&mut self, event_loop: &winit::event_loop::ActiveEventLoop,
                    id: winit::window::WindowId, event: winit::event::WindowEvent);
    fn about_to_wait(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
    fn exiting(&mut self, event_loop: &winit::event_loop::ActiveEventLoop);
}
```

Pseudocode:

```
fn setup(event_loop):
    gl_window = unsafe { GlWindow::new(event_loop, "lil-d", LogicalSize::new(720.0, 720.0)) }
    gl = unsafe { glow::Context::from_loader_function(|s| {
        let cs = CString::new(s).unwrap();
        gl_window.get_proc_address(&cs)
    }) }
    gl = Arc::new(gl)
    egui_glow = EguiGlow::new(event_loop, Arc::clone(&gl), None, None, true)
    self.gl_window = Some(gl_window)
    self.gl = Some(gl)
    self.egui_glow = Some(egui_glow)
    self.state.format_on = true
    self.gl_window.window().request_redraw()

fn clear(color):
    gl = self.gl.unwrap()
    (w, h) = self.gl_window.size()
    unsafe {
        gl.viewport(0, 0, w as i32, h as i32)
        gl.clear_color(color...)
        gl.clear(glow::COLOR_BUFFER_BIT)
    }

fn paint_format(rect, color):
    if rect.w <= 0 or rect.h <= 0: return
    gl = self.gl.unwrap()
    (_, h) = self.gl_window.size()
    y_gl = h as i32 - rect.y - rect.h          // GL origin is bottom-left
    unsafe {
        gl.enable(glow::SCISSOR_TEST)
        gl.scissor(rect.x, y_gl, rect.w, rect.h)
        gl.clear_color(color...)
        gl.clear(glow::COLOR_BUFFER_BIT)
        gl.disable(glow::SCISSOR_TEST)
    }

fn render():
    gl_window = self.gl_window.unwrap()
    egui_glow = self.egui_glow.unwrap()
    layout = layout::compute(gl_window.size(), gl_window.scale_factor())

    self.clear(CONSOLE_BG)                                  // 1. console background, whole window
    fmt = if self.state.format_on { FORMAT_GREEN } else { CONSOLE_BG }
    self.paint_format(layout.format, fmt)                   // 2. delegated region

    station = self.args.station
    format_on = self.state.format_on
    action = ConsoleAction::default()
    egui_glow.run(gl_window.window(), |ui| {                // 3. egui console
        action = console::draw(ui, station, format_on)
    })
    if action.toggle_power {
        self.state.format_on = !self.state.format_on
        gl_window.window().request_redraw()
    }

    egui_glow.paint(gl_window.window())                     // 4. present
    gl_window.swap_buffers()
```

Event handling:

```
fn resumed(event_loop):
    if self.gl_window.is_none(): self.setup(event_loop)

fn window_event(event_loop, _id, event):
    // 1. Let egui see the event (mouse, keyboard, focus).
    if let (Some(gl_window), Some(egui_glow)) = (self.gl_window.as_ref(), self.egui_glow.as_mut()):
        resp = egui_glow.on_window_event(gl_window.window(), &event)
        if resp.repaint: gl_window.window().request_redraw()

    // 2. Handle window lifecycle.
    match event:
        CloseRequested | Destroyed => event_loop.exit()
        Resized(size) =>
            w = NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN)
            h = NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN)
            self.gl_window.resize(w, h)
            self.gl_window.window().request_redraw()
        RedrawRequested => self.render()
        _ => {}

fn about_to_wait(_event_loop):
    // Static UI; egui repaints are driven by on_window_event's `repaint`.
    // No continuous animation yet, so nothing to do.

fn exiting(_event_loop):
    if let Some(egui_glow) = self.egui_glow.as_mut(): egui_glow.destroy()
```

### `main.rs`

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Args::parse();
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut app = app::App::new(args);
    event_loop.run_app(&mut app)?;
    Ok(())
}
```

## Constants

```rust
const WINDOW_TITLE: &str = "lil-d";
const WINDOW_SIZE: (f64, f64) = (720.0, 720.0);           // logical
const CONSOLE_HEIGHT_LOGICAL: f64 = 64.0;                 // in layout.rs
const CONSOLE_BG: [f32; 4] = [0.06, 0.06, 0.07, 1.0];     // near-black
const CONSOLE_FILL: egui::Color32 = egui::Color32::from_rgb(15, 15, 18);
const FORMAT_GREEN: [f32; 4] = [0.10, 0.18, 0.10, 1.0];   // dark, unsaturated green
```

The green is a placeholder for the region the display library will own; it is deliberately distinct from the console background.

## Render/event flow

* The whole window is cleared to `CONSOLE_BG`; the square `layout.format` region is then cleared to `FORMAT_GREEN` (or `CONSOLE_BG` when powered off) using a GL scissor rect.
* egui is run and painted last, drawing the bottom console panel; the root egui Ui paints no background, so the format region shows through.
* egui receives input through `EguiGlow::on_window_event`; a click on the power button toggles `AppState::format_on` and requests a redraw.
* The event loop uses `ControlFlow::Wait` (winit default): frames are drawn on `RedrawRequested`, which is requested on resize, on egui repaint, and on the power toggle.

## Scripts

### `scripts/build.ps1` (extended; existing steps unchanged)

```
... existing: cargo install cyclonedds-idlc, generate into build/gen/rust ...
Push-Location $repoRoot
try {
    if ($Test) { & cargo test --manifest-path 'rust\Cargo.toml' }
    else       { & cargo build --manifest-path 'rust\Cargo.toml' }
    if ($LASTEXITCODE -ne 0) { throw "cargo build/test failed ($LASTEXITCODE)" }
} finally { Pop-Location }

# deploy (new)
$deployBin = Join-Path $repoRoot 'build\deploy\bin'
New-Item -ItemType Directory -Force -Path $deployBin | Out-Null
Copy-Item -Path (Join-Path $repoRoot 'build\rust\debug\lil-d.exe') -Destination $deployBin -Force
```

### `scripts/launch-rust.ps1` (new)

```
[CmdletBinding()] param([int]$Station = 0)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repoRoot 'build\deploy\bin\lil-d.exe'
if (-not (Test-Path $exe)) { throw "not built: $exe; run scripts\build.ps1 first" }
Start-Process -FilePath $exe -ArgumentList '--station', $Station
```

### `scripts/cleanup.ps1` (new)

```
[CmdletBinding()] param()
$ErrorActionPreference = 'Stop'
$apps = @('lil-d')   # later stories append hello-service, hello-view-service, ...
foreach ($name in $apps) {
    Get-Process -Name $name -ErrorAction SilentlyContinue |
        Stop-Process -Force -ErrorAction SilentlyContinue
}
```

All three resolve the repository root from `$PSScriptRoot`, use repository-relative paths, hardcode no drive letter, and are PowerShell 5.1 compatible. Launch and cleanup touch only `build/deploy`.

## Deliverables

1. `rust/ui/lil-d/Cargo.toml` - the binary crate, using the workspace dependencies.
2. `rust/ui/lil-d/src/main.rs`, `cli.rs`, `layout.rs`, `gl_window.rs`, `console.rs`, `app.rs` - as specified above.
3. `rust/Cargo.toml` - add `ui/lil-d` to `members`.
4. `scripts/build.ps1` - assemble `build/deploy/bin` after the build.
5. `scripts/launch-rust.ps1` - new; start `build/deploy/bin/lil-d.exe`.
6. `scripts/cleanup.ps1` - new; stop all configuration processes.
7. Planning: this file, `planning/backlog.md`, `planning/architecture.md`.

## Tests

No headless test is practical for the windowed shell. The minimal test is manual.

## Minimal test

```
.\scripts\build.ps1
.\scripts\launch-rust.ps1
```

The window opens with the console ("station 0" and the power button) and the green square region; the power button toggles the region; `.\scripts\cleanup.ps1` stops it.

## Risks and follow-ups

* **egui_glow API drift.** The plan pins the verified 0.36.2 calls (`EguiGlow::new`, `run(&Window, impl FnMut(&mut Ui))`, `paint`, `on_window_event`); follow the smoke test if the version moves.
* **GL scissor origin.** The Y flip (`h - y - h`) is easy to get wrong; the format region must be visually square and above the console.
* **Panel-inside-root-Ui.** `egui::Panel::bottom(...).show(ui, ...)` inside the `run_ui` root Ui is the 0.36 API; if a repaint or sizing issue appears, fall back to a full-window `CentralPanel` with a manual console rect.
* **Deploy profile.** Debug is deployed now; a release switch is a later decision.
* **`cleanup.ps1` breadth.** Stopping by image name is simple; replace with tracked PIDs or a job object if it proves too broad.
* **Window/console geometry** and the exact green are provisional and will be revisited when `display-lib` takes over the region.

## Files

* Authored: `rust/ui/lil-d/Cargo.toml`, `rust/ui/lil-d/src/{main,cli,layout,gl_window,console,app}.rs`, `scripts/launch-rust.ps1`, `scripts/cleanup.ps1`.
* Changed: `rust/Cargo.toml`, `scripts/build.ps1`, `planning/backlog.md`, `planning/architecture.md`, `planning/current_story.md`.
* Reference (gitignored, not committed): `build/rust-render-smoke/`.
