# Current story: dds-echo-frame

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Parse the RTPS header and submessages (writer GUID, sequence number, sizes) and surface them in the UI.

- **Depends on:** dds-echo-capture.
- **Minimal test:** with traffic generating, a datagram's detail lists its submessages (ids), the participant GUID, writer entity ids, and sequence numbers; unit tests cover the submessage layouts and malformed input.

## Context

- `capture.rs::record_from` fills `class`/`src_addr`/`size`/`hex`/`summary` (`"RTPS"`/`"UDP"`) and leaves `guid_prefix`/`entity_id` as `None`.
- The detail pane already shows `guid`/`entity`/`summary` lines, but nothing fills `guid`/`entity`, and the table's **Summary** column shows only `RTPS`.
- Wire-level only: no IDL, contract, or dependency changes.

## Decisions

- **New `rtps` module** (`src/rtps.rs`); it depends on `model`, and `capture` depends on both (no cycles).
- **Parse eagerly in `record_from`** so every row has a real Summary and the detail is always complete. The cost is trivial at observed rates (~100 datagrams/s; discovery-only otherwise).
- **The parser is bounds-safe:** every read is length-checked against the remaining bytes, so malformed or truncated datagrams never panic.
- **GUID display.** The RTPS header carries the sender participant GUID prefix (12 B = 24 hex); writer/reader fields are 4-byte entity ids. "Writer GUID" = prefix + entity; the detail pane shows the prefix as `guid`, and each frame row shows the writer entity id (a full 32-hex GUID won't fit the 361 px pane).
- **`summary`** becomes e.g. `DATA(sn=42); HEARTBEAT(1..5); INFO_TS`.

## Interfaces

### `rust/tool/dds-echo/src/rtps.rs` (new)

```rust
/// The 20-byte RTPS header.
pub struct Header {
    pub version: (u8, u8),      // major, minor
    pub vendor: [u8; 2],        // vendor id
    pub guid_prefix: [u8; 12],  // sender participant GUID prefix
}

/// One submessage, decoded far enough to describe it.
pub struct Frame {
    pub id: u8,                       // submessage id (0x15 DATA, 0x07 HEARTBEAT, …)
    pub kind: &'static str,           // "DATA" | "HEARTBEAT" | "ACKNACK" | "GAP"
                                      // | "INFO_TS" | "INFO_SRC" | "INFO_DST"
                                      // | "DATA_FRAG" | "PAD" | "0xNN"
    pub writer: Option<[u8; 4]>,      // writer EntityId
    pub reader: Option<[u8; 4]>,      // reader EntityId
    pub sn: Option<u64>,              // writerSN, or heartbeat firstSN
    pub sn_last: Option<u64>,         // heartbeat lastSN
    pub count: Option<u32>,           // heartbeat/acknack count
    pub len: usize,                   // octetsToNextHeader (resolved when 0)
}

pub struct Parsed {
    pub header: Option<Header>,               // None when not RTPS
    pub frames: Vec<Frame>,
    pub src_guid_prefix: Option<[u8; 12]>,    // header prefix, updated by INFO_SRC
}

/// Parse a datagram. Non-RTPS input returns an empty `Parsed` (`header: None`).
pub fn parse(data: &[u8]) -> Parsed;
```

Wire layout to implement (all multi-byte fields honour the submessage `E` flag; `E` is
bit 0 of the flags byte: `1` = little-endian, `0` = big-endian; submessage `length == 0`
means "extends to end of message"):

- **Header (20 B):** `"RTPS"` (4), version major (1), minor (1), vendorId (2), guidPrefix (12).
- **Submessage header (4 B):** submessageId (1), flags (1), octetsToNextHeader (2).
- **DATA `0x15`:** (if `flags & 0x02` inline-QoS: extraFlags (2)), readerId (4), writerId (4),
  writerSN (8 = `i32` high then `u32` low; value = `high·2^32 + low`).
- **HEARTBEAT `0x07`:** readerId (4), writerId (4), firstSN (8), lastSN (8), count (4).
- **ACKNACK `0x06`:** readerId (4), writerId (4), bitmapBase (8), numBits (4), bitmap, count (4).
- **GAP `0x08`:** readerId (4), writerId (4), gapStart (8), numBits (4), bitmap.
- **INFO_TS `0x09`:** if `flags & 0x02` no timestamp; else timestamp (8).
- **INFO_SRC `0x0c`:** unused (4), version (2), vendorId (2), guidPrefix (12) → `src_guid_prefix`.
- **INFO_DST `0x0e`:** guidPrefix (12).
- **DATA_FRAG `0x16`:** readerId (4), writerId (4), writerSN (8), fragmentStartingNum (4),
  fragmentsInSubmessage (2), fragmentSize (2), sampleSize (4).
- **PAD `0x01`:** no payload. Unknown ids: skip by length (or stop if the length is unreadable).

### `rust/tool/dds-echo/src/model.rs` (changed)

```rust
/// One submessage, formatted for display.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub id: u8,
    pub kind: String,    // "DATA", "HEARTBEAT", …
    pub writer: String,  // entity id hex ("000003c2"); empty when absent
    pub reader: String,
    pub seq: String,     // "42", "1..5", or ""
    pub len: usize,
}
```

- `Record` gains `pub frames: Vec<Frame>`.
- Add `Record::frame_summary(&self) -> String` → `"DATA(sn=42); HEARTBEAT(1..5); INFO_TS"` (`""` when there are no frames).

### `rust/tool/dds-echo/src/capture.rs` (changed)

- `record_from`: for RTPS input, run `rtps::parse` and set:
  - `guid_prefix` = `src_guid_prefix` as hex (24 chars);
  - `entity_id` = the last DATA/HEARTBEAT `writer` entity id as hex (8 chars);
  - `summary` = `frame_summary()` (fallback `"RTPS"` when there are no frames);
  - `frames` = the parsed frames mapped to `model::Frame`.
- Non-RTPS input is unchanged (`Class::Udp`, `summary: "UDP"`, no frames).
- Update the `record_from_classifies` test (the RTPS summary is no longer the bare `"RTPS"`).

### `rust/tool/dds-echo/src/main.rs` (changed)

- `to_ui`: map `model::Frame` → `ui::Frame` and set `ui::Record.frames`.

### `rust/tool/dds-echo/ui/dds_echo.slint` (changed)

```slint
export struct Frame {
    id: int,
    kind: string,
    writer: string,
    reader: string,
    seq: string,
    len: int,
}
```

- `Record` gains `frames: [Frame],`.
- In the detail pane, after the metadata lines: a `"Submessages (" + root.selected.frames.length + ")"` header and a fixed-height (~160 px) `ScrollView` list with one ~18 px row per frame (columns: kind, writer, seq, len), above the existing hex/decoded `ScrollView`.

## Pseudocode

### `rtps::parse`

```
if data.len() < 20 || !data.starts_with(b"RTPS"): return Parsed { header: None, ..empty }
header = Header { version: (data[4], data[5]), vendor: data[6..8], guid_prefix: data[8..20] }
src = header.guid_prefix
frames = []
off = 20
while off + 4 <= data.len():
    id = data[off]; flags = data[off+1]; e = flags & 1
    len = read_u16(&data[off+2..off+4], e)
    body = off + 4
    end = if len == 0 { data.len() } else { min(body + len, data.len()) }
    match id:
        DATA | DATA_FRAG | HEARTBEAT | ACKNACK | GAP:
            reader = read_entity(body); writer = read_entity(body+4)
            sn = read_seq(...)         // DATA/DATA_FRAG; HEARTBEAT uses first/last
            (bounds-checked; missing bytes -> None)
        INFO_SRC: src = data[body+8..body+20]
        INFO_TS | INFO_DST | PAD | other: no fields
    frames.push(Frame { id, kind, writer, reader, sn, sn_last, count, len: end - body })
    if len == 0 { break }
    off = end
Parsed { header: Some(header), frames, src_guid_prefix: Some(src) }
```

Notes: `read_u16`/`read_u32`/`read_seq` select endianness from `e`; every read checks
`body + n <= data.len()` and yields `None` (no panic) when short; a non-advancing or
out-of-range `len` breaks the loop.

### `frame_summary`

```
for each frame: "KIND" + ("(sn=" + sn + ")") + ("(first..last)" for heartbeat)
join with "; "
```

## Tests

- `rtps.rs` unit tests:
  - header: version/vendor/guid prefix.
  - DATA (little-endian): reader/writer entity ids, SN, length.
  - HEARTBEAT: `firstSN`/`lastSN`/count.
  - big-endian: lengths and SN read correctly.
  - inline-QoS DATA (`flags & 0x02`) skips `extraFlags`.
  - INFO_SRC updates `src_guid_prefix`.
  - `length == 0` extends to end.
  - truncated / unknown submessage: no panic, loop terminates.
  - non-RTPS input: `header` is `None`, `frames` empty.
- `capture.rs`: `record_from` on a crafted RTPS datagram sets `guid_prefix`/`entity_id`/`summary`/`frames`.
- `model.rs`: update the `record(..)` test helper for the new `frames` field; add a `frame_summary` test.

## Minimal test

```
.\scripts\build.ps1 -Test
.\scripts\traffic-throughput.ps1 -Seconds 5     # then press Start in dds-echo
```

Select a datagram: the detail pane lists its submessages (ids), the participant `guid`,
writer entity ids, and sequence numbers; the Summary column shows the submessage mix.
Release build and deploy via `scripts/build.ps1`.

## Risks and follow-ups

- **Malformed input:** the parser must never panic on short/adversarial datagrams; all reads are length-checked and tested.
- **Entity-id semantics** (which writer is SEDP vs user data, topic identification) belong to `dds-echo-sedp`; this story only decodes the frame envelope.
- **Perf:** parsing runs on the capture thread; revisit (move to `to_ui` or parse on select) only if raw capture throughput becomes a concern.

## Module layout

```
rust/tool/dds-echo/build.rs          Slint compile
rust/tool/dds-echo/ui/dds_echo.slint the UI (+ submessages list)
rust/tool/dds-echo/src/main.rs       module rtps, wiring, tick, to_ui
rust/tool/dds-echo/src/capture.rs    sockets, config, classification, Capture, record_from
rust/tool/dds-echo/src/rtps.rs       RTPS header + submessage parser
rust/tool/dds-echo/src/model.rs      Class/Record/Frame/Stats/TopicCounts/RateTracker + tests
```

## Result

Pending implementation.
