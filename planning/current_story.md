# Current story: dds-and-service-contracts

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Define the DDS layer and the service contracts in `stations-dds`: create the participant from `DDS/cyclonedds-config.xml` and typed readers/writers; define the topic-contract schema and loader for `contracts/topics.toml` and `contracts/services/<app>.toml`; validate each service's declared topics at startup; support a `--topics` flag that prints a service's contract; and write a readable summary to `build/topics.md`.

- **Depends on:** none.
- **Minimal test:** a typed publish/subscribe round-trip; a sample contract loads, validates, prints with `--topics`, and matches `build/topics.md`.

## Context

* `stations-dds` currently only re-exports `cyclonedds`, exposes `type_name::<T>()`, and `include!`s the generated IDL (`lib.rs:16`). The generated types `hello_world::HelloWorldTopic` and `math::Quaternion` exist.
* The pinned `cyclonedds` fork exposes `DomainParticipant::new/with_qos`, `create_topic::<T>(name)`, `create_publisher/subscriber`, `create_writer[_with_qos]::<T>`, `create_reader[_with_qos]::<T>`, `DataWriter::write`, `DataReader::read/take/read_next/take_next`, `Qos::builder()` (`reliable`/`best_effort`/`transient_local`/`keep_last`), and `WaitSet`/`ReadCondition`.
* CycloneDDS applies `DDS/cyclonedds-config.xml` through the `CYCLONEDDS_URI` environment variable (`docs/loopback-adapter.md`); participant creation itself takes no config argument.
* `contracts/` holds only `.gitkeep`; `build.ps1` does not yet generate `build/topics.md` or deploy the config.
* Architecture (`architecture.md:74`) fixes the contract shape: `contracts/topics.toml` (topic name, IDL type, QoS, key fields) and `contracts/services/<app>.toml` (publish/subscribe); validated at startup; `--topics`; `build/topics.md`. `rust/tool/dds-contracts` is already named in the folder structure.

## Decisions

* **`Node` DDS wrapper** (`stations-dds::dds`) over `DomainParticipant`, with typed `writer`/`reader` helpers and a QoS-by-name map. A temporary `Publisher`/`Subscriber` per endpoint is fine: the binding's guards keep them alive for the writer/reader's lifetime.
* **Config via `CYCLONEDDS_URI`.** The layer creates the participant and documents that CycloneDDS reads the shared config from `CYCLONEDDS_URI`; the round-trip test sets it to `<REPO_ROOT>/DDS/cyclonedds-config.xml`, and the launch scripts will point at the deployed `build/deploy/config/`. `build.ps1` begins copying the config to `build/deploy/config/`.
* **Topic names are short** (`HelloWorldTopic`) for the registry and the wire; the scoped IDL/DDS type (`hello_world::HelloWorldTopic`) is recorded separately in the registry as `type`. Validation requires `type` to equal a known `DdsType::type_name()`.
* **Contracts** live in `stations-dds::contract` (schema, loader, validation, `render`), using `serde` + `toml`.
* **`dds-contracts` tool** (`rust/tool/dds-contracts`) loads the repo contracts, validates them, writes `build/topics.md`, and prints a service with `--topics <app>`. It is a developer tool, not deployed (like `equirect-to-cubemap`).
* **`--topics` and startup validation are helpers.** `ServiceContract::render()` is exposed by the tool now and called by real services later; deploying contracts to `build/deploy/config/` for runtime service loading is a follow-up.
* **The round-trip is a committed integration test** that runs in `cargo test` (and `build.ps1 -Test`); it requires the `StationsDDS` adapter, an existing environment prerequisite.

## Interfaces

### `rust/lib/stations-dds/src/dds.rs` (new)

```rust
pub struct Node { participant: cyclonedds::DomainParticipant }
impl Node {
    /// Create the participant for `domain`. CycloneDDS applies the shared
    /// configuration (DDS/cyclonedds-config.xml) through CYCLONEDDS_URI.
    pub fn new(domain: u32) -> cyclonedds::DdsResult<Self>;
    pub fn participant(&self) -> &cyclonedds::DomainParticipant;
    /// A typed writer on the short wire `topic` name.
    pub fn writer<T: cyclonedds::DdsType>(&self, topic: &str, qos: &cyclonedds::Qos)
        -> cyclonedds::DdsResult<cyclonedds::DataWriter<T>>;
    /// A typed reader on the short wire `topic` name.
    pub fn reader<T: cyclonedds::DdsType>(&self, topic: &str, qos: &cyclonedds::Qos)
        -> cyclonedds::DdsResult<cyclonedds::DataReader<T>>;
}

/// The QoS named by a contract ("default" | "reliable" | "best_effort" | "transient_local").
pub fn qos(name: &str) -> Option<cyclonedds::Qos>;
```

