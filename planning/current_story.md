# Current story: dds-echo-capture

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Replace the `dds-echo` mockup's simulated feed with the real UDP capture backend: a multicast/broadcast/unicast listener with `--interface`/`--group`/`--port` (and `--domain`) filters, RTPS detection, and a live hex listing. The toolbar fields drive the capture; the known-topics view reads `contracts/topics.toml`.

- **Depends on:** dds-echo.
- **Minimal test:** with a publisher running, capture shows RTPS-flagged datagrams with their hex; unit tests cover `is_rtps`, config parsing, and a loopback receive.

## Context

- `dds-echo` is a Slint app (`build.rs` + `ui/dds_echo.slint` + `src/main.rs` + `src/model.rs`) whose `Timer` currently emits `SimFeed` records. `model::Record` already carries `class`, `src_addr`, `guid_prefix`, `entity_id`, `dest_port`, `size`, `topic`, `summary`, `hex`, `decoded`; `Class` is `Udp`/`Rtps`/`Unregistered`/`Known`.
- Slint models must be updated on the UI thread, so capture runs off-thread and feeds the UI through a channel drained by the existing `Timer`.
- Our `DDS/cyclonedds-config.xml` sets `<AllowMulticast>true</AllowMulticast>`, so endpoint discovery (and often user data) is multicast; CycloneDDS's default participant-discovery group is `ff02::ffff:239.255.0.1` (IPv6) on port `7400 + 250·domain`.

## Decisions

- **New `capture` module** (sockets + classification); UI-agnostic and testable.
- **Dependencies:** `socket2` (SO_REUSEADDR), `if-addrs` (interface name → scope id / addresses), `clap` (CLI defaults).
- **Config from the toolbar fields.** CLI (`--interface`, `--group`, `--port`, `--domain`) seeds the initial field values; **Start** parses the fields and starts capture; **Stop** stops it.
- **Threading:** one capture thread per socket → `std::sync::mpsc` channel → the Slint `Timer` drains on the UI thread; sockets use a read timeout to poll a stop flag.
- **Classification now:** `Class::Udp` (no `RTPS` magic) or `Class::Rtps` (RTPS, topic not yet identified). `unregistered`/`known` arrive with `dds-echo-sedp`/`-decode`; the topics view lists the registry's known topics at 0 packets.
- **Remove `SimFeed` and `Rng` entirely** from the model.
- **Start failure → error dialog.** `MainWindow` gains `error-message: string` and a modal overlay with an OK button; `on_start` sets it when the config or `Capture::start` fails.
- **Traffic-generator scripts.** Commit three dev scripts under `scripts/` so the capture can be stimulated: `traffic-roundtrip.ps1` (runs the `stations-dds` round_trip test), `traffic-throughput.ps1` (runs the CycloneDDS `ThroughputSubscriber` + `ThroughputPublisher` examples), and `traffic-listtopics.ps1` (runs the `listtopics` example). Each resolves the repo root from `$PSScriptRoot` and the external prefix, points the example at `DDS/cyclonedds-config.xml` via `CYCLONEDDS_URI`, and makes `ddsc.dll` available.

## Interfaces

### `rust/tool/dds-echo/src/capture.rs` (new)

```rust
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::model::{Class, Record};

pub enum IfSpec { Name(String), Index(u32) }

pub struct CaptureConfig {
    pub interfaces: Vec<IfSpec>,
    pub groups: Vec<IpAddr>,
    pub ports: Vec<u16>,
    pub broadcast: bool,
}

impl CaptureConfig {
    /// Group `ff02::ffff:239.255.0.1`, ports `7400 + 250*domain` and `+1`.
    pub fn for_domain(domain: u32) -> Self;
    /// Parse the toolbar fields: `iface` (names/indices, comma-separated),
    /// `group` (IPs), `ports` (comma/space-separated u16).
    pub fn from_fields(iface: &str, group: &str, ports: &str) -> Result<Self, ConfigError>;
}

#[derive(Debug)]
pub enum ConfigError {
    BadInterface(String),
    BadGroup(String),
    BadPort(String),
    Io(String),
}

pub fn is_rtps(data: &[u8]) -> bool;                 // data.starts_with(b"RTPS")
pub fn classify(data: &[u8]) -> Class;               // Rtps | Udp
pub fn record_from(data: &[u8], src: SocketAddr, local_port: u16, elapsed_ms: u64) -> Record;

/// Binds and joins; spawns one thread per socket, sending records to `tx`.
pub struct Capture {
    stop: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
    sockets: Vec<UdpSocket>,     // kept open for the threads' lifetime
}
impl Capture {
    pub fn start(config: &CaptureConfig, tx: Sender<Record>) -> Result<Self, std::io::Error>;
    pub fn stop(self);
}
```

