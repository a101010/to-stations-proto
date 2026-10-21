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

* **Contract-aware `Node`** (`stations-dds::dds`) over `DomainParticipant`. It holds a validated `TopicRegistry`; `writer`/`reader` take only the topic name and enforce that it is registered and that `T::type_name()` equals the entry's `type`, then use the entry's QoS. A temporary `Publisher`/`Subscriber` per endpoint is fine: the binding's guards keep them alive for the writer/reader's lifetime. This makes an unregistered or mis-typed use of DDS fail at creation rather than silently at runtime.
* **Config via `CYCLONEDDS_URI`.** The layer creates the participant and documents that CycloneDDS reads the shared config from `CYCLONEDDS_URI`; the round-trip test sets it to `<REPO_ROOT>/DDS/cyclonedds-config.xml`, and the launch scripts will point at the deployed `build/deploy/config/`. `build.ps1` begins copying the config to `build/deploy/config/`.
* **Topic names are short** (`HelloWorldTopic`) for the registry and the wire; the scoped IDL/DDS type (`hello_world::HelloWorldTopic`) is recorded separately in the registry as `type`. Validation requires `type` to equal a known `DdsType::type_name()`.
* **Contracts** live in `stations-dds::contract` (schema, loader, validation, `render`), using `serde` + `toml`.
* **`dds-contracts` tool** (`rust/tool/dds-contracts`) loads the repo contracts, validates them, writes `build/topics.md`, and prints a service with `--topics <app>`. It is a developer tool, not deployed (like `equirect-to-cubemap`).
* **`--topics` and startup validation are helpers.** `ServiceContract::render()` is exposed by the tool now and called by real services later; deploying contracts to `build/deploy/config/` for runtime service loading is a follow-up.
* **Registry-completeness test.** A `stations-dds` test scans `DDS/*.idl` for `struct <Name>Topic` (scoped by the file's `module`) and asserts that set equals `known_types()` and that every entry is registered. This catches a new topic added to the IDL but not registered, and keeps the hand-kept `known_types()` honest.
* **Coverage listing.** `build/topics.md` records, for each registry topic, the services that publish and subscribe it. Informational only (a topic no service references yet just shows empty); it never fails.
* **Bypass scan.** The raw CycloneDDS topic-naming API must be reachable only through the Node implementation; `dds-contracts --scan` fails the build when it finds a raw call in the scanned source. The scanner is **repo-agnostic**: it takes the roots (and optional allowed paths) from the caller and has no knowledge of this repository's layout. The build supplies the scope: **the specific crate(s) being built** — an explicit list of application/service/tool `src` directories (option a), not all of `rust/`. Libraries, including the Node implementation, are simply never in scope, so there is no hardcoded skip. This guards the one thing the Node API cannot: reaching around it. (The Node API already enforces registration and type at endpoint creation, so Node call literals are not checked statically.)
* **The round-trip is a committed integration test** that runs in `cargo test` (and `build.ps1 -Test`); it requires the `StationsDDS` adapter, an existing environment prerequisite.
* **Node enforcement is also tested end-to-end.** `tests/node_contract.rs` builds a `Node` from the committed registry and asserts that `writer`/`reader` return `UnregisteredTopic` and `TypeMismatch`. Because `check` runs before topic creation, it needs a participant but no discovery; it shares the `round_trip` adapter precondition.

## Interfaces

### `rust/lib/stations-dds/src/dds.rs` (new)

```rust
pub enum Error {
    Dds(cyclonedds::DdsError),
    Contract(crate::contract::ContractError),
    UnregisteredTopic(String),
    TypeMismatch { topic: String, expected: String, found: String },
}

pub struct Node {
    participant: cyclonedds::DomainParticipant,
    registry: crate::contract::TopicRegistry,
}
impl Node {
    /// Create the participant and validate the registry.
    pub fn new(domain: u32, registry: crate::contract::TopicRegistry) -> Result<Self, Error>;
    pub fn participant(&self) -> &cyclonedds::DomainParticipant;
    pub fn registry(&self) -> &crate::contract::TopicRegistry;

    /// Writer for `topic`: it must be registered, `T::type_name()` must equal the
    /// entry's `type`, and the entry's QoS is used.
    pub fn writer<T: cyclonedds::DdsType>(&self, topic: &str)
        -> Result<cyclonedds::DataWriter<T>, Error>;
    /// Reader for `topic`; the same checks as `writer`.
    pub fn reader<T: cyclonedds::DdsType>(&self, topic: &str)
        -> Result<cyclonedds::DataReader<T>, Error>;
}

/// The registry/type check, factored out so it is testable without a participant.
fn check<T: cyclonedds::DdsType>(registry: &crate::contract::TopicRegistry, topic: &str)
    -> Result<&crate::contract::TopicDef, Error>;

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
    pub fn get(&self, name: &str) -> Option<&TopicDef>;
}
impl ServiceContract {
    pub fn load(path: &std::path::Path) -> Result<Self, ContractError>;   // checks name == file stem
    pub fn validate(&self, registry: &TopicRegistry) -> Result<(), ContractError>;
    pub fn render(&self) -> String;                                       // the `--topics` text
}

/// Scoped DDS type names of the generated topic types.
pub fn known_types() -> &'static [&'static str];

/// Per-topic usage for the summary: which services publish and subscribe each
/// registered topic. Informational only.
pub struct TopicUsage { pub topic: String, pub publishers: Vec<String>, pub subscribers: Vec<String> }
pub fn coverage(registry: &TopicRegistry, services: &[ServiceContract]) -> Vec<TopicUsage>;

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

### `rust/tool/dds-contracts/{Cargo.toml,src/main.rs,src/scan.rs}` (new)

```
dds-contracts --contracts <dir> [--scan <dir> …] [--allow <path> …] [--summary <path>] [--topics <app>] [--check]
```

Default: load and validate the registry and every `services/*.toml`, then write `--summary` (default `build/topics.md`) with the registry table, a Usage section (per topic, its publishers and subscribers), and each service's rendered section. `--topics <app>` prints that service's `render()` and exits. `--check` validates only. `--scan <dir>` (repeatable) scans that root for raw CycloneDDS topic-API calls; `--allow <path>` (repeatable) skips a subtree. There are no default scan roots: the caller supplies the scope.

### `rust/tool/dds-contracts/src/scan.rs` (new)

```rust
/// A raw CycloneDDS topic-naming call found in the scanned roots.
pub struct Bypass { pub file: std::path::PathBuf, pub line: usize, pub name: String }

/// The curated raw names, matched textually (name followed by `(` or `::<`).
pub const RAW_TOPIC_APIS: &[&str] = &[
    "create_topic", "create_topic_with_qos",
    "create_topic_from_descriptor", "create_topic_from_descriptor_with_qos",
    "create_topic_from_type_info", "create_topic_from_type_info_with_qos",
    "create_topic_from_sertype", "create_topic_from_sertype_with_qos",
    "find_topic", "dynamic_publish", "dynamic_subscribe",
];

/// Scan `roots` for raw topic-API calls, skipping any path under `allow`. No
/// repository knowledge: the caller supplies the scope. Skips only generic
/// paths (`target/`) and non-`.rs` files.
pub fn scan(roots: &[std::path::PathBuf], allow: &[std::path::PathBuf])
    -> Result<Vec<Bypass>, std::io::Error>;
```

### `rust/lib/stations-dds/tests/round_trip.rs` (new)

```rust
// set CYCLONEDDS_URI = "file://<REPO_ROOT>/DDS/cyclonedds-config.xml"
// registry = TopicRegistry::load(<REPO_ROOT>/contracts/topics.toml)?; registry.validate()?;
// node = Node::new(0, registry)?;
// writer = node.writer::<HelloWorldTopic>("HelloWorldTopic")?;
// reader = node.reader::<HelloWorldTopic>("HelloWorldTopic")?;
// loop up to ~5 s: writer.write(&sample)?; sleep 50 ms;
//     if let Some(s) = reader.take()?.into_iter().next() { assert_eq!(s, sample); return }
// panic!("no sample received")
```

### `rust/lib/stations-dds/tests/node_contract.rs` (new)

```rust
// set CYCLONEDDS_URI once; registry = TopicRegistry::load(<REPO_ROOT>/contracts/topics.toml)
// node = Node::new(0, registry)?
// writer::<HelloWorldTopic>("NotRegistered")    -> Err(UnregisteredTopic)
// writer::<math::Quaternion>("HelloWorldTopic") -> Err(TypeMismatch)
// writer::<HelloWorldTopic>("HelloWorldTopic")  -> Ok
// one test function (no parallel participants); adapter precondition, as round_trip
```

### `rust/Cargo.toml` and `scripts/build.ps1` (changed)

```toml
# rust/Cargo.toml
members = [..., "tool/dds-contracts"]
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

`build.ps1` runs `dds-contracts --contracts <repoRoot>\contracts --scan <src> … --summary <repoRoot>\build\topics.md` after the build (both `-Test` and normal), where each `--scan <src>` is an application/service/tool crate source directory (currently `rust\ui\lil-d\src` and `rust\tool\equirect-to-cubemap\src`; it grows with the `service/*` crates). Libraries are not scanned, so the Node implementation is out of scope. No application crate uses DDS yet, so the scan currently passes vacuously; it becomes meaningful with the first DDS-using service. In the deploy step, `build.ps1` copies `DDS/cyclonedds-config.xml` to `build/deploy/config/`.

## Pseudocode

### `Node::new` / `writer` / `reader` / `check`

```
new(domain, registry):
    registry.validate()?
    participant = DomainParticipant::new(domain)?      // config read from CYCLONEDDS_URI
    return Node { participant, registry }

writer<T>(topic):
    entry = check::<T>(&registry, topic)?
    q = qos(&entry.qos).ok_or(UnknownQos(entry.qos))?   // already validated
    t = participant.create_topic::<T>(topic)?
    publisher = participant.create_publisher()?
    return publisher.create_writer_with_qos::<T>(&t, &q)

reader<T>(topic):
    entry = check::<T>(&registry, topic)?
    q = qos(&entry.qos).ok_or(UnknownQos(entry.qos))?
    t = participant.create_topic::<T>(topic)?
    subscriber = participant.create_subscriber()?
    return subscriber.create_reader_with_qos::<T>(&t, &q)

check<T>(registry, topic):
    entry = registry.get(topic).ok_or(UnregisteredTopic(topic))?
    if entry.type != T::type_name():
        return Err(TypeMismatch { topic, expected: entry.type, found: T::type_name() })
    Ok(entry)
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
if --scan roots:
    for b in scan(roots, allow): eprintln!("{b.file}:{b.line}: raw topic API `{b.name}` (use the Node API)")
    return non-zero if any
registry = TopicRegistry::load(dir/topics.toml)?
registry.validate()?
services = for f in dir/services/*.toml: ServiceContract::load(f)?.validate(&registry)?
if --topics app: print services[app].render(); return
if --check: return
write summary (registry table + coverage(&registry, &services) + each service's render) to --summary
```

### `scan`

```
for root in roots:
    for file in walk(root) where file ends in ".rs" and not under any allow path and not under "target":
        text = read(file)
        for name in RAW_TOPIC_APIS:
            for each occurrence of name immediately followed by "(" or "::<":
                push Bypass { file, line, name }
return bypasses
```

## Tests

All tests use committed inputs (contracts, IDL, fixtures); none writes files at runtime.

* `contract.rs`: parse a valid registry and service; reject an unknown type, unknown QoS, a service-name mismatch, and a duplicate topic; `render()` contains the expected lines; `coverage` lists `hello-service` as a publisher of `HelloWorldTopic` and no subscriber, and shows empty lists for an unreferenced topic.
* `dds.rs`: `check` with `HelloWorldTopic` → Ok; an unregistered name → `UnregisteredTopic`; a registry entry with a mismatched `type` → `TypeMismatch`.
* `stations-dds` completeness: scanning `DDS/*.idl`, the topic set equals `known_types()` and every entry is registered.
* `scan.rs` (dds-contracts): against the committed fixtures `tests/fixtures/scan/` — `bypass.rs` (a raw `create_topic("X")` call) is flagged with the expected line and name, `ok.rs` (a `Node::writer("X")` call) is not, and `allowed/bypass.rs` is skipped when passed through `--allow`.
* `node_contract.rs` (end-to-end): `writer`/`reader` on an unregistered name → `UnregisteredTopic`, a registered name with a mismatched `T` → `TypeMismatch`, and a registered name with the matching `T` → `Ok`; requires the `StationsDDS` adapter (shared with `round_trip`).
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
* **`known_types()`** is a hand-kept list; the completeness test asserts it equals the IDL's topic set and that each entry equals a generated `type_name()`. Emitting it from codegen is a later option.
* **`key` is recorded but not validated.** `TopicDef::key` is parsed and printed but not checked against the IDL `@key` members; key validation is deferred until a format introduces a keyed topic (it is redundant with the IDL and needs a name→type dispatch plus nested-key handling).
* **The bypass scan is a lint, not a proof.** It matches the raw names textually (so it can miss an aliased call or false-positive inside a comment/string), and it only covers the roots the build passes (the application/service/tool crates); the Node library is deliberately out of scope. Until a DDS-using service exists, the build-time scan passes vacuously — the scanner logic itself is exercised by the committed-fixture tests. Option 1 remains the runtime backstop for anything that goes through the Node API.
* **Runtime service loading and startup validation** become real when `hello-service`/`hello-view-service` arrive; deploying contracts to `build/deploy/config/` is a follow-up.
* **`dds-echo`** (the next story) consumes the `Node` layer.
* **`serde`/`toml`** are new workspace dependencies (recorded in `docs/versions.md` if the convention is followed).

## Module layout

Post-2018 layout (`foo.rs` plus a `foo/` directory; no `mod.rs`):

```
rust/lib/stations-dds/src/lib.rs        re-exports, type_name, generated types
rust/lib/stations-dds/src/dds.rs        Node, check, typed writer/reader, qos
rust/lib/stations-dds/src/contract.rs   schema, loader, validation, render, coverage
rust/lib/stations-dds/tests/round_trip.rs
rust/lib/stations-dds/tests/node_contract.rs   end-to-end contract errors
rust/lib/stations-dds/tests/registry.rs IDL/registry completeness
rust/tool/dds-contracts/src/main.rs     the contract tool
rust/tool/dds-contracts/src/scan.rs     the repo-agnostic bypass scan
rust/tool/dds-contracts/tests/scan.rs   scanner tests
rust/tool/dds-contracts/tests/fixtures/scan/   committed scan fixtures
```

## Files

* Authored: `contracts/topics.toml`, `contracts/services/hello-service.toml`, `rust/lib/stations-dds/src/dds.rs`, `rust/lib/stations-dds/src/contract.rs`, `rust/lib/stations-dds/tests/{round_trip.rs,node_contract.rs,registry.rs}`, `rust/tool/dds-contracts/{Cargo.toml,src/main.rs,src/scan.rs}`, `rust/tool/dds-contracts/tests/scan.rs`, `rust/tool/dds-contracts/tests/fixtures/scan/{bypass.rs,ok.rs,allowed/bypass.rs}`.
* Changed: `rust/lib/stations-dds/src/lib.rs`, `rust/lib/stations-dds/Cargo.toml`, `rust/Cargo.toml`, `scripts/build.ps1`, `planning/{architecture,backlog,current_story}.md`, optionally `docs/versions.md` and `README.md`.