### `rust/lib/stations-dds/src/contract.rs` (new)

```rust
#[derive(serde::Deserialize)] pub struct TopicRegistry { pub topics: Vec<TopicDef> }
#[derive(serde::Deserialize)] pub struct TopicDef {
    pub name: String,    // short wire/registry name, e.g. "HelloWorldTopic"
    pub r#type: String,  // scoped IDL/DDS type, e.g. "hello_world::HelloWorldTopic"
    pub qos: String,
    #[serde(default)] pub key: Vec<String>,
}
#[derive(serde::Deserialize)] pub struct ServiceContract {
    pub service: ServiceMeta,
    #[serde(default)] pub publish: Vec<TopicRef>,
    #[serde(default)] pub subscribe: Vec<TopicRef>,
}
#[derive(serde::Deserialize)] pub struct ServiceMeta { pub name: String }
#[derive(serde::Deserialize)] pub struct TopicRef { pub topic: String }

impl TopicRegistry {
    pub fn load(path: &std::path::Path) -> Result<Self, ContractError>;
    pub fn from_toml(text: &str) -> Result<Self, ContractError>;
    pub fn validate(&self) -> Result<(), ContractError>;   // known types + known QoS + unique names
}
impl ServiceContract {
    pub fn load(path: &std::path::Path) -> Result<Self, ContractError>;   // checks name == file stem
    pub fn validate(&self, registry: &TopicRegistry) -> Result<(), ContractError>;
    pub fn render(&self) -> String;                                       // the `--topics` text
}

/// Scoped DDS type names of the generated topic types.
pub fn known_types() -> &'static [&'static str];

pub enum ContractError {
    Io(String), Toml(String), UnknownTopic(String), UnknownType(String),
    UnknownQos(String), NameMismatch { expected: String, found: String }, Duplicate(String),
}
```

### `rust/lib/stations-dds/src/lib.rs` (changed)

```rust
pub mod contract;
pub mod dds;

pub use cyclonedds;
pub fn type_name<T: cyclonedds::DdsType>() -> &'static str;

include!(concat!(env!("REPO_ROOT"), "/build/gen/rust/hello_world.rs"));
```

### `rust/lib/stations-dds/Cargo.toml` (changed)

Add `serde = { workspace = true }` and `toml = { workspace = true }`.

### `contracts/topics.toml` (new, authored)

```toml
# The topic registry. `name` is the short wire/registry name; `type` is the
# scoped IDL/DDS type name (it must match a generated DdsType::type_name()).

[[topics]]
name = "HelloWorldTopic"
type = "hello_world::HelloWorldTopic"
qos = "default"
key = []
```

### `contracts/services/hello-service.toml` (new, authored)

```toml
[service]
name = "hello-service"

[[publish]]
topic = "HelloWorldTopic"

[[subscribe]]
```

### `rust/tool/dds-contracts/{Cargo.toml,src/main.rs}` (new)

```
dds-contracts --contracts <dir> [--summary <path>] [--topics <app>] [--check]
```

Default: load and validate the registry and every `services/*.toml`, then write `--summary` (default `build/topics.md`). `--topics <app>` prints that service's `render()` and exits. `--check` validates only.

### `rust/lib/stations-dds/tests/round_trip.rs` (new)

```rust
// set CYCLONEDDS_URI = "file://<REPO_ROOT>/DDS/cyclonedds-config.xml"
// node = Node::new(0)?;
// writer = node.writer::<HelloWorldTopic>("HelloWorldTopic", &qos("reliable")?)?;
// reader = node.reader::<HelloWorldTopic>("HelloWorldTopic", &qos("reliable")?)?;
// writer.write(&sample)?;
// loop up to ~5 s: if let Some(s) = reader.take()?.into_iter().next() { assert_eq!(s, sample); return }
// panic!("no sample received")
```

### `rust/Cargo.toml` and `scripts/build.ps1` (changed)