### `rust/tool/dds-echo/src/model.rs` (changed)

- Remove `SimFeed` and `Rng` (and their tests). Keep `Class`, `Record`, `Stats`, `TopicCounts`, `RateTracker`, `KNOWN_TOPICS`, `MAX_RECORDS`, `HEX_PREVIEW`.

### `rust/tool/dds-echo/src/main.rs` (changed)

```rust
mod capture;

#[derive(clap::Parser)]
struct Args {
    #[arg(long, default_value = "StationsDDS")] interface: String,
    #[arg(long, default_value = "")] group: String, // empty -> from --domain
    #[arg(long, default_value = "")] ports: String, // empty -> from --domain
    #[arg(long, default_value_t = 0)] domain: u32,
}

struct State {
    capture: RefCell<Option<capture::Capture>>,
    rx: RefCell<Option<std::sync::mpsc::Receiver<model::Record>>>,
    stats: RefCell<Stats>,
    counts: RefCell<TopicCounts>,
    rates: RefCell<RateTracker>,
    start: Cell<Instant>,
    following: Cell<bool>,
}
```

- `main`: parse `Args`; seed `ui.set_iface/group/ports` (group/ports from `--domain` when empty).
- `on_start`: `CaptureConfig::from_fields(...)` → `Capture::start(cfg, tx)` → store `capture`/`rx`; `set_running(true)`; reset rates; `following = true`; start the `Timer`. On any error, `app.set_error_message(...)`.
- `on_stop`: `take()` the capture and `stop()` it; clear `rx`; `set_running(false)`.
- `on_dismiss_error`: `app.set_error_message("".into())`.
- `on_clear`: unchanged.
- `tick`: drain `rx` (`try_recv` loop) → `stats.observe`/`counts.observe`/`records.push(to_ui(&rec))` (cap at `MAX_RECORDS`); `refresh_topics`; set stat properties; follow-the-tail.

### `rust/tool/dds-echo/ui/dds_echo.slint` (changed)

- Add `in-out property <string> error-message: "";` and `callback dismiss-error();`.
- Add a modal overlay after the root layout:
  ```slint
  if (root.error-message != "") : Rectangle {
      background: #000000a0;
      Rectangle {
          x: (parent.width - self.width) / 2;
          y: (parent.height - self.height) / 2;
          width: 360px; height: 150px;
          background: #2a2a2a; border-radius: 8px;
          VerticalLayout {
              padding: 12px; spacing: 8px;
              Text { text: "Capture error"; color: #ff8080; font-weight: 700; }
              Text { text: root.error-message; color: #e0e0e0; wrap: word-wrap; vertical-stretch: 1; }
              HorizontalLayout {
                  Rectangle { horizontal-stretch: 1; }
                  Button { text: "OK"; clicked => { root.dismiss-error(); } }
              }
          }
      }
  }
  ```

### `rust/tool/dds-echo/Cargo.toml` and `rust/Cargo.toml` (changed)

Add `socket2`, `if-addrs`, `clap = { workspace = true }`; the crate also has a `windows-sys` dependency (for interface resolution).

### `scripts/traffic-{roundtrip,throughput,listtopics}.ps1` (new)

Dev scripts that generate traffic for the capture. Each resolves the repo root from `$PSScriptRoot`, sets `CYCLONEDDS_URI` to `DDS/cyclonedds-config.xml`, and (for the example binaries) prepends the CycloneDDS example bin dir (`$EXT_PATH_PREFIX\src\cyclonedds\build\bin\Release`, default `C:\Libraries`) to `PATH`.

- `traffic-roundtrip.ps1 [-Repeat N]` — runs `cargo test -p stations-dds --test round_trip` from the repo root.
- `traffic-throughput.ps1 [-Seconds 30] [-Payload 1024] [-IntervalMs 100] [-Burst 10]` — starts `ThroughputSubscriber.exe` in the background and runs `ThroughputPublisher.exe <payload> <interval> <burst> <seconds>` (the publisher waits for a matched reader).
- `traffic-listtopics.ps1` — runs `listtopics.exe` (a ~10 s discovery participant).

## Pseudocode

### `Capture::start`

