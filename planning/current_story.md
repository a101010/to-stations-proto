# Current story: hello-view-service

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add `HelloWorldViewTopic`, create the hello_world view service (`hello-view-service`) that publishes it, and make the display library render the greeting/attitude it receives (moving the model data out of the library). The display library renders black until it receives a sample.

- **Depends on:** none.
- **Minimal test:** `launch-rust.ps1` starts the view service and `lil-d`; `dds-echo` shows `HelloWorldViewTopic` samples and the display renders the published view.

## Context

- The hello-world format (`display-lib/src/formats/hello_world.rs`) hardcodes `GREETING` and computes a fixed `view_attitude()` (faces North America, north up); `lil-d` renders only on window events, and the library has no DDS.
- `stations-dds` provides `Node::new(domain, registry)`, `writer::<T>(topic)`, `reader::<T>(topic)`, and `contract::{TopicRegistry, ServiceContract, known_types}`; the registry is validated against `known_types()` and the IDL (`stations-dds/tests/registry.rs`).
- `include/display_lib.h` is version 2; the display library is a `cdylib` loaded by `lil-d` and resolves assets from `<exe_dir>/../assets`. No ABI change is needed (the library subscribes itself).
- The display application sets `CYCLONEDDS_URI`? No - the launch script must; CycloneDDS otherwise uses defaults (not the `StationsDDS` adapter).

## Decisions

- **`HelloWorldViewTopic`** is an exact copy of `HelloWorldTopic` (`greeting: string`, `attitude: math::Quaternion`); the rotation is carried by the attitude, not a new field.
- **Topic/registry/QoS:** registered `qos = "best_effort"` (100 Hz view stream); added to `known_types()` so validation and the completeness test pass.
- **Initial attitude (permanent axial tilt):** the earth's polar axis (+Y) is tilted **23.5 deg** from view-up toward `d = normalize([-1, 0, 1])` (equally toward the viewer +Z and to the left -X); the roll is chosen so the initial sub-camera longitude is ~100 deg W (the Americas face the camera; the centered latitude falls out at ~16 deg N). The earth then spins **about that tilted polar axis** (local +Y) in the direction of Earth's real rotation (eastward), one revolution per **12 minutes**.
- **View service** publishes `HelloWorldViewTopic` at **100 Hz, best-effort**; its data is hardcoded here for now (`TODO: from hello-service`, a later story).
- **Display library** subscribes to `HelloWorldViewTopic` on **domain 0** using the registry; it stores the latest `View { greeting, attitude }` in shared state and renders black until one arrives. No hardcoded greeting/attitude remains.
- **Config** is deployed under `<exe_dir>/../config` (`topics.toml`, `services/*.toml`, `cyclonedds-config.xml`); the service and library locate it there (falling back to `REPO_ROOT/contracts` for `cargo run`).
- **`lil-d` renders continuously** so the library polls DDS each frame.
- **`dds-echo` decodes `HelloWorldViewTopic`** too.

## Interfaces

### `DDS/hello_world.idl` (changed)
```idl
module hello_world {
  struct HelloWorldTopic { string greeting; math::Quaternion attitude; };
  struct HelloWorldViewTopic { string greeting; math::Quaternion attitude; };
};
```

### `contracts/topics.toml` (changed)
```toml
[[topics]]
name = "HelloWorldViewTopic"
type = "hello_world::HelloWorldViewTopic"
qos = "best_effort"
key = []
```

### `stations-dds/src/contract.rs` (changed)
- `known_types()` -> add `"hello_world::HelloWorldViewTopic"`.

### `contracts/services/hello-view-service.toml` (new)
```toml
[service]
name = "hello-view-service"
[[publish]]
topic = "HelloWorldViewTopic"
```

### `rust/service/hello-view-service/` (new crate; workspace member)
- `src/main.rs`:
  - `Args`: `--domain` (0), `--rate-ms` (10), `--config <dir>` (default `<exe_dir>/../config`, falling back to `REPO_ROOT/contracts`), `--topics`.
  - Load `topics.toml` + `services/hello-view-service.toml`; `registry.validate()`, `contract.validate(&registry)`; `Node::new(domain, registry)`; `writer::<HelloWorldViewTopic>("HelloWorldViewTopic")`.
  - `--topics`: print `contract.render()` and exit.
  - Loop: every `rate_ms`, `writer.write(&HelloWorldViewTopic { greeting, attitude: view::attitude_at(now) })`.
- `src/view.rs`:
  - Quaternion helpers (`from_axis_angle`, `mul`) over `math::Quaternion` (f64).
  - `initial_attitude() -> math::Quaternion` (R0) and `attitude_at(elapsed: Duration) -> math::Quaternion` = `R0 * spin(theta)`, `theta = 2*pi * elapsed / 720s`.

