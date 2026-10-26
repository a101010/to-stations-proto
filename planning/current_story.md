# Current story: robust-dds-echo

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make `dds-echo` survive many concurrent high-rate streams (for example several 100 Hz topics), not just one: a bounded capture queue with an explicit max buffer size and frequency-first eviction that protects low-frequency and one-shot topics; separate visible `dropped` and `trimmed` counters; lazy hex/decoded formatting on selection rather than per record; a bounded history with a throttled view rebuild instead of per-record O(n) `remove(0)`; and no formatting of records the topic filter excludes.

- **Depends on:** none.
- **Minimal test:** with N concurrent 100 Hz writers plus a one-shot topic, the UI stays responsive, memory stays bounded, `dropped` and `trimmed` are reported, and the one-shot topic's records survive while high-frequency records are trimmed.

## Context

`dds-echo` is a Slint GUI. Capture threads receive UDP datagrams and parse RTPS/SEDP (`capture.rs`), the UI drains them on a 100 ms timer and maintains stats, topic counts, discovery, and the record table (`main.rs`), and Slint models hold the displayed rows (`ui/dds_echo.slint`). `model.rs` is UI-agnostic and unit-tested. The passive-capture attribution limitation is split out to `dds-echo-discovery-attribution`; unattributed streams still group per writer for retention.

## Findings

- `capture.rs:262-287`: every receive thread parses each datagram and pushes a `Record` into an **unbounded** `mpsc` channel. There is no cap and no dropped count.
- `main.rs:240-271`: each tick drains the entire channel and, per record, runs attribution, `decode_record`, `to_ui`, `hex_string`, frame formatting, `all_records.push`, and `remove(0)`.
- `main.rs:73-74`: `all_records` holds fully formatted rows, including hex/decoded text and frames.
- `main.rs:165-177`: `rebuild_records` uses `view.remove(0)` per row over the cap.
- `main.rs:140-154`: `decode_record` runs before the topic filter is applied.
- `model.rs:107-110`: `Record.decoded` is filled eagerly; `hex_string()` is called for every displayed record.
- `ui/dds_echo.slint:128-134`: stats have no dropped or trimmed counters.
- Load verification already exists: `scripts/traffic-throughput.ps1` (100 Hz with `-IntervalMs 10 -Burst 1`) and `scripts/traffic-fake-topic.ps1` (rare topics and known samples).

## Decisions

1. **Frequency basis:** the current buffered count per key, not cumulative counts or a rolling rate.
2. **Grouping key:** attributed topic, else writer GUID, else source address. The capture queue learns writer-to-topic from SEDP before UI attribution; unattributed streams group per writer.
3. **Frequency-first eviction:** while over a cap, evict the oldest record of the key with the most buffered records. Never evict a key with one buffered record while any key has two or more.
4. **All-singleton fallback:** no separate quota. If every buffered key has one record and the buffer is still over its hard cap, evict the globally oldest record; the hard cap wins.
5. **Counters:** `dropped` counts capture-queue evictions (records the UI never observed, so not in stats/counts/history). `trimmed` counts history evictions (records already observed and still counted in stats/topic counts, but removed from the table). They are separate chips and reset on Start and Clear.
6. **Formatting:** build lightweight table rows for matching records; build hex/decoded/frames only in the detail pane on selection.
7. **View updates:** rebuild the Slint view with `VecModel::set_vec` at most once per dirty tick, never per record; rebuild immediately on filter changes.
8. **Caps:** `QUEUE_MAX_RECORDS = 4096`, `QUEUE_MAX_BYTES = 32 MiB`, `DRAIN_MAX_PER_TICK = 2048`, history `MAX_RECORDS = 1000` and `HISTORY_MAX_BYTES = 64 MiB`.

## Design

### `FrequencyBuffer<T>` (`model.rs`)

```rust
pub struct FrequencyBuffer<T> {
    entries: BTreeMap<u64, (String, T)>,       // id -> (key, value), arrival order
    by_key: HashMap<String, VecDeque<u64>>,    // per-key ids, oldest first
    max_records: usize,
    max_bytes: usize,
    bytes: usize,
}

impl<T> FrequencyBuffer<T> {
    pub fn push(&mut self, key: String, size: usize, value: T) -> Vec<(u64, T)>;
    pub fn drain_batch(&mut self, max: usize) -> Vec<(u64, T)>;
    pub fn iter(&self) -> impl Iterator<Item = (u64, &T)>;
    pub fn find(&self, id: u64) -> Option<&T>;
    pub fn clear(&mut self);
}
```

Eviction loop while over either cap:

1. Pick the key with the largest `by_key[key].len()` that has at least two buffered records.
2. Pop that key's oldest id.
3. If no key has two or more records, pop the globally oldest entry (`entries.pop_first()`).
4. Remove the entry from `entries`, decrement `bytes`, and remove the id from `by_key`.

Draining removes globally oldest records, so retained rows stay in arrival order. Finding the maximum is linear in the number of distinct keys; that is small for DDS topics and can be indexed later if a workload produces thousands of distinct keys.

### Capture queue (`capture.rs`)

```rust
struct QueueState {
    buffer: FrequencyBuffer<Record>,
    writer_topic: HashMap<String, String>, // learned from SEDP announcements
}

pub struct CaptureQueue {
    inner: Mutex<QueueState>,
    dropped: AtomicU64,
}
```

