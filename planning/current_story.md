# Current story: dds-echo-decode

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Decode known-type DATA samples from the wire and show the fields in the detail pane (raw hex otherwise), and drive the **Known** topics from the registry (`contracts/topics.toml`) instead of the hardcoded list. Field-level display is interim (a formatted dump); a proper field model is deferred to `topic-field-metadata`.

- **Depends on:** none.
- **Minimal test:** with the crafted stimulus running, a `HelloWorldTopic` sample shows the decoded fields in the detail pane and its row sits under **Known**; an unregistered topic still shows hex.

## Context

- The UI already renders the decoded/hex choice: `main::to_ui` sets `known: r.is_known()` and `decoded: r.decoded`, and `ui/dds_echo.slint` shows `decoded` vs `hex`. `decoded` is never filled, and `model::KNOWN_TOPICS` is a hardcoded `["HelloWorldTopic"]`.
- `record.topic` is attributed in `main::tick` from SEDP (`Discovery::observe_and_attr`); `model::Frame` does not carry the DATA payload (only `rtps::Frame::payload` does).
- `CdrDeserializer::<T>::deserialize(data, encoding)` (fork `cyclonedds/src/serialization.rs`, re-exported by `stations-dds`) expects the CDR sample **without** the 4-byte encapsulation header, in **native endianness** (`dds_stream_normalize` starts at offset 0; `bswap=false`); `CdrEncoding` selects XCDR1/XCDR2 only. It validates and returns `Err` on malformed input (no panic).
- The generated `hello_world::HelloWorldTopic` derives `Debug`, so `format!("{sample:#?}")` is a usable interim dump.
- No real publisher of `HelloWorldTopic` exists yet (`hello-service` is later; `traffic-roundtrip.ps1` uses a single participant, so no SEDP is emitted and nothing is attributed). Verification uses the crafted stimulus.

## Decisions

- **Registry-driven known set.** Load `contracts/topics.toml` at startup via `stations-dds::contract::TopicRegistry` (default path from `REPO_ROOT`, `--registry` override) and convert it to a UI-agnostic `model::KnownTopics` (name -> idl_type), so `model.rs` stays independent of `stations-dds`.
- **Add a `stations-dds` dependency** to `dds-echo` (registry, generated types, and `cyclonedds::{CdrDeserializer, CdrEncoding}`). The bypass scan is unaffected (only serialization/contract APIs are used).
- **Decode is a compile-time dispatch** topic/type -> `CdrDeserializer::<T>`; only `hello_world::HelloWorldTopic` for now.
- **Interim rendering** is `format!("{sample:#?}")`; a field-level model (display and editing) is deferred to `topic-field-metadata`.
- Decode runs in `tick` after attribution, only for known topics; unknown topics, a failed decode, or a non-native-endian payload fall back to hex.
- If the registry fails to load, log to stderr and use an empty known set (everything shows as Unknown/hex).

## Interfaces

### `rust/tool/dds-echo/Cargo.toml` (changed)

```toml
stations-dds = { workspace = true }
```

### `rust/tool/dds-echo/src/model.rs` (changed)

```rust
/// The registry's topics: name -> scoped IDL/DDS type name.
#[derive(Clone, Debug, Default)]
pub struct KnownTopics {
    by_name: BTreeMap<String, String>,
}
impl KnownTopics {
    pub fn insert(&mut self, name: &str, idl_type: &str);
    pub fn contains(&self, name: &str) -> bool;
    pub fn idl_type(&self, name: &str) -> Option<&str>;
    pub fn names(&self) -> impl Iterator<Item = &str>;
}
```

- Remove the `KNOWN_TOPICS` const.
- `Record::is_known(&self, known: &KnownTopics) -> bool`.
- `TopicCounts::rows(&self, known: &KnownTopics) -> Vec<TopicRow>` (seeds the Known group from `known.names()`).
- `Frame` gains `pub payload: Option<std::ops::Range<usize>>` (the DATA serialized payload, into `record.hex`).

### `rust/tool/dds-echo/src/capture.rs` (changed)

- `frame_from` copies `f.payload` into `model::Frame.payload`.

### `rust/tool/dds-echo/src/decode.rs` (new)

