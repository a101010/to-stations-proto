# Current story: robust-dds-echo

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make `dds-echo` survive many concurrent high-rate streams (for example several 100 Hz topics), not just one: a bounded capture queue with frequency-first eviction that protects low-frequency and one-shot topics; separate visible `dropped` and `trimmed` counters; lazy hex/decoded formatting on selection rather than per record; a bounded history with a throttled view rebuild instead of per-record O(n) `remove(0)`; and no formatting of records the topic filter excludes. Reopened after real-world testing: the receive loop must observe stop during continuous traffic, and the table must be virtualized with its rebuild throttled so Stop does not block the UI.

- **Depends on:** none.
- **Minimal test:** with N concurrent 100 Hz writers plus a one-shot topic, the UI stays responsive, memory stays bounded, `dropped` and `trimmed` are reported, and the one-shot topic's records survive while high-frequency records are trimmed; pressing Stop under live traffic returns immediately.

Topic attribution remains the separate `dds-echo-discovery-attribution` story. Starting `dds-echo` first does not fix it: SEDP is not visible to a non-participant sniffer.

## Context

The first pass of this story is implemented:

- `capture.rs` has a bounded `CaptureQueue` with frequency-first eviction, singleton protection, a learned writer-to-topic index, and a `dropped` counter.
- `model.rs` has `FrequencyBuffer` and `History`, frequency-ranked retention with `MAX_RECORDS`/`HISTORY_MAX_BYTES`, and the separate `trimmed` count.
- `main.rs` drains a bounded batch per 100 ms tick, keeps raw history, formats lightweight rows, builds hex/decoded/frames only on selection, and preserves selection by history id.
- `ui/dds_echo.slint` shows `dropped` and `trimmed` chips.

Two problems remain after real-world use:

1. Stop blocks the UI under continuous traffic.
2. The table still rebuilds all retained rows on every dirty tick, which can starve input on slower machines.

## Findings

- **Stop freeze.** `recv_loop` checks the stop flag only in the `recv_from` timeout branch. Under continuous traffic `recv_from` keeps returning `Ok`, so the receive thread never sees the flag. `Capture::stop()` sets the flag and joins the receive threads; the joins never return, so the UI thread blocks inside `on_stop` and Windows reports "Not responding." This is independent of table cost.
- **View rebuild cost.** `rebuild_view` calls `records.set_vec` with all matching retained rows (up to 1000), and the Slint table is a `ScrollView` with a plain `for` over all rows, so every row component is instantiated. This runs on every dirty 100 ms tick.
- **Attribution.** Passive SEDP capture is not sufficient even when `dds-echo` starts first; this is the separate follow-up story.
- **Memory.** The queue/history caps are working; memory plateaus and is not the freeze cause.

## Decisions

1. Keep the implemented frequency basis, grouping key, singleton protection, all-singleton fallback, caps, counters, lazy formatting, and frequency-ranked history.
2. Check `stop` at the top of every receive-loop iteration and again after handling each datagram.
3. Keep the read timeout as the idle-loop bound; it also bounds Stop latency when the socket is idle.
4. Add a lightweight `RecordRow` for the table and use `ListView` so only visible rows are instantiated.
5. Keep the full `Record` only for the selected detail.
6. Rebuild the table at most every `VIEW_REBUILD_MS = 250`; keep draining, stats, and topic counts on the 100 ms tick. Rebuild immediately on filter changes and Start/Clear.
7. Preserve selection by history id through `displayed_ids`.

## Plan

### Step 1 - Capture shutdown (priority 1)

In `capture.rs::recv_loop`, make the stop flag authoritative:

```rust
loop {
    if stop.load(Ordering::Relaxed) {
        break;
    }
    match udp.recv_from(&mut buf) {
        Ok((n, src)) => {
            // ... build and push the record ...
            if stop.load(Ordering::Relaxed) {
                break;
            }
        }
        Err(e) if WouldBlock/TimedOut => {
            if stop.load(Ordering::Relaxed) {
                break;
            }
        }
        Err(e) => { eprintln!(...); break; }
    }
}
```

