# Current story: dds-echo

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the `dds-echo` traffic inspector as a Slint GUI mockup driven by simulated traffic: Start/Stop capture, a high-level stats view, a collapsible known-topics view, a datagram table, and a detail pane with hex. No real capture yet; the known topics are hardcoded for the mockup.

- **Depends on:** none.
- **Minimal test:** run the app; Start fills the stats, the topics view, and the table from the simulated feed; Stop halts; selecting a row shows fields and hex; Clear resets.

## Context

- GUI stack decision: the display apps keep `winit + glutin/glow + egui`; the tool/launcher/controller GUIs use **Slint**. `dds-echo` is the first Slint app (`architecture.md`, "Stack").
- Slint `1.19.0`: a `.slint` UI compiled by `slint-build` in `build.rs`, pulled in with `slint::include_modules!()`. Default backend is winit + FemtoVG (Slint owns the window and context). Slint exposes Rust and C++ APIs. License: GPL-3.0-only / Slint Royalty-free 2.0 / Commercial (accepted).
- `dds-echo` stays a `tool/` crate (a developer tool with a GUI).
- The mockup is deliberately backend-free: a simulated feed exercises the layout, models, and stats. Later stories add real UDP capture, RTPS/SEDP parsing, and known-type decode.

## Decisions

- **Crate at `rust/tool/dds-echo`**, a Slint app: `build.rs` compiles `ui/dds_echo.slint`; `src/main.rs` wires it; `src/model.rs` holds the data model.
- **Simulated feed via a Slint `Timer`** (100 ms) — no threads; Start/Stop starts and stops it. The capture story replaces the timer with a background thread marshalled through `slint::invoke_from_event_loop`.
- **Plain-Rust data model** (`Record`, `Stats`, `SimFeed`, …) separate from the UI, so it is unit-testable and the later backend plugs in unchanged.
- **Generated Slint types isolated in `mod ui { slint::include_modules!(); }`**, so `ui::Record`/`ui::TopicRow` do not clash with `model::Record`/`model::TopicRow`.
- **Known topics hardcoded** (`KNOWN_TOPICS`) for the mockup; capture/SEDP read `contracts/topics.toml`.
- **Layout**: toolbar (Start/Stop/Clear, inert config fields, a `SIMULATED` badge), a stats strip, a left topics view with collapsible **Known** and **Unknown** groups, a center records table (virtualized), and a **right-hand** detail pane (fields + hex).

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

### `rust/tool/dds-echo/src/model.rs` (new) — UI-agnostic, unit-tested

```rust
pub const MAX_RECORDS: usize = 1000;          // table cap
pub const HEX_PREVIEW: usize = 512;           // bytes shown in the detail pane
pub const KNOWN_TOPICS: &[&str] = &["HelloWorldTopic"];   // hardcoded for the mockup

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Packet, Sample }
impl Kind { pub fn label(self) -> &'static str; }   // "packet" | "sample"

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub elapsed_ms: u64,        // since the feed started
    pub kind: Kind,
    pub source: String,         // e.g. "::1:7401"
    pub dest_port: u16,
    pub size: usize,            // bytes
    pub rtps: bool,             // starts with the RTPS magic
    pub topic: Option<String>,  // Some for a "sample"
    pub summary: String,        // one-line table text
    pub hex: Vec<u8>,           // payload preview
}
impl Record {
    pub fn time_label(&self) -> String;   // "MM:SS.mmm"
    pub fn hex_string(&self) -> String;   // lowercase "xx xx …", first HEX_PREVIEW bytes
    pub fn is_known(&self) -> bool;       // topic in KNOWN_TOPICS
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats { pub packets: usize, pub bytes: usize, pub rtps: usize }
impl Stats {
    pub fn observe(&mut self, r: &Record);   // packets += 1; bytes += size; rtps += rtps as usize
    pub fn reset(&mut self);
}

#[derive(Clone, Debug, PartialEq)]
pub struct TopicRow { pub name: String, pub known: bool, pub count: usize }

#[derive(Clone, Debug, Default)]
pub struct TopicCounts { counts: std::collections::BTreeMap<String, usize> }
impl TopicCounts {
    pub fn observe(&mut self, r: &Record);   // counts a Some(topic)
    pub fn rows(&self) -> Vec<TopicRow>;     // every KNOWN_TOPIC (count 0 if unseen), then unknown topics seen, sorted
    pub fn reset(&mut self);
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rates { pub pps: usize, pub bps: usize }

#[derive(Clone, Debug, Default)]
pub struct RateTracker { prev_packets: usize, prev_bytes: usize, prev_ms: u64 }
impl RateTracker {
    /// Per-second rates since the previous sample; the first sample returns zeros.
    pub fn sample(&mut self, stats: &Stats, now_ms: u64) -> Rates;
}

/// Deterministic xorshift64 PRNG — keeps the feed dependency-free and testable.
#[derive(Clone, Debug)]
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Self;
    pub fn next_u64(&mut self) -> u64;
    pub fn below(&mut self, n: u64) -> u64;            // [0, n)
    pub fn range(&mut self, lo: u64, hi: u64) -> u64;  // [lo, hi]
}

/// Emits RTPS-looking datagrams plus a few HelloWorldTopic samples.
#[derive(Clone, Debug)]
pub struct SimFeed { rng: Rng, seq: u64 }
impl SimFeed {
    pub fn new(seed: u64) -> Self;
    pub fn burst(&mut self) -> usize;            // records for one tick: 1..=4
    pub fn next(&mut self, now_ms: u64) -> Record;
    fn packet(&mut self, now_ms: u64) -> Record;
    fn sample(&mut self, now_ms: u64) -> Record; // topic = "HelloWorldTopic"
}
```