```toml
# rust/Cargo.toml
members = [..., "tool/dds-contracts"]
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

`build.ps1` runs `dds-contracts --contracts <repoRoot>\contracts --summary <repoRoot>\build\topics.md` after the build (both `-Test` and normal), and in the deploy step copies `DDS/cyclonedds-config.xml` to `build/deploy/config/`.

## Pseudocode

### `Node::new` / `writer` / `reader`

```
new(domain):
    participant = DomainParticipant::new(domain)?      // config read from CYCLONEDDS_URI
    return Node { participant }

writer<T>(topic, qos):
    t = participant.create_topic::<T>(topic)?
    publisher = participant.create_publisher()?
    return publisher.create_writer_with_qos::<T>(&t, qos)

reader<T>(topic, qos):
    t = participant.create_topic::<T>(topic)?
    subscriber = participant.create_subscriber()?
    return subscriber.create_reader_with_qos::<T>(&t, qos)
```

### `qos`

```
match name:
    "default"          => Qos::builder().build()
    "reliable"         => Qos::builder().reliable().build()
    "best_effort"      => Qos::builder().best_effort().build()
    "transient_local"  => Qos::builder().transient_local().build()
    _                  => None
```

### `TopicRegistry::validate`

```
names = {}
for t in topics:
    if names.contains(t.name): return Err(Duplicate(t.name))
    names.insert(t.name)
    if not known_types().contains(t.type): return Err(UnknownType(t.type))
    if qos(t.qos).is_none(): return Err(UnknownQos(t.qos))
Ok(())
```

### `ServiceContract::validate`

```
for ref in publish + subscribe:
    if not registry.topics.iter().any(|t| t.name == ref.topic):
        return Err(UnknownTopic(ref.topic))
Ok(())
```

### `dds-contracts` main

```
parse args
registry = TopicRegistry::load(dir/topics.toml)?
registry.validate()?
services = for f in dir/services/*.toml: ServiceContract::load(f)?.validate(&registry)?
if --topics app: print services[app].render(); return
if --check: return
write summary (registry table + each service's render) to --summary
```

## Tests

* `contract.rs`: parse a valid registry and service; reject an unknown type, unknown QoS, a service-name mismatch, and a duplicate topic; `render()` contains the expected lines.
* `dds-contracts`/`stations-dds`: the `hello-service` `--topics` render appears in the generated `build/topics.md`.
* `round_trip.rs`: the typed publish/subscribe round-trip (requires the `StationsDDS` adapter).
* Existing `stations-dds` `type_names_match_scopes` test unchanged.

## Minimal test

```
.\scripts\build.ps1 -Test
build\rust\release\dds-contracts.exe --contracts contracts --topics hello-service
```

`build.ps1 -Test` builds, runs the round-trip and contract tests, and regenerates `build/topics.md`; the `--topics` output matches the `hello-service` section of `build/topics.md`.

## Risks and follow-ups

* **Environment.** The round-trip needs the `StationsDDS` adapter and `CYCLONEDDS_URI` (an existing prerequisite); a missing adapter fails the test with the CycloneDDS interface error.
* **`known_types()`** is a hand-kept list; a test asserts each entry equals a generated `type_name()`. Emitting it from codegen is a later option.
* **Runtime service loading and startup validation** become real when `hello-service`/`hello-view-service` arrive; deploying contracts to `build/deploy/config/` is a follow-up.
* **`dds-echo`** (the next story) consumes the `Node` layer.
* **`serde`/`toml`** are new workspace dependencies (recorded in `docs/versions.md` if the convention is followed).

## Module layout

Post-2018 layout (`foo.rs` plus a `foo/` directory; no `mod.rs`):

```
rust/lib/stations-dds/src/lib.rs        re-exports, type_name, generated types
rust/lib/stations-dds/src/dds.rs        Node, typed writer/reader, qos
rust/lib/stations-dds/src/contract.rs   schema, loader, validation, render
rust/lib/stations-dds/tests/round_trip.rs
rust/tool/dds-contracts/src/main.rs     the contract tool
```

## Files

* Authored: `contracts/topics.toml`, `contracts/services/hello-service.toml`, `rust/lib/stations-dds/src/dds.rs`, `rust/lib/stations-dds/src/contract.rs`, `rust/lib/stations-dds/tests/round_trip.rs`, `rust/tool/dds-contracts/{Cargo.toml,src/main.rs}`.
* Changed: `rust/lib/stations-dds/src/lib.rs`, `rust/lib/stations-dds/Cargo.toml`, `rust/Cargo.toml`, `scripts/build.ps1`, `planning/{architecture,backlog,current_story}.md`, optionally `docs/versions.md` and `README.md`.