`Capture::stop` keeps setting the flag and joining. A continuously busy socket exits before the next `recv_from`; an idle socket exits within one read timeout.

Add a regression test:

- Bind a capture on an ephemeral unicast port.
- Start a sender thread that sends RTPS datagrams continuously until told to stop.
- Call `Capture::stop()` from another thread and signal completion over a channel.
- Wait at most one second; on timeout, stop the sender and fail the test.

Check: pressing Stop under live 100 Hz traffic returns immediately; pressing Stop on an idle capture returns within the read timeout.

### Step 2 - Virtualized, lightweight table

In `ui/dds_echo.slint`:

- Add a `RecordRow` struct with `time`, `protocol`, `source`, `size`, `topic`, `submessages`, and `known`.
- Change `records` to `[RecordRow]`.
- Replace the table's `ScrollView` + `VerticalLayout` + `for` with a `ListView` whose only child is the single `for` loop. Keep the header, row height, styling, selection highlight, and `TouchArea` click. Keep follow-the-tail properties bound to the ListView.
- Keep the detail pane's `selected` model as the existing full `Record`.

In `main.rs`:

- Use `VecModel<ui::RecordRow>` for the table.
- `to_row` returns `ui::RecordRow` and no longer creates hex, decoded, or frame data.
- `to_detail` keeps building the full `ui::Record` on selection.
- `displayed_ids` stays aligned for selection lookup.

### Step 3 - Throttled rebuild

- Add `const VIEW_REBUILD_MS: u64 = 250;`.
- Add `last_view_ms: Cell<u64>` to `State`, reset on Start.
- In `tick`, rebuild only when `dirty` and `now_ms - last_view_ms >= VIEW_REBUILD_MS`; update `last_view_ms`.
- Rebuild immediately for topic filter changes, unselect-all, Start, and Clear.
- Keep stats, rates, topic counts, and `dropped`/`trimmed` on the 100 ms tick.

### Step 4 - Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

1. Start `build\deploy\bin\dds-echo.exe`; press Start.
2. Send `.\scripts\traffic-fake-topic.ps1` once, then run four concurrent 100 Hz streams:
   `traffic-throughput.ps1 -Seconds 60 -Payload 1024 -IntervalMs 10 -Burst 1`.
3. Confirm the rare topic's records remain while high-frequency rows churn; scrolling, selecting, and filtering stay responsive.
4. Press Stop while traffic is live: the UI returns immediately and shows Stopped; repeated Stop clicks never produce "Not responding."
5. With an idle capture, Stop returns within the read timeout.
6. Sample `Get-Process dds-echo | Select WorkingSet64` every 5 seconds; memory plateaus.
7. Confirm `trimmed` rises separately from `dropped` once history is full.
8. Stress with `traffic-throughput.ps1 -Seconds 20 -Payload 60000 -IntervalMs 1 -Burst 100`; confirm `dropped` can rise and rare records still survive.
9. Stop traffic with `Get-Process ThroughputPublisher,ThroughputSubscriber | Stop-Process -Force`.

## Tests

- Continuous-sender Stop regression test in `capture.rs` with a bounded join timeout.
- Existing `dds-echo` unit tests continue to pass.
- Manual Stop-under-load and idle-Stop checks.
- Manual table scroll/selection/detail checks after the `ListView` conversion.

## Notes and risks

- `ListView` accepts only a single `for` child; the header stays outside it. Follow-tail behavior must be rechecked against the ListView scroll properties.
- Stop can take up to one read timeout (200 ms) on an idle socket; that is acceptable and does not freeze the window.
- Throttling delays table updates to 250 ms while stats stay at 100 ms; this is intentional to keep input responsive.
- Attribution remains `unidentified` until `dds-echo-discovery-attribution`; this story does not attempt to fix it.
- The table continues to show the frequency-retained history, not a separate live-tail mode.

## Bookkeeping

When this story passes, move it from `backlog.md` to `completed_stories.md` with a fresh completion timestamp, then plan `dds-echo-discovery-attribution`.
