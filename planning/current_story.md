# Current story: dds-echo

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the `dds-echo` traffic inspector as a Slint GUI mockup driven by simulated traffic: Start/Stop capture, a high-level stats view, a collapsible known-topics tree, and a datagram list with a detail/hex pane. No real capture yet; the known topics are hardcoded for the mockup.

- **Depends on:** none.
- **Minimal test:** the window opens; Start populates the stats, known-topics tree, and datagram list from the simulated feed; Stop halts it; selecting a row shows details and hex.

## Context

* Gui stack decision: the display apps keep `winit + glutin/glow + egui`; the tool/launcher/controller GUIs use **Slint**. `dds-echo` is the first Slint app.
* Slint `1.19.0`: a `.slint` UI compiled by `slint-build` in `build.rs`, pulled in with `slint::include_modules!()`. Default backend is winit + FemtoVG (Slint owns the window and context); Slint exposes Rust and C++ APIs. License: GPL-3.0-only / Slint Royalty-free 2.0 / Commercial (accepted).
* `dds-echo` stays a `tool/` crate (a developer tool with a GUI).
* The mockup is deliberately backend-free: a simulated feed exercises the layout, models, and stats. Later stories add the real UDP capture, RTPS/SEDP parsing, and known-type decode.

## Decisions

* **Crate at `rust/tool/dds-echo`**, a Slint app: `build.rs` compiles `ui/dds_echo.slint`; `src/main.rs` wires it; `src/model.rs` holds the data model.
* **Simulated feed via a Slint `Timer`.** No threads; Start/Stop starts and stops the timer. The capture story replaces the timer with a background thread marshalled through `slint::invoke_from_event_loop`.
* **Plain-Rust data model** (`records`, `stats`, `SimFeed`, `KNOWN_TOPICS`) separate from the UI, so it is unit-testable and the later backend plugs in unchanged.
* **Known topics hardcoded** for the mockup; the capture/SEDP stories read `contracts/topics.toml`.
* **One window**: toolbar (Start/Stop, inert interface/group/port fields, a `SIMULATED` badge), a stats strip, a left collapsible **Known topics** tree, a center datagram table (virtualized), and a detail pane (fields + hex).

## Interfaces

### `rust/tool/dds-echo/Cargo.toml` (new)

```toml
[package]
name = "dds-echo"
version = "0.0.0"
edition = "2021"
publish = false
build = "build.rs"

[dependencies]
slint = "1.19"

[build-dependencies]
slint-build = "1.19"
```

### `rust/tool/dds-echo/build.rs` (new)

```rust
fn main() {
    slint_build::compile("ui/dds_echo.slint").expect("compile dds_echo.slint");
}
```

### `rust/tool/dds-echo/ui/dds_echo.slint` (new)

```slint
export struct Record { time: string, kind: string, source: string, size: int, rtps: bool, topic: string, summary: string, hex: string }
export struct TopicStat { name: string, known: bool, count: int }

export component MainWindow inherits Window {
    in-out property <bool> running: false;
    in-out property <int> packets: 0;
    in-out property <int> bytes: 0;
    in-out property <int> pps: 0;
    in-out property <int> bps: 0;
    in-out property <int> participants: 0;
    in-out property <int> endpoints: 0;
    in-out property <[Record]> records: [];
    in-out property <[TopicStat]> topics: [];
    in-out property <Record> selected: { ... };
    callback start();
    callback stop();
    callback clear();
    callback select(int);
    // toolbar, stats strip, Known-topics tree (collapsible), records table, detail/hex pane
}
```

### `rust/tool/dds-echo/src/main.rs` (new)

```rust
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    // wire start/stop/clear/select to model.rs; drive a slint::Timer that appends
    // simulated Records to a VecModel and updates the stats properties
    ui.run()
}
```

### `rust/tool/dds-echo/src/model.rs` (new)

```rust
pub struct Record { pub time: String, pub kind: String, pub source: String, pub size: usize,
                    pub rtps: bool, pub topic: String, pub summary: String, pub hex: String }
pub struct TopicStat { pub name: String, pub known: bool, pub count: usize }

#[derive(Default)]
pub struct Stats { pub packets: usize, pub bytes: usize, pub rtps: usize }
impl Stats { pub fn observe(&mut self, r: &Record); }

/// Emits RTPS-looking datagrams plus a few HelloWorldTopic samples.
pub struct SimFeed { /* ... */ }
impl SimFeed { pub fn new() -> Self; pub fn next(&mut self) -> Record; }

/// Hardcoded for the mockup; the capture story reads contracts/topics.toml.
pub const KNOWN_TOPICS: &[&str] = &["HelloWorldTopic"];
```

### `rust/Cargo.toml` and `scripts/build.ps1` (changed)

Add member `"tool/dds-echo"`. `build.ps1` deploys `dds-echo.exe` to `build/deploy/bin` and adds `rust\tool\dds-echo\src` to the bypass-scan roots (the tool calls no DDS API).

## Pseudocode

### Start/Stop and the simulated feed

```
start():
    if running: return
    running = true
    timer.start(repeat 100ms, tick)

tick():
    for i in 0..random(0..4):
        r = feed.next()
        stats.observe(r)
        records_model.push(r)                 // keep the last N
        bump the matching TopicStat count
    update stats properties (packets, bytes, pps, bps)

stop(): running = false; timer.stop()
clear(): records_model.clear(); stats = default; topic counts = 0
select(i): selected = records[i]
```

### `SimFeed::next`

```
if random() < 0.25:
    a HelloWorldTopic record (kind "sample", topic "HelloWorldTopic",
        summary "greeting=…", hex = a plausible CDR blob)
else:
    a raw datagram record (kind "packet", rtps = true, hex = random bytes
        prefixed with the "RTPS" magic for most)
```

## Tests

No UI automation; the model is tested directly.

* `model.rs`: `SimFeed::next` yields records with non-empty `hex`; the "RTPS" magic is present on RTPS records; `Stats::observe` accumulates `packets`/`bytes`/`rtps`; a `HelloWorldTopic` record has topic `HelloWorldTopic`.

## Minimal test

```
.\scripts\build.ps1
build\deploy\bin\dds-echo.exe
```

The window opens; **Start** fills the stats, the known-topics tree, and the datagram list from the simulated feed; **Stop** halts it; selecting a row shows its fields and hex; **Clear** resets.

## Risks and follow-ups

* **Toolchain.** Slint 1.19 requires rustc >= 1.95; verify the pinned `stable-x86_64-pc-windows-msvc` is current.
* **Renderer.** Slint defaults to winit + FemtoVG (OpenGL), but Slint owns the window and context — no manual GL plumbing.
* **Event loop.** Slint's loop is main-thread; the capture story must update models from its background thread via `slint::invoke_from_event_loop`.
* **Build weight.** Slint pulls a sizable dependency tree; the first build is slow.
* **Mockup data.** `KNOWN_TOPICS` and the simulated samples are placeholders; the capture/SEDP stories replace them with the registry and real traffic.

## Module layout

```
rust/tool/dds-echo/build.rs          Slint compile
rust/tool/dds-echo/ui/dds_echo.slint the UI
rust/tool/dds-echo/src/main.rs       window wiring + timer
rust/tool/dds-echo/src/model.rs      Record/TopicStat/Stats/SimFeed/KNOWN_TOPICS
```

## Files

* Authored: `rust/tool/dds-echo/{Cargo.toml,build.rs,ui/dds_echo.slint,src/main.rs,src/model.rs}`.
* Changed: `rust/Cargo.toml`, `scripts/build.ps1`, `planning/{architecture,backlog,current_story}.md`, optionally `README.md`.
