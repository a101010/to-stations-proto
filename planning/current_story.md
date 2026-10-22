# Current story: dds-echo-sedp

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Parse SEDP/SPDP so `dds-echo` learns the participants and endpoints it hears, the topic/type each endpoint uses, and how to attribute data traffic to those topics. Accumulate per-topic packet stats in the topics panel (grouped by topic, as now); datagrams whose topic cannot be determined keep accumulating under the existing `unknown` row.

- **Depends on:** dds-echo-frame.
- **Minimal test:** with traffic generating, the inspector lists the participants/endpoints/topics it observes; per-topic counts accumulate in the Topics panel; unattributable datagrams stay under `unknown`. Unit tests cover the parameter-list layouts and malformed input.

## Context

- `rtps.rs` already walks submessages, locates the reader/writer/SN inside DATA, and exposes `sub`/`base`; it does not expose the DATA payload.
- `model.rs::TopicCounts` already counts packets per `record.topic` and an `unidentified` bucket, and `rows()` seeds the `Known` group from `KNOWN_TOPICS` (currently the hardcoded `["HelloWorldTopic"]` placeholder). `record.topic` is never set today, so the tree is effectively static.
- `dds_echo.slint` already declares `participants` and `endpoints` stat chips (lines 122-123, 204-205) that nothing populates.
- Wire facts confirmed from the CycloneDDS source (`vendor/cyclonedds`): SEDP/SPDP payloads are a **PL_CDR** encapsulation header + parameter list; `PID_TOPIC_NAME 0x0005`, `PID_TYPE_NAME 0x0007`, `PID_PARTICIPANT_GUID 0x0050`, `PID_ENDPOINT_GUID 0x005a`, `PID_SENTINEL 0x0001`; parameter `length` is pre-padded to a multiple of 4; GUIDs are 16 B (prefix 12 + entity id 4); entity kind byte is `entity[3] & 0x3f` (`0x02/0x03` writer, `0x04/0x07` reader). The DATA serialized payload starts at `base + 16`.

## Decisions

- **New `sedp` module** (`src/sedp.rs`): a bounds-safe wire parser for the PL_CDR parameter list. `capture` depends on `rtps` and `sedp`; `rtps` stays transport-only (no discovery knowledge).
- **Payload via a range, not a copy.** `rtps::Frame` gains `payload: Option<Range<usize>>` (absolute offsets into the datagram) for DATA submessages only; `capture::record_from` slices it and calls `sedp::parse`. Avoids copying user-data payloads at capture rates.
- **Detect discovery by payload content**, not entity id: any DATA whose payload carries `PID_ENDPOINT_GUID` is an endpoint announcement, `PID_PARTICIPANT_GUID` a participant. This is vendor-agnostic and needs no builtin-endpoint-id table.
- **Registry/known matching is deferred** (next story). This story does not classify observed topics as known: every topic learned from SEDP is shown under **Unknown**. The `KNOWN_TOPICS` placeholder and the `Known` group are left untouched. (Open point: if the placeholder should be dropped so the `Known` group is empty until the next story, say so - otherwise a name match to `HelloWorldTopic` can still surface as known.)
- **Left panel stays grouped by topic.** No participants/endpoints groups. Participants/endpoints surface as the two existing **stat chips** (counts) and in the **detail pane** for the selected datagram.
- **Attribute data by writer GUID.** `Discovery` maps full GUID (prefix+entity, 32 hex) -> topic from SEDP; `record.topic` is filled from that map before counting, so attributed data lands on its topic row. Unattributed datagrams (discovery, or data before its announcement) land on the existing `unknown`/`unidentified` row. This satisfies "accumulate stats under the topics we hear".
- **Dedup by GUID; no dispose/lease teardown** in this story (noted as a risk).

## Interfaces

### `rust/tool/dds-echo/src/rtps.rs` (changed)

```rust
pub struct Frame {
    // ...existing fields...
    /// Absolute range, in the datagram, of a DATA submessage's serialized
    /// payload (4-byte encapsulation header onward); None for non-DATA or empty.
    pub payload: Option<std::ops::Range<usize>>,
}
```

