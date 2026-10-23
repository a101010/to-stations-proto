# Current story: hello-view-service-fixes

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Debug and fix the problems seen when running the Rust configuration after `hello-view-service`: `lil-d` opens behind every other window; duplicate services/displays get launched; the view samples are all `unidentified`; and the earth's north-pole point moves on screen instead of the earth spinning about its tilted polar axis.

- **Depends on:** none.
- **Minimal test:** after `cleanup.ps1` and a single `launch-rust.ps1`, the `lil-d` window opens in front; exactly one `hello-view-service` and one `lil-d` run; the earth spins about a fixed tilted pole (the north-pole point is stationary); view samples are categorized (or their non-categorization is understood).

The `dds-echo`-under-load work is split out to `robust-dds-echo`.

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
- **All `unidentified`.** The view `DATA` comes from a writer whose topic dds-echo never learned: `capture.rs:42-50` binds only the multicast group on ports `7400/+1`, and the view writer's SEDP is not on that group, so its DATA stays unattributed. This is the same SEDP-visibility gap seen during `dds-echo-sedp`.
- **Rotation (north-pole point appeared to move) — resolved, not a bug.** The pole marker (Step 0) showed the object `+Y` cap fixed while the earth spins about it: the rotation is correct. The perceived tumbling came from the tilt leaning 45 deg to the left, which put the pole point near the limb, where the geometry crossing behind it read as motion. The source is correct — the service sends `R0·spin` about the earth's `+Y` (`view.rs:61-65`), the display uploads column-major `R` with `transpose=false` and the shader computes `u_attitude * a_dir` (`earth.rs:42,144,149,248-264`), and the cube map maps north to `+Y` (`cubemap.rs:45-55`, `CREDITS.md:45`). Orientation changed to lean straight toward the viewer so the pole point is clearly visible (see Step 3).

## Plan

### Step 0 - North-pole-marker diagnostic (done, reverted)

A temporary pole/arctic marker in `earth.rs` confirmed the object `+Y` cap is fixed under the spin (see Findings), so the rotation is correct. The marker has been removed.

### Step 1 - Launch hygiene (priority 1)

`scripts/launch-rust.ps1` runs `cleanup.ps1` (or shares its app list) before starting, so repeated launches do not stack services/displays. Check: two consecutive launches leave exactly one `hello-view-service` and one `lil-d`.

### Step 2 - Window ordering (priority 1)

In `rust/ui/lil-d/src/app.rs::setup`, after `set_visible(true)`, raise/focus the window (`gl_window.window().focus_window()`); if Windows foreground-lock blocks it, add `request_user_attention(Critical)`. Check: `lil-d` is the front window after launch.

### Step 3 - Rotation orientation (priority 3)

No rotation bug to fix (see Findings). The pole's view-space direction is now set directly in `rust/service/hello-view-service/src/view.rs` by two angles: `POLE_VIEW_ANGLE_DEG = 55` (its angle from the camera axis; the 66.5 deg tangent limit for the arctic circle, smaller shows more of the far side) and `POLE_SCREEN_TILT_DEG = 45` (apparent tilt from vertical toward the left). This gives `pole_in_view() = [-0.579, 0.579, 0.574]`: the whole arctic circle is visible with ~11.5 deg beyond it, and the pole sits ~45 deg up-left. Lighting is unchanged (upper right, `LIGHT_DIR`). The initial sub-camera point is centered on North America (~35 deg N, 100 deg W); `initial_attitude` builds it with the cube map's longitude convention `lon = atan2(x, z)` (the earlier `[cos lon, sin lon]` form centered the Pacific). Keep 100 Hz. Optionally add a permanent display-side test that `quat_to_mat3(q)·(+Y) == rotate(q, +Y)` for a tilted `q` (the existing test only covers a Z rotation), pinning the convention.

### Step 4 - Attribution (after the above)

Small investigation first: decide between forcing discovery/metatraffic onto the multicast group via `DDS/cyclonedds-config.xml`, having `dds-echo` also bind the participants' unicast metatraffic ports, or accepting the limitation.

## Questions

1. Step 0 answered: the object-`+Y` cap is fixed; the rotation was correct and the earlier perception was the pole sitting near the limb. Marker to be reverted before the real fixes.
2. Step 4: force discovery multicast, bind unicast metatraffic ports in dds-echo, or accept the gap?
