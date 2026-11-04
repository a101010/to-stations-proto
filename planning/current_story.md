# Current story: dds-echo-copy-detail

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make the `dds-echo` detail pane selectable read-only text so the user can select any portion and copy it with Ctrl+C. Render the selected record's time, protocol, sender (including `self`), participant and writer GUIDs, size, topic, submessages, decoded fields, and hex in one selectable text view. No copy button is added.

- **Depends on:** none.
- **Minimal test:** select a record, select sender lines in the detail pane, press Ctrl+C, and paste into an editor; the pasted text is the selected portion.

## Context

The detail pane is built from plain `Text` elements, so nothing can be selected or copied. Users need to copy sender information and decoded fields into reports and issues.

Slint's read-only `TextEdit` supports mouse selection and Ctrl+C; copy is accepted even when `read-only` is true. No clipboard crate or copy control is needed.

The selected record already exists in `state.history`; a single plain-text formatter can produce the detail view from `model::Record` and `KnownTopics`.

## Decisions

1. **One selectable detail view.** Replace the structured detail elements with a single read-only `TextEdit` containing the formatted detail text.
2. **Native copy only.** No copy button, no copy-sender/copy-all action. The user selects text and presses Ctrl+C.
3. **No clipboard dependency.** Do not add `arboard` or other clipboard crates.
4. **Plain-text formatter.** Generate one string from the selected `model::Record` and `KnownTopics`.
5. **TextEdit owns scrolling.** Remove the outer detail `ScrollView` to avoid nested scroll areas.
6. **Hex stays the displayed preview.** The text includes `Record::hex_string()` (the truncated preview), matching the current pane.
7. **No architecture change.** This is a `dds-echo` tool feature.

## Interfaces

### Rust formatter

Add in `rust/tool/dds-echo/src/main.rs`:

```rust
fn detail_text(r: &model::Record, known: &KnownTopics) -> String;
```

Output format:

```text
time: ...
protocol: ...
source: self (<address>)    // or the address for remote senders
participant: ...
writer: ...
size: ...
topic: ...
submessages: ...

submessages:
  <frame line>
  <frame announcement detail>
  ...

decoded:
  ...

hex:
  ...
```

Reuse the existing `frame_line`, `frame_detail`, `decoded_text`, and `Record::hex_string()` helpers.

### UI model

Add to `ui::Record`:

```slint
text: string,
```

- `to_detail` sets `text` from `detail_text(r, known)`.
- `empty_record` sets `text: ""`.
- Existing fields may remain for compatibility even if the detail pane no longer renders them separately.

### Detail UI (`ui/dds_echo.slint`)

- Import `TextEdit`.
- Replace the detail pane's `ScrollView` content with a read-only `TextEdit`:

```slint
VerticalLayout {
    padding: 8px;
    spacing: 4px;

    Text { text: "Detail"; color: #ffffff; font-weight: 700; }

    detail-text := TextEdit {
        text: root.selected.text;
        read-only: true;
        font-family: "Courier New";
        font-size: 12px;
        wrap: no-wrap;
        vertical-stretch: 1;
    }
}
```

The read-only `TextEdit` provides selection, Ctrl+A, Ctrl+C, and its own scrolling. No copy control is added.

## Implementation steps

1. **Planning bookkeeping.**
   - The completed `gui-single-instance` story has been moved to `completed_stories.md` with completion time `2026-11-04 13:48:29` and removed from `backlog.md`; `dds-echo-copy-detail` is the active story.
2. **Add the formatter.**
   - Implement `detail_text` in `main.rs`.
   - Include the self/remote source rule and all required sections.
3. **Extend the UI model.**
   - Add `text` to `ui::Record`.
   - Set it in `to_detail` and `empty_record`.
4. **Replace the detail pane.**
   - Import `TextEdit`.
   - Remove the detail `ScrollView` and replace its contents with the read-only `TextEdit`.
   - Do not add a copy button or copy-status line.
5. **Tests.**
   - Unit-test `detail_text` for all required sections.
   - Test a self-origin record and a discovery record.
   - Add a decoded-sample case if practical.
6. **Documentation.**
   - Add a short `README.md` note that detail text is selectable and Ctrl+C works.
7. **Verify once.**
   - `cargo test -p dds-echo`
   - `.\scripts\build.ps1 -Test`
   - `.\scripts\build.ps1`
   - One manual selection/Ctrl+C run.
8. **Record completion** in `current_story.md` while leaving the backlog status `in progress` until the user asks to plan the next story.

## Acceptance test

Automated, in `main.rs` tests:

- `detail_text` includes time, protocol, sender, participant, writer, size, topic, submessages, decoded fields, and hex.
- A self-origin record renders `source: self (<address>)`.
- A discovery record includes its frame/announcement lines.
- A record with a decodable payload includes decoded fields.

Manual, one run:

1. Start `dds-echo` and capture traffic.
2. Select a record.
3. Select the sender and GUID lines, press Ctrl+C, and paste; verify only that selection appears.
4. Select part of the hex or decoded text, press Ctrl+C, and paste.
5. Select a different record and verify the detail text updates.
6. Verify long detail content scrolls correctly.
7. Press Ctrl+A then Ctrl+C and verify the whole detail text is copied.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml -p dds-echo
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\build\deploy\bin\dds-echo.exe
```

- `build.ps1 -Test` must pass once.
- `build.ps1` must deploy once.
- One manual selection/Ctrl+C run.
- No screenshots and no repeated UI runs.

## Risks

- **TextEdit appearance:** the standard read-only `TextEdit` may differ visually from the surrounding dark pane; acceptable for this tool.
- **Scrolling:** the `TextEdit` owns scrolling; remove the outer detail `ScrollView` to avoid nested scrolling.
- **Large records:** the copied hex matches the truncated preview, not the full datagram.
- **Selection updates:** selecting a different record replaces the text and clears the old selection.
- **Read-only behavior:** Slint accepts Copy in read-only `TextEdit`; paste and cut remain disabled.

## Files touched

- `planning/current_story.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `rust/tool/dds-echo/src/main.rs`
- `rust/tool/dds-echo/ui/dds_echo.slint`
- `README.md`

## Result

Not started.

## Bookkeeping

- `gui-single-instance` was completed at `2026-11-04 13:48:29` and moved to `completed_stories.md`.
- `dds-echo-copy-detail` is `in progress` in `backlog.md`.
- Do not move `dds-echo-copy-detail` to `completed_stories.md` until the user asks to plan the next story.