```
resolved = for each IfSpec: if-addrs -> { v6_index: Option<u32>, addresses: Vec<IpAddr> }
sockets = {}
for port in config.ports:
    families = { v6 if any group is v6 or config uses v6; v4 if any group is v4 or broadcast }
    for family in families:
        s = socket2::Socket::new(family, DGRAM, UDP); s.set_reuse_address(true)
        s.bind(wildcard(family):port)          // v6: [::]:port, v4: 0.0.0.0:port
        s.set_broadcast(config.broadcast)
        udp = UdpSocket::from(s); udp.set_read_timeout(200ms)
        for group in config.groups where group.family == family:
            udp.join_multicast(group, if_index(resolved, group))
        sockets.push((udp, port))
stop = Arc::new(AtomicBool::new(false))
threads = for (udp, port) in sockets:
    let stop = stop.clone(); let tx = tx.clone()
    spawn(move || {
        let epoch = Instant::now(); let mut buf = [0u8; 65535]
        loop {
            match udp.recv_from(&mut buf) {
                Ok((n, src)) => { let _ = tx.send(record_from(&buf[..n], src, port, epoch.elapsed().as_millis() as u64)); }
                Err(e) if e.kind() in {WouldBlock, TimedOut} => if stop.load(Relaxed) { break },
                Err(other) => { eprintln!("capture: {other}"); break }
            }
        }
    })
Capture { stop, threads, sockets }
```

### `CaptureConfig::from_fields`

```
interfaces = iface.split(',') -> trim -> Index if all digits else Name
groups = group.split(&[',', ' ']) -> IpAddr::from_str
ports  = ports.split(&[',', ' ']) -> u16::from_str
(any empty group/ports: use for_domain defaults)
```

### `record_from`

```
class = classify(data)
Record { class, src_addr: src.to_string(), dest_port: local_port, size: data.len(),
         hex: data.to_vec(),
         summary: if is_rtps(data) { "RTPS" } else { "UDP" }.to_string(),
         topic: None, guid_prefix: None, entity_id: None, decoded: String::new(), elapsed_ms }
```

## Tests

- `capture.rs` unit (no sockets): `is_rtps`/`classify` (magic vs not); `CaptureConfig::for_domain(0)` → group `ff02::ffff:239.255.0.1`, ports `{7400, 7401}`; `from_fields` parses names/indices, IPs, ports, and rejects bad input; `record_from` sets `class`/`size`/`hex`/`src_addr`/`summary`.
- `capture.rs` integration (adapter-free): bind a capture on `127.0.0.1:0` (plain unicast, no group) and send an RTPS-magic datagram and a non-RTPS one from a test socket; assert the received `Record`s have the right `Class`, `size`, and `hex`.
- `model.rs`: keep the stats / topic-count / rate / hex tests; drop the `SimFeed`/`Rng` tests.

## Minimal test

```
.\scripts\build.ps1 -Test
.\scripts\launch-rust.ps1   # or run build\deploy\bin\dds-echo.exe alongside a publisher
```

With a publisher running, `dds-echo` shows RTPS-flagged datagrams and their hex; Stop halts capture; the Live/Stopped badge tracks the state.

## Risks and follow-ups

- **Windows multicast:** the join needs the interface scope id (`if-addrs` resolves `StationsDDS`); the adapter must exist and be up.
- **Stop:** blocking `recv_from` needs the read timeout (200 ms) to observe the stop flag; `Capture::stop` sets the flag and joins.
- **Unicast** is only observed for ports we bind; multicast/broadcast otherwise.
- **UI-thread updates:** models are only touched in `tick` (via the channel), never from capture threads.
- **Sockets that fail to bind/join** surface as a start error in the dialog; partial binds of a multi-socket config are a later refinement.

## Module layout

```
rust/tool/dds-echo/build.rs          Slint compile
rust/tool/dds-echo/ui/dds_echo.slint the UI (+ error dialog)
rust/tool/dds-echo/src/main.rs       module capture, clap args, wiring, tick
rust/tool/dds-echo/src/capture.rs    sockets, config, classification, Capture
rust/tool/dds-echo/src/model.rs      Class/Record/Stats/TopicCounts/RateTracker + tests
```

## Result

Implemented. The simulated feed is gone; `dds-echo` captures real UDP. `src/capture.rs` (sockets, config, classification) binds `socket2` sockets (SO_REUSEADDR, `IPV6_V6ONLY`), joins multicast groups on the resolved interfaces, and runs one receive thread per socket feeding an `mpsc` channel; the Slint `Timer` drains it on the UI thread. Toolbar fields (fed from `clap` `--interface`/`--group`/`--port`/`--domain`) drive capture on **Start**; **Stop** joins the threads; a start failure shows an in-window error dialog. `model.rs` lost `SimFeed`/`Rng`.

