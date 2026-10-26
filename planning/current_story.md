# Current story: hello-view-service-fixes

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Debug and fix the problems seen when running the Rust configuration after `hello-view-service`: `lil-d` opens behind every other window; duplicate services/displays get launched; the view samples are all `unidentified`; and the earth's north-pole point moves on screen instead of the earth spinning about its tilted polar axis.

- **Depends on:** none.
- **Minimal test:** after `cleanup.ps1` and a single `launch-rust.ps1`, the `lil-d` window opens in front; exactly one `hello-view-service` and one `lil-d` run; the earth spins about a fixed tilted pole (the north-pole point is stationary); view samples are categorized (or their non-categorization is understood).

The `dds-echo`-under-load work is split out to `robust-dds-echo`. The attribution fix is split out to `dds-echo-discovery-attribution`.

## Context

`hello-view-service` publishes `HelloWorldViewTopic` at 100 Hz best-effort (a permanent 23.5 deg axial tilt, spun about the tilted polar axis, one revolution per 12 minutes); the display library subscribes on domain 0 and renders the received greeting/attitude (black until a sample arrives). Working directories: `rust/ui/lil-d`, `rust/lib/display-lib`, `rust/service/hello-view-service`, `rust/tool/dds-echo`, `scripts/`.

## Findings

Runtime inspection of a live launch:

```
hello-view-service 26916   (no window)
hello-view-service 28680   (no window)
lil-d              34560   window "lil-d"
lil-d              37276   window "lil-d"
dds-echo           19892   window "dds-echo"
```

- **Duplicates.** There were two `hello-view-service` and two `lil-d` processes: `launch-rust.ps1` was run repeatedly and nothing stops a previous run, so the streams stack and discovery is doubled.
- **`lil-d` window exists but is behind everything.** Both `lil-d` processes have a top-level window ("lil-d", nonzero `MainWindowHandle`) but it opens behind the two service consoles and dds-echo. `gl_window.rs:38` creates the window `with_visible(false)`; `app.rs:84-87` calls `set_visible(true)` + `request_redraw()` and nothing else. `lil-d` is a GUI app (`main.rs:8`), so the two *service* consoles (console subsystem) appear first and hold the foreground.
- **All `unidentified`.** The view `DATA` comes from a writer whose topic dds-echo never learned. `capture.rs:42-50` binds only the multicast group on ports `7400/+1`, with no DDS participant, and `model.rs:300-327` attributes DATA by the writer GUID learned from SEDP. SEDP is directed to matched readers and is not replayed to a late-joining passive sniffer, so starting `dds-echo` after the service and display have already discovered each other misses the announcement that names the view writer; its DATA then stays unattributed.
  - No configuration option forces SEDP onto multicast: `AllowMulticast=spdp` forces unicast endpoint discovery (`C:\Libraries\src\cyclonedds\etc\cyclonedds.xsd:275`).
  - Binding the participants' unicast metatraffic ports (`7410 + 2*participantIndex`) is unsafe: `SO_REUSEADDR` delivers each unicast datagram to one socket, so capture can steal discovery from the real participant, and the participant index is not fixed.
- **Rotation (north-pole point appeared to move) — resolved, not a bug.** The pole marker (Step 0) showed the object `+Y` cap fixed while the earth spins about it: the rotation is correct. The perceived tumbling came from the tilt leaning 45 deg to the left, which put the pole point near the limb, where the geometry crossing behind it read as motion. The source is correct — the service sends `R0·spin` about the earth's `+Y` (`view.rs:61-65`), the display uploads column-major `R` with `transpose=false` and the shader computes `u_attitude * a_dir` (`earth.rs:42,144,149,248-264`), and the cube map maps north to `+Y` (`cubemap.rs:45-55`, `CREDITS.md:45`). Orientation changed to lean straight toward the viewer so the pole point is clearly visible (see Step 3).

## Decisions