### `rust/lib/display-lib/src/` (changed)
- `Cargo.toml`: add `stations-dds = { workspace = true }`.
- `src/view.rs` (new):
  ```rust
  pub(crate) struct View { pub greeting: String, pub attitude: [f32; 4] }
  pub(crate) type ViewState = std::rc::Rc<std::cell::RefCell<Option<View>>>;
  pub(crate) struct ViewSource { _node: Node, reader: DataReader<HelloWorldViewTopic> }
  impl ViewSource {
      pub(crate) fn new(domain: u32, config_dir: &Path) -> Result<Self, String>;
      pub(crate) fn poll(&self, state: &RefCell<Option<View>>); // take() -> update state
  }
  ```
  - Registry from `<config_dir>/topics.toml`; on failure, `Display::new` keeps `view = None`.
- `src/lib.rs`: `Display` gains `view: Option<ViewSource>` and `state: ViewState`; `Display::new` builds the format manager with `state.clone()` and best-effort `ViewSource::new`; `Display::render` polls before rendering.
- `src/formats.rs`: `registry(gl, assets_root, state)` passes the state to `HelloWorld`.
- `src/formats/hello_world.rs`: drop `GREETING`, `view_attitude()`, and the `attitude` field; hold `state: ViewState`; on render, if `state` is `None`, clear black and return; else use `view.greeting`/`view.attitude`. Light stays hardcoded.

### `rust/ui/lil-d/src/app.rs` (changed)
- Request a redraw every frame (`about_to_wait`), so the display library polls DDS continuously.

### Scripts (changed)
- `scripts/build.ps1`: deploy `build/rust/release/hello-view-service.exe`; copy `contracts/topics.toml` -> `build/deploy/config/topics.toml` and `contracts/services/` -> `build/deploy/config/services/`; add `rust\service\hello-view-service\src` to the bypass-scan roots.
- `scripts/launch-rust.ps1`: set `$env:CYCLONEDDS_URI` to the deployed `config/cyclonedds-config.xml`; start `hello-view-service.exe`; then start `lil-d.exe --station N`.
- `scripts/cleanup.ps1`: add `hello-view-service` to the stopped images.

### `rust/tool/dds-echo/src/decode.rs` (changed)
- Add a `"hello_world::HelloWorldViewTopic"` arm (same deserialization as `HelloWorldTopic`).

### `rust/Cargo.toml` (changed)
- `members` += `service/hello-view-service`.

## Pseudocode

```
// service loop (100 Hz)
t = start.elapsed()
attitude = initial_attitude() * quat_from_axis_angle([0,1,0], 2*pi * t/720s)
writer.write(&HelloWorldViewTopic { greeting: "Hello, world!", attitude })
```

```
// display-lib render
if let Some(view) = &self.view { view.poll(&self.state) }   // reader.take() -> *state = Some(View{..})
self.format_manager.render(...)                            // hello-world: black when state is None
```

## Tests

- `hello-view-service/src/view.rs`: the polar axis is 23.5 deg from +Y toward `d`; the initial sub-camera longitude is ~-100 deg; `attitude_at(6 min)` differs from `attitude_at(0)` by a 180 deg rotation about the polar axis.
- `display-lib`: `ViewState` defaults to `None`; the pure asset/format tests are unchanged (no DDS in tests). `registry.rs` completeness passes with the new type.
- `dds-echo`: `decode` of a `CdrSerializer`-serialized `HelloWorldViewTopic` returns the fields.

## Minimal test

```
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-rust.ps1
# then: run build\deploy\bin\dds-echo.exe and observe HelloWorldViewTopic samples;
# the lil-d region shows the tilted, slowly spinning earth with the greeting.
.\scripts\cleanup.ps1
```

## Risks and follow-ups

- The display library now links CycloneDDS (static) and needs `config/`; run it from `build/deploy` (via the launch script) or the format stays black.
- 100 Hz best-effort is a high-rate stream; the display polls at frame rate and renders the latest received sample.
- The tilt axis/order and the initial roll are visual tuning; the tests pin the pole angle, the spin rate, and the initial longitude.
- Continuous rendering in `lil-d` costs CPU/GPU; acceptable for a display.
- Per-station views and the model row (`hello-service`) come later; `hello-view-service` holds the data until then.

## Module layout

```
DDS/hello_world.idl                      + HelloWorldViewTopic
contracts/topics.toml                    + HelloWorldViewTopic (best_effort)
contracts/services/hello-view-service.toml  publish HelloWorldViewTopic
stations-dds/src/contract.rs             known_types() += the new type
rust/service/hello-view-service/         main.rs, view.rs (attitude math)
rust/lib/display-lib/src/{view.rs,lib.rs,formats.rs,formats/hello_world.rs}
rust/ui/lil-d/src/app.rs                 continuous render
rust/tool/dds-echo/src/decode.rs         + HelloWorldViewTopic arm
scripts/{build,launch-rust,cleanup}.ps1
```