Interface resolution note: neither `if-addrs` (it filters to `IpDadStatePreferred`, dropping the KM-TEST adapter) nor `if_nametoindex` (it rejects Windows friendly names) resolves `StationsDDS`, so the IPv6 scope id comes from `GetAdaptersAddresses` with `GAA_FLAG_INCLUDE_ALL_INTERFACES` (via `windows-sys`), with `if-addrs` still supplying IPv4 addresses.

Verified: `scripts/build.ps1 -Test` passes — 10 `dds-echo` tests, including an adapter-free loopback unicast receive and an adapter multicast receive on `ff02::ffff:239.255.0.1:7400`; the release build and deploy succeed; the deployed `dds-echo.exe` launches and joins the group with no error. The visual minimal test (Start shows live RTPS datagrams with hex; Stop halts; the Live/Stopped badge tracks state) remains for the user.

Deviations: dropped the `ConfigError::Io` variant; `Capture::start` returns `io::Result` and `local_ports()`/the `sockets` field are `#[allow(dead_code)]` (used only by tests); the `Class::Unregistered`/`Known` variants carry `#[allow(dead_code)]` until `dds-echo-sedp`/`-decode`.

Traffic generators: `scripts/traffic-{roundtrip,throughput,listtopics}.ps1` were added and verified — `traffic-throughput.ps1 -Seconds 5` generated traffic (the publisher wrote samples) and `dds-echo` captured its RTPS datagrams on `StationsDDS`.

Capture UI fixes after review: the detail pane's hex `Text` is now bounded to the viewport and top/left-aligned, so it wraps and shows the whole (capped) packet instead of a clipped single line (`content-width: hexscroll.visible-width` in `ui/dds_echo.slint`); `HEX_PREVIEW` was raised to 4096 with a `… (+N bytes)` truncation note; and `model::TopicCounts` counts datagrams with no identified topic so the topics view's **Unknown** group header matches the stats' packet total (an `unidentified` row is shown). All four `dds-echo` classes remain, but DDS traffic is all `RTPS` until `dds-echo-sedp` identifies topics.

Further fixes: `hex_string()` now **pre-wraps** at 16 bytes per line (`HEX_BYTES_PER_LINE`), rendered with `wrap: no-wrap; horizontal-alignment: left; vertical-alignment: top; font-size: 12px` — deterministic and stable across window resizes because the detail pane is fixed-width. `dds-echo` sets `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` so the release build has no console window (debug keeps it). And `DDS/cyclonedds-config.xml` gains `<AddrsetCosts><mc>1</mc><uc>4</uc></AddrsetCosts>` so CycloneDDS's locator cost model **prefers multicast for user data** (its defaults `mc=3`/`uc=2` send a lone reader's data by unicast). Verified: with `traffic-throughput.ps1 -Seconds 6`, `dds-echo` captured 138 datagrams — 132 on the multicast user-data port (7401) plus 6 discovery — versus discovery-only before.

Hex review, take two: the pre-wrapped hex was still clipped at both ends because Slint **centers** an implicit-size element that is outside a layout (the `Text` was a direct `ScrollView` child wider than the pane), so the fix is (a) put the `Text` in a `HorizontalLayout` with a trailing stretch spacer to left-align it, and (b) widen the detail pane so a 16-byte line (~338px at 12px) fits. The pane first went `340px → 420px`, which left ~3 byte-pairs of slack on the right, then was tightened to **356px** (`min/max-width`) to remove that slack; 356px clipped the last digit, so it was nudged to **361px** (+5px) to fit. The window default grew to `1900x1040` (just under 1080p) to keep the records pane wide; `HEX_BYTES_PER_LINE` stays 16.

Throughput count is **not** data loss: with `ThroughputPublisher 1024 100 10 6` the publisher wrote 560 samples and the subscriber received 560 samples / 577,920 B (0 out of order), while `dds-echo` captured 120 datagrams / 609,420 B — *more* bytes than the subscriber, because CycloneDDS coalesces ~5 samples per datagram. `dds-echo` counts **datagrams**, not samples, so its total is expected to be lower; documented in `README.md`. No `SO_RCVBUF` change was needed. Sample-level counting belongs to `dds-echo-sedp`.

## Files

- Authored: `rust/tool/dds-echo/src/capture.rs`, `scripts/traffic-roundtrip.ps1`, `scripts/traffic-throughput.ps1`, `scripts/traffic-listtopics.ps1`.
- Changed: `rust/tool/dds-echo/{Cargo.toml,src/main.rs,src/model.rs,ui/dds_echo.slint}`, `rust/Cargo.toml`, `planning/{backlog,current_story}.md`, `docs/versions.md` (`socket2`, `if-addrs`, `windows-sys`).