- For `id == 0x15` (DATA): `start = body + base + 16`. If the `Q` flag (`flags & 0x02`) is set, walk the inline-QoS parameter list (u16 pid, u16 len, value) from `start` until `PID_SENTINEL`, then the payload begins after it. `payload = Some(start..end)` when `start <= end`, else `None`.
- DATA_FRAG (`0x16`) and other submessages: `payload = None`.

### `rust/tool/dds-echo/src/sedp.rs` (new)

```rust
/// Which an announced endpoint is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointKind { Writer, Reader, Other }

/// One discovery announcement decoded from a DATA payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Discovered {
    Participant { guid_prefix: [u8; 12] },
    Endpoint {
        guid: [u8; 16],
        kind: EndpointKind,
        topic: Option<String>,
        type_name: Option<String>,
    },
}

/// Parse a PL_CDR serialized payload (encapsulation header + parameter list).
/// Non-parameter-list payloads and malformed/truncated input yield None; never panics.
pub fn parse(payload: &[u8]) -> Option<Discovered>;
```

- Read `representation_identifier` big-endian from bytes `[0..2]`: `0x0002/0x000a` -> BE, `0x0003/0x000b` -> LE; anything else (CDR/CDR2 or unknown) -> `None`.
- Walk parameters from offset 4: `pid` (u16), `len` (u16), `value = &payload[off..off+len]`; stop on `PID_SENTINEL` or a short read. Extract `0x0050 -> participant guid prefix`, `0x005a -> guid`, `0x0005 -> topic`, `0x0007 -> type_name`; skip the rest.
- `kind` from `guid[12..16][3] & 0x3f`.
- Choose `Endpoint` when a GUID was found, else `Participant`, else `None`.
- `read_string(value, le)`: u32 length (LE/BE) -> that many bytes -> strip one trailing NUL -> lossy UTF-8.

### `rust/tool/dds-echo/src/model.rs` (changed)

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointKind { Writer, Reader, Other }
impl EndpointKind { pub fn label(self) -> &'static str; }   // "writer" | "reader" | "other"

/// A discovered endpoint, formatted for display.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub guid: String,      // 32 hex (prefix + entity)
    pub prefix: String,    // 24 hex
    pub kind: EndpointKind,
    pub topic: String,     // "" when the endpoint carries no topic (builtin)
    pub type_name: String, // "" when absent
}
```

- `Record` gains `pub endpoints: Vec<Endpoint>` and `pub participant_guids: Vec<String>` (prefixes seen in a participant announcement).
- `Record::writer_guid(&self) -> Option<String>` = `guid_prefix` + `entity_id` when both present.
- New discovery state (UI-thread):

```rust
#[derive(Default)]
pub struct Discovery {
    participants: BTreeMap<String, usize>,   // prefix -> endpoint count
    endpoints: BTreeMap<String, Endpoint>,   // guid -> endpoint
}
impl Discovery {
    /// Learn this record's announcements, then set `r.topic` from its writer GUID
    /// when the topic is still unknown and the writer is a known endpoint with a topic.
    pub fn observe_and_attr(&mut self, r: &mut Record);
    pub fn participants(&self) -> usize;
    pub fn endpoints(&self) -> usize;
    pub fn reset(&mut self);
}
```

- `TopicCounts` is unchanged; attribution sets `record.topic` before `counts.observe`.

### `rust/tool/dds-echo/src/capture.rs` (changed)

- After `rtps::parse`, for each frame with `payload`, call `sedp::parse(&data[range])`; map `Discovered` -> `model::Endpoint` / `record.participant_guids`.
- Add tests for a crafted SEDP datagram and a crafted SPDP datagram.

### `rust/tool/dds-echo/src/main.rs` (changed)

- `State` gains `discovery: RefCell<Discovery>`.
- `tick`, per drained record: `discovery.observe_and_attr(&mut record)`, then `stats.observe`, `counts.observe`, `records.push(to_ui(&record))`.
- Set `app.set_participants(discovery.participants() as i32)` and `app.set_endpoints(...)`.
- `to_ui` maps `record.endpoints` -> `ui::Endpoint`.
- `on_clear` resets `discovery` and the two chips.

### `rust/tool/dds-echo/ui/dds_echo.slint` (changed)

```slint
export struct Endpoint {
    guid: string,
    kind: string,
    topic: string,
    type-name: string,
}
```

- `Record` gains `endpoints: [Endpoint]`.
- Detail pane: after the metadata lines, when `root.selected.endpoints.length > 0`, a `"discovery (" + ... + ")"` header and a short `ScrollView` list (columns: guid, kind, topic, type).
- The `participants`/`endpoints` stat chips are unchanged in layout; they now receive data.

## Pseudocode

### `sedp::parse`

```
if payload.len() < 4: return None
id = be_u16(payload[0..2])
le = match id { 0x0002|0x000a => false, 0x0003|0x000b => true, _ => return None }
off = 4; (pguid, guid, topic, ty) = (None, None, None, None)
while off + 4 <= payload.len():
    pid = u16(payload, off, le); len = u16(payload, off+2, le)
    off += 4
    if pid == 0x0001: break
    value = payload.get(off..off+len) else return None
    match pid {
        0x0050 => pguid = value[0..12]
        0x005a => guid = value[0..16]
        0x0005 => topic = read_string(value, le)
        0x0007 => ty = read_string(value, le)
    }
    off += len
