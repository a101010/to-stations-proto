# Current story: dds-echo-topic-filter

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make the left **Topics** pane a multi-select filter for the center datagram list. Clicking a topic row toggles its selection (background-highlighted); the center list then shows only datagrams whose topic matches any selected row (union), or all traffic when nothing is selected. The **Topics (packets)** title clears the selection and is always visible; a right-click context menu offers **Unselect All**. Also, the detail pane's spare height must sit below the hex, not between sections.

- **Depends on:** none.
- **Minimal test:** select `FakeTopicOne` (only its data datagrams remain), add `FakeTopicTwo` (both appear), click the title (all traffic returns), and the context menu's **Unselect All** also clears.

## Context

- `records: VecModel<ui::Record>` is the only store; `tick` appends each captured record and trims to `MAX_RECORDS`.
- `refresh_topics` rebuilds `ui::TopicRow { name, known, packets }` every tick; rows are not clickable and carry no selection.
- The center table has no filter; the detail pane is one `ScrollView` whose content has no trailing stretch, so spare height is distributed between sections.
- `TopicCounts::rows()` already yields `Known` + `Unknown` (including the `unidentified` row).

## Decisions

- Keep a backing `all_records` (the full, capped list) plus the filtered **view** `records`; append incrementally on capture, rebuild the view only when the selection changes.
- Selection is a set of row names; a record's row name is its topic, or `unidentified` when the topic is empty. Union (OR); empty selection = all.
- The **Topics (packets)** title is the always-visible clear header.
- The context menu is an in-window overlay (Slint has no built-in context menu): a right-click opens it at the cursor; a full-window transparent layer dismisses it.
- On filter change, reset `selected-index` (indices shift).
- No `Total` row.

## Interfaces

### `rust/tool/dds-echo/src/model.rs` (changed)

```rust
/// The topic rows selected in the topics pane. Empty = match everything.
#[derive(Clone, Debug, Default)]
pub struct TopicSelection {
    names: BTreeSet<String>,
}
impl TopicSelection {
    pub fn toggle(&mut self, name: &str);
    pub fn clear(&mut self);
    pub fn contains(&self, name: &str) -> bool;
    /// Empty selection matches everything; `None`/empty topic is `unidentified`.
    pub fn matches(&self, topic: Option<&str>) -> bool;
}
```

### `rust/tool/dds-echo/src/main.rs` (changed)

- `State` gains `selection: RefCell<TopicSelection>` and `all_records: RefCell<Vec<ui::Record>>`.
- `fn ui_topic(r: &ui::Record) -> Option<&str>` (empty topic -> `None`).
- `fn rebuild_records(view, all, selection)` — clear + push matching, cap at `MAX_RECORDS`.
- `fn apply_selection(app, state, records, known, unknown)` — `refresh_topics` + `rebuild_records` + reset `selected-index`.
- `refresh_topics` takes `selection` and sets `selected: selection.contains(&row.name)`.
- `tick` drain: build `to_ui`; push to `all_records`; if `selection.matches(record.topic.as_deref())` push to the view; trim both.
- New handlers: `toggle-topic(name)`, `clear-topics()`, `topic-menu(x, y)`, `dismiss-topic-menu()`, `unselect-all-topics()`.
- `Clear` also clears `all_records` + selection.

### `rust/tool/dds-echo/ui/dds_echo.slint` (changed)

- `TopicRow` gains `selected: bool`.
- `TopicGroup` gains `callback toggle-row(string)` and `callback topic-menu(length, length)`; each row is a `TouchArea` with a selected background and a `pointer-event` handler (left = toggle, right = open the menu at `absolute-position + mouse`).
- `MainWindow` gains the five callbacks and `topic-menu-visible/-x/-y`; the `Topics (packets)` title is a clickable `TouchArea`; a `TopicGroup` wires its callbacks to the window's.
- A top-level overlay `Rectangle` (shown when `topic-menu-visible`) hosts a dismiss `TouchArea` and the menu `Rectangle` with the **Unselect All** item.
- The detail content gains a trailing `Rectangle { vertical-stretch: 1; }`.

## Pseudocode

```
// tick, per captured record
selection = state.selection; all = state.all_records
ui = to_ui(record)
all.push(ui.clone()); trim(all)
if selection.matches(record.topic.as_deref()): view.push(ui); trim(view)

// on selection change
fn apply_selection():
    refresh_topics(counts.rows(), selection)
    view.clear()
    for r in all: if selection.matches(ui_topic(r)): view.push(r.clone())
    selected_index = -1
```

## Tests

- `model.rs`: `TopicSelection` — toggle add/remove, `clear`, `matches` for a named topic, `unidentified` (`None`/empty), and empty-selection = all.

## Minimal test

```
.\scripts\build.ps1 -Test
.\scripts\build.ps1
# then: build\deploy\bin\dds-echo.exe, Start, .\scripts\traffic-fake-topic.ps1 -Repeat 2
```

Expect `FakeTopicOne`/`FakeTopicTwo`/`unidentified` rows; selecting filters the center list (union) with highlighted rows; the title and the context menu's **Unselect All** both clear; spare detail height sits below the hex.

## Risks and follow-ups

- Overlay z-order and right-click handling in Slint (no native context menu).
- Rebuilding the view clones `ui::Record` (SharedString clones are cheap) and only happens on selection changes.
- Follow-the-tail still acts on the filtered view.

## Module layout

```
rust/tool/dds-echo/src/model.rs  + TopicSelection
rust/tool/dds-echo/src/main.rs   selection state, filtering, callbacks
rust/tool/dds-echo/ui/dds_echo.slint  row selection, title clear, context menu, detail stretch
```

## Result

Implemented. `model::TopicSelection` (toggle/clear/contains/matches) is the pure filter; `main` keeps a backing `all_records` and the filtered `records` view, appending matching records on capture and rebuilding on selection change via `apply_selection`; `refresh_topics` marks each `ui::TopicRow` selected. The Slint topics rows are clickable (left toggles, right opens the menu at the cursor), the `Topics (packets)` title clears the selection, and a right-click overlay menu offers **Unselect All**. The detail content gained a trailing `vertical-stretch: 1` rectangle so spare height falls below the hex. Files changed: `rust/tool/dds-echo/src/{model.rs,main.rs}` and `ui/dds_echo.slint`.

Verified: `scripts/build.ps1 -Test` passes with 42 `dds-echo` tests (1 new: `topic_selection_toggle_and_match`). Release build and deploy succeed. The interactive check (select/filter/unselect, the context menu, the detail spacing) remains for the user.
