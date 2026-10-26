# Current story: robust-dds-echo

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make `dds-echo` survive many concurrent high-rate streams (for example several 100 Hz topics), not just one: a bounded capture queue with frequency-first eviction that protects low-frequency and one-shot topics; separate visible `dropped` and `trimmed` counters; lazy hex/decoded formatting on selection rather than per record; a bounded history with a throttled view rebuild instead of per-record O(n) `remove(0)`; and no formatting of records the topic filter excludes. Reopened for real-use bug fixes: the stop check, virtualized `ListView`, and rebuild throttle are in place; scrolling up must pause live table updates and selection must stay responsive while paused.

- **Depends on:** none.
- **Minimal test:** with N concurrent 100 Hz writers plus a one-shot topic, the UI stays responsive, memory stays bounded, `dropped` and `trimmed` are reported, and the one-shot topic's records survive while high-frequency records are trimmed; pressing Stop under live traffic returns immediately; scrolling up freezes the table while stats continue, selecting a row is responsive, and scrolling to the bottom resumes live updates.

The story stays active until the user asks to plan the next story. Topic attribution remains the separate `dds-echo-discovery-attribution` story.

## Current state

The first pass and stop fix are committed:

- Bounded `CaptureQueue` with frequency-first eviction, singleton protection, writer-to-topic learning, and a `dropped` counter.
- `FrequencyBuffer`/`History` with frequency-ranked retention and a separate `trimmed` counter.
- Lazy row/detail formatting; `RecordRow` plus a virtualized `ListView`; 250 ms full-rebuild throttle.
- `recv_loop` observes the stop flag before receiving and after each datagram; continuous-sender regression test.

Remaining bug: follow-tail and selection responsiveness. The table keeps rebuilding and snapping to the bottom while the user scrolls up, so inspecting and selecting rows is awkward.

## Findings

- **Follow-tail cause.** The ListView's `content-y` was bound two-way to a Rust mirror, but the underlying `Flickable` removes content-y bindings on user scroll. The mirror can remain stale, so the Rust tick keeps forcing the list to the bottom and keeps rebuilding the table.
- **Selection responsiveness.** Rebuilding up to 1000 rows every 250 ms while the user is trying to read or select makes selection feel unresponsive even with a virtualized `ListView`.
- **Stop freeze.** Already fixed; the receive loop now observes stop during continuous traffic.
- The attribution problem is unrelated and remains the separate `dds-echo-discovery-attribution` story.

## Decisions

1. Keep the frequency basis, grouping, singleton protection, all-singleton fallback, caps, counters, lazy formatting, and history retention.
2. Keep the stop fix, `RecordRow`/`ListView`, and 250 ms rebuild throttle.
3. Move follow-tail state into Slint so it reads the real ListView position instead of the stale Rust mirror.
4. While the user is scrolled up, freeze the table entirely; keep draining records, history, stats, topic counts, and counters. Resume and rebuild when the user returns to the bottom.
5. Preserve selection by history id as before; a frozen table makes selection stable.
6. Do not move this story to `completed_stories.md` until the user explicitly asks to plan the next story.

## Plan

### Step 1 - Slint-side follow-tail

In `ui/dds_echo.slint`:

- Add `in-out property <bool> follow: true;`.
- Remove the two-way `content-y` binding and the `list-content-y` / `list-content-height` / `list-visible-height` mirror properties if they are no longer used.
- In the ListView's `scrolled` handler, compute the real position:

```slint
scrolled => {
    root.follow = list.content-y <= (list.visible-height - list.content-height) + 1px;
}
```

- Add a low-frequency timer that sticks to the bottom only while following:

```slint
Timer {
    interval: 100ms;
    running: root.follow;
    repeat: true;
    triggered => {
        if (root.follow) {
            list.content-y = min(0px, list.visible-height - list.content-height);
        }
    }
}
```

- Remove the `list-scrolled` callback if it is no longer needed.

Behavior: scrolling up sets `follow = false` and stops the timer; scrolling to the bottom sets `follow = true` and the timer keeps the list at the bottom.

### Step 2 - Rust table update policy

In `main.rs`:

- Remove `State.following`, the `on_list_scrolled` handler, and the tick's forced `set_list_content_y`.
- Add `pending_view: Cell<bool>` to `State`.
- In `tick`:
  - Always drain the queue and update history, stats, topic counts, discovery, and counters.
  - Set `pending_view` when records arrived or history was trimmed.
  - If `app.get_follow()` is false, do not rebuild the table; leave the displayed rows frozen.
  - If follow is true and `pending_view` is set and the 250 ms throttle elapsed, rebuild, clear `pending_view`, and update `last_view_ms`.
- Start and Clear set `app.set_follow(true)` and clear `pending_view`.
- Filter changes still rebuild immediately; when the user is scrolled up, the Slint timer will not auto-scroll because follow remains false.
- Keep selection by history id; while paused the model is stable, so row selection and detail should be immediate.

### Step 3 - Additional bugs

The user is still testing. Add any further observed symptoms to this story as bug fixes; do not split them out unless they belong to a separate subsystem (attribution remains separate).

### Step 4 - Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

1. Start live capture (view service or 100 Hz streams).
2. Scroll up: rows stop changing and the list stays where it was; stats/counters continue.
3. Select rows and open details while paused; selection and detail should be responsive.
4. Scroll to the bottom: live updates resume and the list follows the tail.
5. Press Stop while live: the UI returns immediately (`Responding=True`).
6. Run four concurrent 100 Hz streams plus a rare topic: memory plateaus, `dropped`/`trimmed` update, rare records survive.
7. Confirm scrolling, filtering, selection, and decoded detail still work.

## Tests

- Existing 61 `dds-echo` tests, including the continuous-sender Stop regression.
- Manual scroll-up/pause, scroll-to-bottom/resume, selection responsiveness, Stop-under-load, and memory plateau checks.
- Add a focused test if the follow logic is extracted into testable Rust; otherwise the Slint-side behavior is verified manually.

## Notes and risks

- The Slint timer and user scroll can race; the `follow` guard in the timer and the `scrolled` handler keep the user's scroll authoritative.
- While paused, history continues to accumulate; the table rebuild on resume shows the retained set and then follows the bottom.
- A filter change while paused rebuilds immediately; that is intentional, and follow stays false until the user scrolls to the bottom.
- Selection while follow is true still rebuilds at 250 ms; if it remains sluggish, the next step is to pause rebuilds while a detail is selected or reduce the cadence.
- Attribution remains separate; this story does not attempt to categorize `HelloWorldViewTopic`.

## Bookkeeping

Do not move this story to `completed_stories.md`. Keep it `in progress` until the user asks to plan the next story; then record completion and plan the next story.