### `rust/tool/dds-echo/ui/dds_echo.slint` (new)

```slint
export struct Record {
    time: string, kind: string, source: string, size: int,
    rtps: bool, known: bool, topic: string, summary: string, hex: string,
}
export struct TopicRow { name: string, known: bool, count: int }

component StatChip inherits Rectangle {   // private helper
    in property <string> label;
    in property <string> value;
    // pill: value (bold) + label (dim)
}

component TopicGroup inherits Rectangle { // private helper
    in property <string> title;
    in-out property <bool> expanded: true;
    in property <[TopicRow]> rows;
    callback toggle();
    // header row (click -> toggle) + `if expanded { ListView { for r in rows: name / count } }`
}

export component MainWindow inherits Window {
    title: "dds-echo";
    preferred-width: 1180px;
    preferred-height: 760px;
    color-scheme: dark;

    // state
    in-out property <bool> running: false;
    in-out property <bool> simulated: true;

    // stats
    in-out property <int> packets: 0;
    in-out property <int> bytes: 0;
    in-out property <int> rtps: 0;
    in-out property <int> pps: 0;
    in-out property <int> bps: 0;
    in-out property <int> participants: 0;
    in-out property <int> endpoints: 0;

    // capture config (present but inert in the mockup)
    in-out property <string> iface: "StationsDDS";
    in-out property <string> group: "ff02::ffff:239.255.0.1";
    in-out property <string> ports: "7400, 7401";

    // models
    in-out property <[Record]> records: [];
    in-out property <[TopicRow]> known-topics: [];
    in-out property <[TopicRow]> unknown-topics: [];

    // selection / detail
    in-out property <Record> selected: {
        time: "", kind: "", source: "", size: 0, rtps: false, known: false, topic: "", summary: "", hex: ""
    };
    in-out property <int> selected-index: -1;

    callback start();
    callback stop();
    callback clear();
    callback select(int);

    // ---- layout ----
    // VerticalLayout (padding 8, spacing 8):
    //  1. toolbar: HorizontalLayout {
    //       Button "Start" (enabled: !running)  -> root.start()
    //       Button "Stop"  (enabled: running)   -> root.stop()
    //       Button "Clear"                       -> root.clear()
    //       TextInput iface / group / ports (read-only)
    //       "SIMULATED" badge  (visible: simulated)
    //     }
    //  2. stats: HorizontalLayout { StatChip x7: Packets/Bytes/RTPS/pps/bps/Participants/Endpoints }
    //  3. main: HorizontalLayout {
    //       left (240px): Text "Topics"; TopicGroup "Known" (rows: known-topics);
    //                     TopicGroup "Unknown" (rows: unknown-topics)
    //       center (fill): header row (Time/Kind/Source/Topic/Size/Summary);
    //                      ListView { for r[i] in records: row (TouchArea -> root.select(i);
    //                                 background highlighted when i == selected-index) }
    //       right (340px): detail fields from `selected`; ScrollView { Text selected.hex (monospace) }
    //     }
}
```