1. **Launch hygiene:** `launch-rust.ps1` calls `cleanup.ps1` before starting, reusing its image list instead of duplicating it.
2. **Window ordering:** after showing the window, call `focus_window()` (winit's Windows force-activation path) and request critical user attention as the fallback.
3. **Rotation:** keep the two-angle pole placement and 12-minute spin; add a permanent display-side convention test. Keep 100 Hz.
4. **Attribution:** accept the passive-capture limitation for this story, document it, and open `dds-echo-discovery-attribution` as the follow-up.

## Plan

### Step 1 - Launch hygiene (priority 1)

In `scripts/launch-rust.ps1`, after `$ErrorActionPreference = 'Stop'` and before the build-path check and any `Start-Process`, stop the previous run:

```powershell
# Stop a previous run first: repeated launches must not stack services/displays
# (which doubles streams and DDS discovery).
& (Join-Path $PSScriptRoot 'cleanup.ps1')
```

Keep the remaining logic (path check, `CYCLONEDDS_URI`, 500 ms delay, `lil-d --station N`). Update the stale comment in `scripts/cleanup.ps1:9-10` that says the Rust launch script starts only `lil-d`.

Tradeoff: launching one configuration stops another configuration's processes too. That matches "cleanup stops all processes for all configurations" and the configurations would collide on DDS anyway. If per-configuration stops are needed later, factor the app list into a shared script then.

Check: two consecutive `launch-rust.ps1` runs leave exactly one `hello-view-service` and one `lil-d`.

### Step 2 - Window ordering (priority 1)

In `rust/ui/lil-d/src/app.rs::setup`, after the hidden first paint, show, raise, and focus:

```rust
if let Some(gl_window) = self.gl_window.as_ref() {
    let window = gl_window.window();
    window.set_visible(true);
    window.request_redraw();
    window.focus_window();
    window.request_user_attention(Some(winit::window::UserAttentionType::Critical));
}
```

`focus_window` on Windows already uses winit's `force_window_active` (the Alt-key `SendInput` + `SetForegroundWindow` path), so it should override the foreground lock; the attention request flashes the taskbar if it still fails and is a no-op once focused. Keep `with_visible(false)` and the hidden first paint to avoid the startup flicker.

Check: after `cleanup.ps1` and a single `launch-rust.ps1`, `lil-d` is the foreground window.

### Step 3 - Rotation convention test (priority 3)

No rotation fix: the pole's view-space direction is set in `rust/service/hello-view-service/src/view.rs` by `POLE_VIEW_ANGLE_DEG = 55` and `POLE_SCREEN_TILT_DEG = 45`; `pole_in_view() = [-0.579, 0.579, 0.574]`, the whole arctic circle is visible with ~11.5 deg beyond it, and the pole sits ~45 deg up-left. Lighting is unchanged (upper right, `LIGHT_DIR`). The initial sub-camera point is centered on North America (~35 deg N, 100 deg W); `initial_attitude` uses the cube map's longitude convention `lon = atan2(x, z)`.

Add a permanent display-side test in `rust/lib/display-lib/src/earth.rs` pinning the convention (the existing test only covers a Z rotation). For a tilted quaternion, the columns of `quat_to_mat3` must equal the rotated basis vectors:

```rust
#[test]
fn quat_to_mat3_columns_are_rotated_basis_vectors() {
    let axis = normalize3([1.0, 1.0, 1.0]);
    let (s, c) = (0.3_f32).sin_cos(); // 30 degrees
    let q = [axis[0] * s, axis[1] * s, axis[2] * s, c];
    let m = quat_to_mat3(q);
    for (i, e) in [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]].iter().enumerate() {
        let want = rotate(q, *e); // local f32 test helper with the same convention
        for r in 0..3 {
            assert!((m[i * 3 + r] - want[r]).abs() < 1e-5);
        }
    }
}
```

Keep the view service at 100 Hz. The visual check remains the real one: the arctic cap stays fixed while the continents rotate.

### Step 4 - Attribution limitation (accepted)

No `dds-echo` behavior change in this story. Document the limitation in the module docs of `rust/tool/dds-echo/src/capture.rs` (and/or the `Discovery` docs in `model.rs`): endpoint attribution depends on observing SEDP during capture; SEDP is directed to matched readers and is not replayed to a late-starting passive sniffer, so samples from endpoints discovered before capture starts stay `unidentified`.

Optionally confirm the explanation during verification: start `dds-echo` and press Start before `launch-rust.ps1`; note whether `HelloWorldViewTopic` becomes attributed. Record the result in Findings.

The fix lives in the backlog as `dds-echo-discovery-attribution`: a discovery-only participant using the fork's builtin publication/subscription readers to learn writer GUID -> topic, while raw UDP capture still supplies the datagram list.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\cleanup.ps1
.\scripts\launch-rust.ps1
Get-Process hello-view-service,lil-d | Group-Object ProcessName | Select Name,Count
# expect 1/1; run launch-rust.ps1 again and re-check: still 1/1
# lil-d is in front; the earth spins about a fixed tilted pole
# run build\deploy\bin\dds-echo.exe, Start, observe HelloWorldViewTopic under unidentified
.\scripts\cleanup.ps1
```

## Bookkeeping

When the story passes, move it from `planning/backlog.md` to `planning/completed_stories.md` with the completion timestamp.