```rust
/// Decode a known-type DATA payload (encapsulation header + CDR) into a
/// human-readable dump; `None` for an unknown type, a non-native-endian
/// encapsulation, or malformed data.
pub fn decode(idl_type: &str, payload: &[u8]) -> Option<String>;
```

- Match `"hello_world::HelloWorldTopic"` -> `HelloWorldTopic`.
- Check bytes `[0..2]` are `CDR_LE` (`0x0001`); strip the 4-byte encapsulation; `CdrDeserializer::<HelloWorldTopic>::deserialize(&payload[4..], CdrEncoding::Xcdr1).ok()`; `format!("{sample:#?}")`.

### `rust/tool/dds-echo/src/main.rs` (changed)

- `mod decode;`.
- `Args` gains `--registry <path>` (default `concat!(env!("REPO_ROOT"), "/contracts/topics.toml")`).
- `State` gains `known: KnownTopics`.
- Thread `&KnownTopics` through `to_ui`, `refresh_topics`, and `tick`.
- In `tick`, after `observe_and_attr`: if the record's topic is known, find the DATA frame whose `writer == record.entity_id` with a payload, and set `record.decoded = decode(idl_type, &record.hex[range]).unwrap_or_default()`.

### `rust/tool/dds-echo/ui/dds_echo.slint` (changed)

- The decoded/hex area keys off `root.selected.decoded != ""` (label and text), so a known topic that fails to decode falls back to hex instead of showing an empty pane.

## Pseudocode

```
// main: load the registry once
let registry = TopicRegistry::load(&registry_path);   // errors logged, empty known set on failure
let known: KnownTopics = <name, idl_type from registry.topics>;

// tick, per captured record, after Discovery::observe_and_attr:
if let Some(topic) = &record.topic {
    if let Some(idl_type) = known.idl_type(topic) {
        if let Some(f) = record.frames.iter().rev()
            .find(|f| f.kind == "DATA" && f.payload.is_some()) {
            record.decoded = decode(idl_type, &record.hex[f.payload.clone().unwrap()])
                .unwrap_or_default();
        }
    }
}
```

## Tests

- `decode.rs`: `CdrSerializer::serialize(&HelloWorldTopic { .. }, CdrEncoding::Xcdr1)` + a `CDR_LE` header -> `decode` returns a dump containing `greeting`, the greeting text, and the quaternion fields; unknown idl_type -> `None`; truncated payload -> `None`; `CDR_BE` encapsulation -> `None`; empty payload -> `None`.
- `model.rs`: `KnownTopics` `insert`/`contains`/`idl_type`/`names`; `TopicCounts::rows` seeds Known from `KnownTopics`; `Record::is_known`.
- Update existing `model`/`capture`/`main` tests for the new signatures and the `Frame.payload` field (the `record(..)`/`frame(..)` helpers gain the field).

## Minimal test

```
.\scripts\build.ps1 -Test
.\scripts\build.ps1
# build\deploy\bin\dds-echo.exe, Start, then:
.\scripts\traffic-fake-topic.ps1 -Repeat 2
```

Select the `HelloWorldTopic` data datagram: the detail pane shows the decoded fields, and its row is under **Known**. An unregistered topic's datagram still shows hex.

## Risks and follow-ups

- **Endianness:** only native (little-endian) samples decode; `CDR_BE` falls back to hex.
- **Dependency weight:** `stations-dds` links the CycloneDDS binding into `dds-echo` (static; no runtime DLL), and building `dds-echo` now requires the generated IDL (produced by `scripts/build.ps1`).
- **Registry path:** the default is the compile-time `REPO_ROOT` (fine for a dev tool run from the repo); `--registry` overrides.
- **Field display** is pretty-printed until `topic-field-metadata` provides a field model.

## Module layout

```
rust/tool/dds-echo/src/model.rs    + KnownTopics; Frame.payload; is_known/rows take &KnownTopics
rust/tool/dds-echo/src/capture.rs  frame_from copies payload
rust/tool/dds-echo/src/decode.rs   CdrDeserializer dispatch (HelloWorldTopic)
rust/tool/dds-echo/src/main.rs     registry load, KnownTopics state, tick decode
rust/tool/dds-echo/ui/dds_echo.slint  decoded/hex keyed on decoded != ""
```