Slint specifics to confirm at implementation: `ListView` row index and the click -> `select(i)` / highlight pattern; whether exported structs derive Rust `Default` (the `selected` initializer provides all fields regardless); style/dark selection via `slint_build`/`SLINT_STYLE` or `color-scheme`.

### `rust/tool/dds-echo/src/main.rs` (new)

```rust
mod model;
mod ui { slint::include_modules!(); }

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{Duration, Instant};

use slint::{Model as _, ModelRc, Timer, TimerMode, VecModel};

use crate::model::{RateTracker, SimFeed, Stats, TopicCounts};
use crate::ui::MainWindow;

const TICK: Duration = Duration::from_millis(100);

/// model::Record -> ui::Record
fn to_ui(r: &model::Record) -> ui::Record;

/// Fill the two topic models from `rows` (known vs unknown).
fn refresh_topics(rows: &[model::TopicRow],
                  known: &VecModel<ui::TopicRow>, unknown: &VecModel<ui::TopicRow>);

/// One timer tick: generate records, update stats, refresh models, set properties.
fn tick(app: &MainWindow, feed: &RefCell<SimFeed>, stats: &RefCell<Stats>,
        counts: &RefCell<TopicCounts>, rates: &RefCell<RateTracker>,
        records: &VecModel<ui::Record>, known: &VecModel<ui::TopicRow>,
        unknown: &VecModel<ui::TopicRow>, start: Instant);

fn main() -> Result<(), slint::PlatformError>;
```

`main` builds `MainWindow`; creates the shared `Rc`/`VecModel`/`Timer` state; seeds the topics view from `KNOWN_TOPICS`; wires `on_start` (set `running`, reset the rate tracker, start the repeated `Timer` calling `tick`), `on_stop` (clear `running`, `timer.stop()`), `on_clear` (clear the models/stats, reset the property values, `selected-index = -1`), and `on_select` (set `selected`/`selected-index` from the non-negative row index); then `app.run()`.

### `rust/Cargo.toml` and `scripts/build.ps1` (changed)

Add member `"tool/dds-echo"`. `build.ps1` deploys `dds-echo.exe` to `build/deploy/bin` and adds `rust\tool\dds-echo\src` to the bypass-scan roots (the tool calls no DDS API).

## Pseudocode

### `model.rs`

```
Kind::label: Packet => "packet"; Sample => "sample"

Record::time_label:
    ms = elapsed_ms
    format!("{:02}:{:02}.{:03}", ms/60000, (ms/1000)%60, ms%1000)

Record::hex_string:
    out = ""; for b in hex[..min(len, HEX_PREVIEW)]: out += format!("{:02x} "); if truncated: out += "…"; out

Record::is_known:
    topic is Some(t) and KNOWN_TOPICS.contains(t)

Stats::observe: packets += 1; bytes += r.size; if r.rtps: rtps += 1
Stats::reset:   *self = default

TopicCounts::observe: if let Some(t) = &r.topic: counts[t] += 1
TopicCounts::reset:   counts.clear()
TopicCounts::rows:
    rows = []
    for name in KNOWN_TOPICS: rows.push(TopicRow { name, known: true, count: counts.get(name).unwrap_or(0) })
    for (name, count) in counts where not KNOWN_TOPICS.contains(name):
        rows.push(TopicRow { name, known: false, count })
    sort rows by (known desc, name); rows

RateTracker::sample(stats, now_ms):
    dt_ms = now_ms - prev_ms
    if prev_ms == 0 or dt_ms == 0: prev_* = stats; prev_ms = now_ms; return Rates { 0, 0 }
    pps = (stats.packets - prev_packets) * 1000 / dt_ms
    bps = (stats.bytes   - prev_bytes)   * 1000 / dt_ms
    prev_packets = stats.packets; prev_bytes = stats.bytes; prev_ms = now_ms
    Rates { pps, bps }

Rng: xorshift64; below(n): next_u64() % n; range(lo, hi): lo + below(hi - lo + 1)

SimFeed::new(seed): rng = Rng::new(seed); seq = 0
SimFeed::burst:     1 + rng.below(4)
SimFeed::next(now_ms):
    seq += 1
    if rng.below(4) == 0: return sample(now_ms)   // ~25% samples
    else:                 return packet(now_ms)

SimFeed::packet(now_ms):
    len = 20 + rng.below(300)
    payload = [ b"RTPS", 2, 4, vendor, 12-byte guid-prefix, ... len bytes ]
    Record { elapsed_ms: now_ms, kind: Packet, source: rand_addr(), dest_port: 7400/7401,
             size: len, rtps: true, topic: None, summary: "RTPS", hex: payload }

SimFeed::sample(now_ms):
    hex = a plausible encoder-header + CDR-ish blob
    Record { elapsed_ms: now_ms, kind: Sample, source: rand_addr(), dest_port: 7401,
             size: hex.len(), rtps: true, topic: Some("HelloWorldTopic"),
             summary: "Hello, world!", hex }
```