if guid: Endpoint { guid, kind_of(guid[12..16]), topic, ty }
elif pguid: Participant { pguid } else None
```

### `Discovery::observe_and_attr`

```
for p in r.participant_guids: participants.entry(p).or_insert(0)
for e in r.endpoints: participants.entry(e.prefix).or_insert(0); endpoints[e.guid] = e
if r.topic.is_none() and r.writer_guid():
    if let Some(e) = endpoints.get(writer_guid) and !e.topic.is_empty(): r.topic = e.topic
```

## Tests

- `rtps.rs`: DATA payload range points at the encapsulation header (Q clear); a Q-set DATA skips the inline-QoS list; DATA with no payload.
- `sedp.rs`: participant (LE) with `PID_PARTICIPANT_GUID`; endpoint (BE) with `PID_ENDPOINT_GUID` + topic/type; writer vs reader kind; 4-byte-padded string; CDR (non-PL) payload -> `None`; truncated parameter / missing sentinel -> `None`, no panic; unknown PIDs skipped.
- `capture.rs`: `record_from` on a crafted SEDP DATA datagram yields the expected `endpoints`; on a crafted SPDP datagram yields the participant prefix.
- `model.rs`: `Discovery::observe_and_attr` dedups by GUID, builds the writer->topic map, sets `record.topic` for a matching writer, leaves an unknown writer's topic unset, and reports participant/endpoint counts; `reset` clears.
- `model.rs`: update the `record(..)` helper for the new `endpoints`/`participant_guids` fields.

## Minimal test

```
.\scripts\build.ps1 -Test
.\scripts\traffic-roundtrip.ps1 -Repeat 3      # HelloWorldTopic endpoint announced
.\scripts\traffic-throughput.ps1 -Seconds 6    # Throughput data to attribute
```

Start dds-echo first: the Topics panel lists the observed topics with packet counts (attributed data), unattributable datagrams stay under `unknown`, and the participants/endpoints chips show counts. Select an SEDP datagram to see its announcements in the detail pane. Release build and deploy via `scripts/build.ps1`.

## Risks and follow-ups

- **Ordering:** user data that arrives before its SEDP announcement is counted as `unknown`; acceptable and expected until re-announcement.
- **Dispose/lease:** SEDP disposes are ignored, so stale endpoints accumulate; defer.
- **Q-set inline QoS** is handled for DATA; **DATA_FRAG** fragments are not (SEDP is small and unfragmented).
- **Known-topic matching** (the next story) will move observed topics from Unknown to Known and reconnect the placeholder list.

## Module layout

```
rust/tool/dds-echo/src/rtps.rs   RTPS header/submessage parser (+ DATA payload range)
rust/tool/dds-echo/src/sedp.rs   PL_CDR parameter-list parser -> Discovered
rust/tool/dds-echo/src/capture.rs sockets, classification, record_from (+ SEDP wiring)
rust/tool/dds-echo/src/model.rs  Class/Record/Frame/Endpoint/Discovery/Stats/TopicCounts + tests
rust/tool/dds-echo/src/main.rs   module decl, Discovery state, tick attribution, to_ui
rust/tool/dds-echo/ui/dds_echo.slint  Endpoint struct, Record.endpoints, detail discovery list
```