- `push` learns writer topics from `frame.announcement`, computes the key, pushes into the frequency buffer, and increments `dropped` for each record the queue evicts.
- `drain(max)`, `dropped()`, and `clear()`; `clear` discards pending records without counting them as dropped and resets the writer-topic index.
- `Capture::start` takes `Arc<CaptureQueue>`; receive threads push instead of sending on a channel.
- The key index is not lost when a record is evicted, so later data from that writer is still grouped by its topic.

### History (`model.rs` / `main.rs`)

- `History` wraps `FrequencyBuffer<Record>` capped at `MAX_RECORDS` (1000) and `HISTORY_MAX_BYTES` (64 MiB), and evicts frequency-first.
- `History::push` computes the key (`record.topic`, else `record.writer_guid()`, else `record.src_addr`) and returns the evicted entries so `main` can add their count to `trimmed`.
- `History::matching(&selection)` filters before any row formatting.
- Remove `Record.decoded`; decoding happens on selection.

### UI flow (`main.rs`)

- State: `queue: RefCell<Option<Arc<CaptureQueue>>>`, `history: RefCell<History>`, `displayed_ids: RefCell<Vec<u64>>` (aligned with the visible rows), `dropped: Cell<u64>`, `trimmed: Cell<u64>`, `selected_id: Cell<Option<u64>>`.
- Start: create the queue, reset history/view/stats/counters, reset the dropped display.
- Tick: drain at most `DRAIN_MAX_PER_TICK`; for each record run discovery attribution, stats, and topic counts, then push into history and add the returned eviction count to `trimmed`; update `dropped` from the queue.
- Rebuild the view once per dirty tick, collecting rows with `to_row` (table fields only) and their history ids together, then `records.set_vec(rows)` and replace `displayed_ids`. Excluded records are never passed to `to_row`.
- `apply_selection` rebuilds immediately on topic toggles, clear, and unselect-all.
- `on_select`: read `displayed_ids[index]`, find the raw record in history, build `to_detail` (hex, decoded, frames), and set it as the detail pane.
- Preserve `selected_id` across rebuilds; set `selected_index` to its new position, or clear the detail and index when it was trimmed.
- Clear: clear the pending queue, history, models, stats, counts, discovery, dropped, trimmed, selection, and the detail.
- Stop: stop threads, read the final `dropped` count, drop the queue, and leave the history and counters visible.

### UI (`ui/dds_echo.slint`)

- Add `in-out property <int> dropped: 0;` and `in-out property <int> trimmed: 0;`.
- Add `StatChip`s for `dropped` and `trimmed` next to `packets`.
- `Record` needs no new field; selection maps the visible row index through `displayed_ids`.
- The per-topic trimmed breakdown is not included; `trimmed` is the separate global counter.

## Plan

### Step 1 - Retention buffer and history

- Add `FrequencyBuffer` and `History` to `model.rs`, remove `Record.decoded`, and update the existing test helpers.
- Tests: high-frequency versus singleton retention, all-singleton fallback, byte cap, arrival order, filter matching, `find`, `clear`.

### Step 2 - Bounded capture queue

- Replace the channel with `CaptureQueue` and the writer-topic index; update `Capture::start` and `recv_loop`.
- Update the capture tests to poll the queue instead of `recv_timeout`.
- Tests: count cap, byte cap, frequency-first eviction, writer-topic keying, dropped count, FIFO drain, clear.

### Step 3 - UI state, lazy formatting, throttled rebuild

- Replace `rx`/`all_records`/`decode_record`/`to_ui`/`rebuild_records` with the queue, history, `to_row`, `to_detail`, and `rebuild_view`.
- Wire the `dropped` and `trimmed` counters through Start, tick, Stop, and Clear.
- Keep stats/counts/discovery observing every drained record, including records later trimmed from history.

### Step 4 - Slint changes

- Add the `dropped` and `trimmed` properties and chips.
- Rebuild the generated bindings through `scripts/build.ps1`; no changes to `build.ps1` or dependencies.

### Step 5 - Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

1. Start `build\deploy\bin\dds-echo.exe`; press Start.
2. Send `.\scripts\traffic-fake-topic.ps1` once, then run four concurrent 100 Hz streams:
   `traffic-throughput.ps1 -Seconds 60 -Payload 1024 -IntervalMs 10 -Burst 1`.
3. Confirm the rare topic's rows remain while high-frequency rows churn; scrolling, selecting, and filtering stay responsive.
4. Sample `Get-Process dds-echo | Select WorkingSet64` every 5 seconds; memory plateaus.
5. Confirm `trimmed` rises separately from `dropped` once history is full.
6. Stress with `traffic-throughput.ps1 -Seconds 20 -Payload 60000 -IntervalMs 1 -Burst 100`; confirm `dropped` can rise and rare records still survive.
7. Stop traffic with `Get-Process ThroughputPublisher,ThroughputSubscriber | Stop-Process -Force`.

## Notes and risks

- Full `set_vec` rebuild once per dirty tick is the throttle; if profiling shows it is too costly at 1000 rows, switch to incremental appends plus periodic trims without changing the retention model.
- The capture thread still parses RTPS and copies raw bytes per datagram; this story targets UI responsiveness and bounded memory.
- Unattributed streams group per writer because of the passive-capture limitation; proper attribution is the `dds-echo-discovery-attribution` follow-up.
- A buffer consisting entirely of one-shot topics still evicts the oldest one when the hard cap is reached; no separate rare-topic quota.