### `main.rs`

```
to_ui(r):
    ui::Record {
        time: r.time_label(), kind: r.kind.label(), source: r.source, size: r.size as i32,
        rtps: r.rtps, known: r.is_known(), topic: r.topic.clone().unwrap_or_default(),
        summary: r.summary, hex: r.hex_string(),
    }

refresh_topics(rows, known, unknown):
    known.clear(); unknown.clear()
    for row in rows: push onto (row.known ? known : unknown) as ui::TopicRow { name, known, count: count as i32 }

tick(app, feed, stats, counts, rates, records, known, unknown, start):
    now_ms = start.elapsed().as_millis() as u64
    for _ in 0..feed.borrow_mut().burst():
        rec = feed.borrow_mut().next(now_ms)
        stats.borrow_mut().observe(&rec)
        counts.borrow_mut().observe(&rec)
        records.push(to_ui(&rec))
        while records.row_count() > MAX_RECORDS: records.remove(0)
    refresh_topics(&counts.borrow().rows(), known, unknown)
    r = rates.borrow_mut().sample(&stats.borrow(), now_ms)
    s = stats.borrow()
    app.set_packets(s.packets as i32); app.set_bytes(s.bytes as i32); app.set_rtps(s.rtps as i32)
    app.set_pps(r.pps as i32); app.set_bps(r.bps as i32)
    // participants/endpoints stay 0 until the SEDP story

main:
    app = MainWindow::new()?
    create shared state: feed, stats, counts, rates, records, known, unknown, start (Cell<Instant>), timer (Rc<Timer>)
    app.set_records(ModelRc::from(records.clone())); likewise known/unknown
    refresh_topics(&counts.borrow().rows(), &known, &unknown)          // seed the Known group
    on_start: upgrade weak; set running; start.set(now); reset rates; timer.start(Repeated, TICK, tick-closure)
    on_stop:  upgrade weak; set running false; timer.stop()
    on_clear: upgrade weak; clear models; reset stats/counts/rates; refresh_topics; set stat properties 0; selected-index = -1
    on_select(i): if i >= 0 and records.row_data(i) is Some(r): set selected = r; selected-index = i
    app.run()
```

Note: the `Timer` callback, `on_clear`, and `on_select` closures capture `slint::Weak<MainWindow>` (upgraded per invocation) plus the shared `Rc` handles, so there are no reference cycles.

## Tests

`src/model.rs` `#[cfg(test)]` (no UI automation):

- `SimFeed::next` returns a record with non-empty `hex`; RTPS records start with `b"RTPS"`; over ~100 draws at least one `Sample` has `topic == Some("HelloWorldTopic")`.
- `Stats::observe` accumulates `packets`/`bytes`/`rtps`.
- `TopicCounts::rows` lists `HelloWorldTopic` as `known == true` with count 0 before any sample, then counts samples; an unknown topic appears with `known == false`.
- `RateTracker::sample` returns zeros on the first call and correct per-second rates for a synthetic step.
- `Record::hex_string` is lowercase pairs and truncates at `HEX_PREVIEW`.
- `Rng` is deterministic for a fixed seed.

## Minimal test

