# Current story: gui-single-instance

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add a reusable Windows named-mutex single-instance guard for manually launched singleton GUI tools, and use it in `dds-echo`. A second `dds-echo` launch restores and foregrounds the existing window, then exits without creating a DDS participant or window.

- **Depends on:** none.
- **Minimal test:** start `dds-echo`, minimize it, and launch a second `dds-echo`; the first window returns to the foreground, the second process exits, and only one `dds-echo` process and one DDS participant remain.

## Context

The duplicate `self` discovery rows seen in `dds-echo` came from two running `dds-echo` processes, not from one message being captured twice. Each process is its own DDS participant and emits its own SPDP/SEDP. The current `self` labeling is correct: only the current process's participant is `self`; another process is a genuinely different participant.

`cleanup.ps1` does not stop `dds-echo`, so a stale inspector can survive and create confusing duplicate discovery. A single-instance guard prevents this class of confusion without changing capture or self-labeling semantics.

The guard is useful to manually launched singleton GUI tools. It must not be used by fleet-launched applications or per-station/per-display windows, where multiple instances are legitimate.

## Decisions

1. **Reusable library.** Put the guard in a new shared crate `rust/lib/single-instance` so future manually launched singleton GUI tools can use the same behavior. `dds-echo` is the first consumer.
2. **Per-session named mutex.** Use `Local\to-stations-<tool>` so one instance is allowed per Windows session. Do not use `Global\`; it would require broader rights and block across sessions.
3. **Title-based activation.** The window title is the activation key. `dds-echo` uses the existing title `dds-echo`. Future tools must keep a stable title.
4. **Guard after argument parsing.** Acquire the mutex after `clap` has parsed arguments (so `--help` and `--version` still work) and before opening the discovery log, resolving configuration, creating a DDS participant, or creating the Slint window.
5. **Duplicate behavior.** On an existing mutex, close the duplicate's handle, restore and foreground the existing window, then exit with success. The duplicate creates no log entries, DDS resources, writers, or window.
6. **Fail open on unexpected errors.** If mutex creation fails for a reason other than “already exists”, report to stderr and start normally rather than blocking the tool.
7. **No fleet use.** Do not use the guard for `lil-d`, `big-d`, `paws`, the services, or anything started by a launch script. Those can legitimately have multiple instances.
8. **No change to self labeling.** The guard eliminates accidental duplicate instances; it does not relabel other participants as `self`.

## Interfaces

### `single-instance` crate

Proposed public API:

```rust
pub enum Acquire {
    Primary(SingleInstance),
    Duplicate,
}