```
.\scripts\build.ps1
build\deploy\bin\dds-echo.exe
```

The window opens; Start fills the stats, the Known/Unknown topics groups, and the table from the simulated feed; Stop halts it; selecting a row shows its fields and hex; the groups collapse; Clear resets.

## Risks and follow-ups

- **Toolchain.** Slint 1.19 requires rustc >= 1.95; verify the pinned `stable-x86_64-pc-windows-msvc` is current.
- **Renderer.** Slint defaults to winit + FemtoVG (OpenGL), but Slint owns the window and context — no manual GL plumbing.
- **Event loop.** Slint's loop is main-thread; the capture story must update models from its background thread via `slint::invoke_from_event_loop`.
- **Slint API details** (Timer signature, `ListView` selection, struct `Default`, style/dark API) are confirmed when writing the code.
- **Build weight.** Slint pulls a sizable dependency tree; the first build is slow.
- **Mockup data.** `KNOWN_TOPICS` and the simulated samples are placeholders; capture/SEDP replace them with the registry and real traffic.

## Module layout

```
rust/tool/dds-echo/build.rs          Slint compile
rust/tool/dds-echo/ui/dds_echo.slint the UI
rust/tool/dds-echo/src/main.rs       module ui, wiring, timer
rust/tool/dds-echo/src/model.rs      Kind/Record/Stats/TopicCounts/RateTracker/Rng/SimFeed + tests
```

## Result

Implemented. `rust/tool/dds-echo` is a Slint GUI app (`build.rs` + `ui/dds_echo.slint` + `src/main.rs` + `src/model.rs`) driven by a simulated feed: toolbar (Start/Stop/Clear, inert config fields, `SIMULATED` badge), stats strip, a left Known/Unknown collapsible topics view, a virtualized records table, and a right detail/hex pane. `scripts/build.ps1` builds it, deploys `dds-echo.exe` to `build/deploy/bin`, and adds its `src` to the bypass-scan roots.

Verified: `scripts/build.ps1 -Test` passes; `dds-echo`'s 6 `model.rs` tests pass (feed yields RTPS + samples, stats/topic-count/rate aggregation, hex formatting, RNG determinism); the release build and deploy succeed; the deployed `dds-echo.exe` launches and stays running. The visual minimal test (Start populates the UI, Stop/Clear/select behave) remains for the user to confirm.

Deviations from the plan: pinned **Slint `1.18`** (the newest published; `1.19` is unreleased); imported the standard widgets (`Button`, `LineEdit`, `ListView`, `ScrollView` from `std-widgets.slint`) and used `LineEdit` (not `TextInput`); `RateTracker` carries an explicit `initialized` flag (so a first sample at `now_ms == 0` is handled); added `slint::ComponentHandle` to the imports.

Changes after review:
- **Classes** replace `kind`: `UDP` / `RTPS` / `unregistered` / `known` (`model::Class`), color-coded in the table. "Decode failed" is a detail-pane state, not a class. The feed emits all four.
- **`source`** is the sender address (`host:port`) in the table; the GUID prefix and entity id are shown only in the Detail pane.
- **Follow the tail**: the records `ListView` was replaced by a `ScrollView` + `VerticalLayout { for … }` (a `ListView`'s runtime-managed `content-y` made the binding unreliable). A `following` flag (starting true) is toggled by the ScrollView's `scrolled` callback — which fires only on user scroll, not on programmatic scroll — and each tick scrolls to `-(content-height − visible-height)` while following. So it follows from start, pauses when scrolled up, and resumes at the end.
- **Topics list**: the group headers show the group's **packet total** in the parentheses; each row shows its per-topic packet count (known and unregistered).
- **Config fields** (`iface`/`group`/`ports`) are editable only while stopped (`read-only: root.running`).
- **Status badge**: the `SIMULATED` badge became a capture-status indicator — **Live** (green) while running, **Stopped** (gray) otherwise.

## Files

- Authored: `rust/tool/dds-echo/{Cargo.toml,build.rs,ui/dds_echo.slint,src/main.rs,src/model.rs}`.
- Changed: `rust/Cargo.toml`, `scripts/build.ps1`, `planning/{architecture,backlog,current_story}.md`, optionally `README.md`.