pub fn acquire(tool_name: &str) -> Acquire;
pub fn activate_existing_window(window_title: &str);
```

- `SingleInstance` is an RAII guard holding the Windows mutex handle; `Drop` calls `CloseHandle`.
- `acquire` derives the mutex name as `Local\to-stations-<tool_name>`.
- On `ERROR_ALREADY_EXISTS`, the duplicate handle is closed immediately and `Duplicate` is returned.
- On non-Windows targets, `acquire` always returns `Primary` with a no-op guard and `activate_existing_window` does nothing.
- `activate_existing_window` is best-effort and never panics.

### Windows API surface

The crate uses `windows-sys` 0.59 with these features:

- `Win32_Foundation` — `GetLastError`, `CloseHandle`, `ERROR_ALREADY_EXISTS`
- `Win32_Security` — required by the `CreateMutexW` signature
- `Win32_System_Threading` — `CreateMutexW`, `GetCurrentThreadId`, `AttachThreadInput`
- `Win32_UI_WindowsAndMessaging` — `FindWindowW`, `ShowWindow`, `IsIconic`, `GetForegroundWindow`, `GetWindowThreadProcessId`, `SetForegroundWindow`, `BringWindowToTop`, `SW_RESTORE`

The mutex is created with a null security descriptor and `bInitialOwner = FALSE`.

### Window activation

`activate_existing_window(title)`:

1. Find the existing top-level window with `FindWindowW(null, title)`.
2. Retry briefly (for example 40 attempts at 50 ms) because the first instance may still be creating its window.
3. If found:
   - `ShowWindow(hwnd, SW_RESTORE)` when minimized.
   - `SetForegroundWindow(hwnd)`.
   - `BringWindowToTop(hwnd)`.
   - If `SetForegroundWindow` fails, attach the duplicate's input thread to the foreground thread with `AttachThreadInput`, call `SetForegroundWindow` again, then detach.
4. If no window appears within the retry window, return; the duplicate still exits.

### `dds-echo` wiring

In `rust/tool/dds-echo/src/main.rs`, immediately after `Args::parse()`:

```rust
let args = Args::parse();
let _instance = match single_instance::acquire("dds-echo") {
    Acquire::Primary(guard) => guard,
    Acquire::Duplicate => {
        single_instance::activate_existing_window("dds-echo");
        return Ok(());
    }
};
```

- The guard lives until `main` returns.
- Do not open `DiscoveryLog` or touch `AppResources` before this check.
- Keep the existing `windows_subsystem` behavior; the duplicate exits without a window.

### Workspace and dependencies

- Add `rust/lib/single-instance` to the `rust/Cargo.toml` workspace members.
- Add `single-instance = { path = "lib/single-instance" }` to `[workspace.dependencies]`.
- Add `single-instance = { workspace = true }` to `rust/tool/dds-echo/Cargo.toml`.
- The new crate uses `windows-sys` as a Windows-only dependency with the features above.

## Implementation steps

1. **Planning bookkeeping.**
   - `dds-echo-send` has been moved to `completed_stories.md` with completion time `2026-11-04 13:23:16` and removed from `backlog.md`.
   - `gui-single-instance` is the active story in `backlog.md`.
   - `dds-echo-copy-detail` remains `todo` and no longer depends on the completed `dds-echo-send`.
2. **Create the `single-instance` crate.**
   - Add `Cargo.toml` and `src/lib.rs`.
   - Implement the no-op non-Windows path and the Windows named-mutex path.
3. **Implement acquisition.**
   - Encode the mutex name as UTF-16.
   - Call `CreateMutexW`.
   - Check `GetLastError()` for `ERROR_ALREADY_EXISTS`.
   - Return `Primary` with the handle guard or `Duplicate` after closing the handle.
   - On unexpected failure, print a warning and return a no-op primary guard.
4. **Implement activation.**
   - `FindWindowW` with the title.
   - Retry for a short window.
   - Restore, foreground, and top the window.
   - Use `AttachThreadInput` as the fallback for foreground restrictions.
5. **Wire `dds-echo`.**
   - Add the dependency.
   - Insert the acquire/activate block after `Args::parse()` and before all runtime setup.
   - Hold the guard for the process lifetime.
6. **Architecture note.**
   - Record the rule in `planning/architecture.md`: manually launched singleton GUI tools use the guard; fleet-launched and per-station/per-display applications do not.
7. **Tests and verification.**
   - Add the mutex unit test (see below).
   - Run `cargo test -p single-instance`.
   - Run `.\scripts\build.ps1 -Test` and `.\scripts\build.ps1`.
   - Perform the manual duplicate-launch test.

## Acceptance test

Automated, in `rust/lib/single-instance`:

- Acquire a unique mutex name: first call returns `Primary`.
- While the first guard is alive, a second acquire with the same name returns `Duplicate`.
- After dropping the first guard, a third acquire returns `Primary`.
- Use a process-unique name so parallel tests do not collide.

Manual, one run:

1. Start `dds-echo` from `build/deploy/bin`.
2. Minimize the window.
3. Launch a second `dds-echo`.
4. Verify the first window restores and comes to the foreground.
5. Verify the second process exits and no second DDS participant appears in Capture.
6. Verify only one `dds-echo.exe` process remains.
7. Start `dds-echo --help` while the first instance runs; help still prints and exits.
8. Start a second instance with the first window already foreground; it exits and the first stays focused.

## Verification

```powershell
cargo test --manifest-path rust/Cargo.toml -p single-instance
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\build\deploy\bin\dds-echo.exe
```

- The mutex unit test must pass.
- `build.ps1 -Test` must pass once.
- `build.ps1` must deploy once.
- One manual duplicate-launch run.
- No screenshots and no repeated UI runs.

## Risks

- **Foreground restriction:** Windows can refuse `SetForegroundWindow`; `AttachThreadInput` plus `SW_RESTORE` is the fallback. If activation still fails, the duplicate exits and the existing instance can be selected from the taskbar.
- **Window not yet created:** if the first instance is still starting, the retry window covers normal startup; if it times out, the duplicate exits without foregrounding it.
- **Title coupling:** activation uses the window title. Future tools must keep stable titles and pass them to `activate_existing_window`.
- **Mutex scope:** `Local\` allows one instance per Windows session, not per machine. Two logged-in sessions can each run one instance.
- **Mutex failure:** an unexpected `CreateMutexW` failure fails open, so a guard failure never prevents the tool from starting.
- **No per-domain/per-interface support:** the guard is per tool, so two inspectors on different DDS domains/interfaces cannot run simultaneously.
- **Fleet misuse:** applying the guard to fleet/per-station windows would incorrectly block legitimate multiple instances; the architecture note records the boundary.

## Files touched

- `planning/current_story.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/architecture.md`
- `rust/Cargo.toml`
- `rust/lib/single-instance/Cargo.toml` (new)
- `rust/lib/single-instance/src/lib.rs` (new)
- `rust/tool/dds-echo/Cargo.toml`
- `rust/tool/dds-echo/src/main.rs`

## Result

Not started.

## Bookkeeping

- `dds-echo-send` was completed at `2026-11-04 13:23:16` and moved to `completed_stories.md`.
- `gui-single-instance` is `in progress` in `backlog.md`.
- `dds-echo-copy-detail` is the planned next story in `backlog.md`.
- Do not move `gui-single-instance` to `completed_stories.md` until the user asks to plan the next story.
