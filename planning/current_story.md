# Current story: cpp-paws

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the C++ `paws` using SDL3, Dear ImGui, and SDL gamepad input: the virtual Xbox controller, real controller selection, per-display persistence, R3 checkbox, Start/B/Back/A/D-pad/right-stick behavior, instance guard, window placement, and the same DDS commands as Rust `paws`. Add it to the C++ deploy and `launch-cpp.ps1`. Finish the pure C++ two-display configuration.

- **Depends on:** none.
- **Minimal test:** `launch-cpp.ps1` with two displays and a temporary `LOCALAPPDATA` runs the pure C++ configuration; hello-world, menu, arcball, Lux Color, Lux Pos, C++ `paws`, persistence, and display isolation work; the Rust configuration still runs.

## Context

- `cpp-lux-pos` is complete at `2026-11-24 14:42:42`.
- Every other C++ application, service, and display library is implemented.
- `launch-cpp.ps1` now uses the C++ `paws`.
- The Rust `paws` remains the Slint implementation with gilrs XInput gamepad support.
- C++ GUI applications use SDL3 + Dear ImGui; C++ `paws` must use SDL gamepad APIs.
- `paws` commands, state path, and persisted controller schema already exist in Rust.
- The existing C++ `support` library provides state paths, placement resolution, and singleton guards.
- `scripts/cleanup.ps1` already stops the `paws` image name.
- Manual testing found four follow-up defects:
  - physical-controller Y axes are inverted while virtual mouse sticks are correct;
  - physical-controller input stops when the `paws` window is not focused;
  - C++ `paws` does not restore its saved window size and overwrites the size saved by Rust `paws`;
  - the C++ layout is only an approximation of the Rust layout, both default windows are larger than desired, and the Rust left stick sits too close to the D-pad.

## Decisions

1. **Use SDL3 + Dear ImGui.**
   - Functional equivalent of the Rust Slint window, not pixel-for-pixel identical.
   - Reuse the C++ SDL/OpenGL/ImGui startup and window-state patterns from `lil-d`.
2. **Use SDL gamepad APIs instead of gilrs.**
   - Enumerate Xbox/XInput-style gamepads by name.
   - Preserve Rust behavior for selection, reconnect, Virtual mode, and persistence.
3. **Keep the `paws/<display-id>.toml` schema Rust-compatible.**
   - Store optional `[controller]` and `[position]` tables.
   - Use existing `support::SavedPosition` fields.
   - Write the complete state atomically so position saves do not erase the controller.
4. **Keep command behavior identical to Rust.**
   - Port the command state machine and tests from `commands.rs`.
   - Publish the same `FormatManagerInputTopic` messages with display id and window 0.
5. **Finish the pure C++ configuration.**
   - `launch-cpp.ps1` starts C++ `paws`.
   - `launch-rust.ps1` remains unchanged.
6. **No changes to IDL, contracts, services, or display-library behavior.**
7. **Fix the SDL Y-axis convention in the SDL adapter.**
   - SDL reports stick Y axes as up-negative.
   - `Controls::read` expects up-positive input before converting to screen-down.
   - Convert SDL Y axes in `SdlPadState::axis` so physical and virtual sticks use the same screen direction.
8. **Allow gamepad input while Paws is unfocused.**
   - Set `SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS` before `SDL_Init`.
   - Keep the existing gamepad event polling.
9. **Restore saved sizes in the C++ window-state path.**
   - Use the saved inner size when selecting the current size before placement.
   - Fix C++ `paws` and C++ `lil-d`, which share the same `support::restore_or_cascade` usage.
   - Never save the fallback default over a valid saved size.
10. **Make both Paws windows smaller.**
    - Default `840x560`.
    - Minimum `640x420`.
11. **Bring the C++ layout closer to Rust and raise the Rust left stick.**
    - Use Rust’s circular/pill controls, header arrangement, and body proportions.
    - Move the Rust left stick from `0.36` to approximately `0.30` of body height.
    - Apply the same stick position to C++.

## Scope

**In scope**

- C++ `paws` application.
- SDL gamepad enumeration and control.
- Virtual controller UI with mouse interaction.
- Controller selection and persistence.
- Window placement persistence.
- Command state machine and DDS writer.
- C++ deploy and launch integration.
- Tests and architecture updates.
- Follow-up fixes for physical-controller Y direction and background input.
- Window-size restoration fixes for C++ Paws and C++ lil-d.
- Smaller default/minimum window sizes for Rust and C++ Paws.
- C++ layout parity work and Rust left-stick repositioning.

**Out of scope**

- Changes to Rust Paws behavior other than layout and window sizing.
- URL/launch-hybrid work.
- New DDS topics or contracts.
- New display-library features.
- Slint C++ integration.
- README restructuring.

## Files

```text
cpp/apps/controls/
  CMakeLists.txt
  paws/
    CMakeLists.txt
    src/main.cpp
    src/app.hpp
    src/app.cpp
    src/state.hpp
    src/state.cpp
    src/window_state.hpp
    src/window_state.cpp
    src/gamepad.hpp
    src/gamepad.cpp
    src/controls.hpp
    src/controls.cpp
    src/commands.hpp
    src/commands.cpp
    src/stick.hpp
    src/stick.cpp
    tests/main.cpp
```

Other changes:

```text
cpp/CMakeLists.txt
scripts/build.ps1
scripts/launch-cpp.ps1
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No `docs/versions.md`, `DDS/`, or `contracts/` changes are needed.

## Implementation

### 1. Paws state

Create an app-local `PawsState` module:

- State path: `support::state_path("paws", std::to_string(display_id))`.
- Schema:

```toml
[controller]
name = "Xbox Wireless Controller"
uuid = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
id = 2

[position]
monitor = "\\.\DISPLAY1"
monitor_width = 1920
monitor_height = 1080
x = 100
y = 50
width = 960
height = 640
```

- `controller` and `position` are optional.
- Load tolerates missing, incomplete, or malformed files by returning defaults.
- Save both tables together atomically with a temporary file and `MoveFileExW`.
- Do not use `support::save` for this file; it writes only position fields and would erase the controller.
- Preserve unknown optional position fields as the Rust schema does.

### 2. Window position and persistence

Adapt the existing `lil-d` SDL window-state logic:

- Restore position and inner size through `support::restore_or_cascade`.
- Use canonical Win32 monitor device names with the same geometry fallback.
- Restore before showing the window.
- Track move/resize changes and save on the shared interval or forced close/focus events.
- Save the complete `PawsState` after restoring the resolved position.
- Save on quit, close request, focus loss, and window move/resize.
- Save failures are logged and non-fatal.
- Use the same cascade slot as the display id, matching Rust `paws`.

### 3. Gamepad enumeration and selection

Create plain gamepad data structures independent of SDL where possible:

- `ChoiceKey { name, uuid[16], id }`
- `SavedController { name, uuid[16], id }`
- `Choice { joystick_id, key }`
- `Selection { Virtual, Gamepad(id) }`

Use SDL3:

- `SDL_GetGamepads`
- `SDL_GetGamepadNameForID`
- `SDL_GetGamepadGUIDForID`
- `SDL_OpenGamepad`
- `SDL_CloseGamepad`
- `SDL_GetGamepadConnected`
- `SDL_GetGamepadName`
- SDL gamepad added/removed events

Filter only names containing `xbox` or `xinput`, case-insensitively.

Port Rust matching rules:

- Prefer saved id among matching entries.
- If UUID is all zero, use name matching and saved id.
- If UUID is zero and no name matches, the saved id is the remaining identity.
- If UUID is nonzero, allow a unique UUID match when the name changed.
- If multiple candidates remain and none has the saved id, do not auto-select.

Selection behavior:

- Dropdown index 0 is `Virtual`.
- Selecting `Virtual` clears the remembered controller.
- Selecting a gamepad stores its name, UUID, and id.
- Startup auto-selects only the remembered controller if currently connected.
- Disconnect falls back to `Virtual` but retains the remembered controller.
- A remembered controller that appears later is auto-selected.
- Other newly connected controllers are never auto-selected.
- Refresh choices on add/remove events.

Cross-language caveat: SDL GUID bytes and gilrs UUIDs may differ for the same physical device. The shared schema and zero-UUID/name/id fallbacks keep behavior compatible for the common case; do not claim exact cross-language identity when driver UUIDs differ.

### 4. Controls and stick normalization

Port `controls.rs` and `stick.rs`.

`Controls`:

- left/right stick `(x, y)`;
- A, B, X, Y;
- LB, RB;
- LT, RT;
- D-pad up/down/left/right;
- Back, Start, R3.

Use SDL gamepad axes/buttons:

- left stick X/Y;
- right stick X/Y;
- face buttons: South=A, East=B, West=X, North=Y;
- shoulders: left/right shoulder buttons;
- triggers: left/right trigger axes at 0.5 threshold;
- D-pad buttons;
- Back, Start, right thumb.

Apply the Rust deadzone:

- clamp to `[-1, 1]`;
- deadzone `0.08`;
- rescale outside the deadzone;
- invert Y for top-left screen coordinates.

`clamp_stick`:

- invalid or non-positive radius -> `(0, 0)`;
- divide by radius;
- clamp length to 1 without changing direction.

### 5. Command state and sender

Port `commands.rs` exactly:

- cursor interval `33ms`;
- cursor speed `20`;
- right stick active when max abs axis `> 0.1`;
- send cursor movement only when active and due;
- send one zero `CursorMove` when the right stick transitions from active to rest;
- D-pad sends `Navigate` only on press edges;
- A sends right-button `Pointer` press and release edges;
- B sends `MenuCancel` on press edge;
- Back sends `OverlayCancel` on press edge;
- Start sends `MenuToggle` on press edge;
- any `r3_checked` change sends `ToggleArcball`.

`CommandSender`:

- resolve contracts from explicit `--config`, deploy config, or `./contracts`;
- load and validate `topics.toml` and `services/paws.toml`;
- create `FormatManagerInputTopic` writer;
- if DDS or contract setup fails, log and continue with the window functional;
- send with `display_id`, `window = 0`, empty format, and the computed command fields.

### 6. C++ application and UI

Create a `paws` SDL3 + OpenGL + Dear ImGui application.

CLI:

- `--display-id` required;
- `--domain` default `0`;
- `--config <dir>`.

Startup:

1. Parse args.
2. Acquire guard `paws-display-<id>`; duplicate exits with failure.
3. Load `PawsState`.
4. Load controller choices and select remembered controller if connected.
5. Create hidden SDL window, OpenGL 3.3 context, glad, ImGui.
6. Set window title and minimum size (`720x480`), preferred size `960x640`.
7. Restore window position/size.
8. Create DDS command sender.
9. Show and raise the window.

Loop at roughly 16 ms:

- poll SDL and ImGui events;
- poll gamepad add/remove and selected controller state;
- update `r3_checked` on physical R3 press edges;
- draw the virtual controller UI;
- track/save window position;
- update and send command transitions.

UI layout:

- header with `paws #<display-id>` and a Controller combo containing `Virtual` plus Xbox gamepads;
- central rounded controller body;
- LT/RT, LB/RB, left/right sticks, D-pad, A/B/X/Y, Back/Start, and the R3 checkbox-style toggle;
- `R3 Off` / `R3 On`;
- status line for Virtual versus Controller mode.

Virtual mode:

- mouse-drag sticks update normalized values;
- clicking/holding controller buttons sets their down state;
- clicking the R3 toggle flips `r3_checked`;
- controller mode ignores mouse control of sticks/buttons;
- use ImGui invisible buttons/hit areas and draw-list primitives; no Slint files or assets.

Controller mode:

- mirror selected SDL gamepad state;
- physical R3 press toggles the checkbox;
- physical disconnect falls back to Virtual.

Close handling:

- save complete state;
- destroy ImGui, GL, window, and SDL;
- do not leave the DDS writer or gamepad handles dangling.

### 7. CMake, deploy, and launch

- Add `apps/controls` to `cpp/CMakeLists.txt`.
- Add `paws` as a `WIN32` executable linking:
  - `support`;
  - `stations-dds`;
  - `format-list`;
  - `imgui`;
  - `glad`;
  - `OpenGL::GL`;
  - `SDL3::SDL3-static`;
  - `tomlplusplus`.
- Add `paws-tests` compiling the pure state/controls/gamepad/commands/stick modules and tests.
- Add `paws-tests` to CTest; expected total 14/14.
- Update `scripts/build.ps1` to copy `build\cpp\Release\paws.exe` into `build/deploy/cpp/bin`.
- Update `scripts/launch-cpp.ps1`:
  - `$paws` resolves to `$cppBin\paws.exe`;
  - description says the configuration is fully C++;
  - no Rust paws stand-in remains.
- No `cleanup.ps1` change; the process name stays `paws`.

### 8. Architecture

Update:

- C++ runtime is fully C++ after this story.
- `paws` section describes both the Rust Slint implementation and the C++ SDL3/ImGui implementation.
- C++ application tree lists `apps/controls/paws`.
- Runtime state remains shared and Rust-compatible.
- `launch-rust.ps1` continues to use Rust `paws`.

### 9. Follow-up fixes after manual testing

#### Physical-controller Y axes

In `cpp/apps/controls/paws/src/app.cpp`:

- In `SdlPadState::axis`, negate the raw SDL value for `PadAxis::LeftStickY` and `PadAxis::RightStickY`.
- Leave X axes, trigger axes, and all buttons unchanged.
- Keep `Controls::read`'s screen-coordinate behavior unchanged.
- Keep the virtual mouse stick path unchanged.

Add a pure regression helper/test for SDL-style Y values so physical up/down direction is covered without hardware:

- SDL up `-1` maps to controls-up;
- SDL down `+1` maps to controls-down;
- virtual stick behavior remains unchanged.

#### Background gamepad input

In `cpp/apps/controls/paws/src/main.cpp`, before `SDL_Init`:

```cpp
SDL_SetHint(SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS, "1");
```

- Keep `SDL_INIT_VIDEO | SDL_INIT_GAMEPAD`.
- Do not alter keyboard or mouse focus behavior.
- If verification shows the hint alone is insufficient on the current SDL build, add `SDL_UpdateGamepads()` once per frame before reading controller values.

### 10. Window sizing and layout parity follow-up

#### Saved window size

The C++ window-state path currently passes the current default inner size into
`support::restore_or_cascade`, so the returned size is always the default. The
restore path then saves that default, overwriting the Rust-saved size.

Fix both C++ SDL window-state implementations:

- `cpp/apps/controls/paws/src/window_state.cpp`
- `cpp/apps/displays/lil-d/src/window_state.cpp`

In each `restore` method:

1. Read the current inner size from SDL as the fallback.
2. If `saved->size()` exists and is valid, use that as the effective inner size.
3. Compute the outer window size from the effective inner size plus borders.
4. Pass those values into `support::restore_or_cascade`.
5. Apply the returned size with `SDL_SetWindowSize`.
6. Save the resolved size afterward.

Add a regression check that a saved inner size is used instead of the
current/default size, and that a later save preserves it.

#### Smaller defaults and minimums

Use:

- Rust `paws.slint`:
  - preferred `840x560`
  - minimum `640x420`
- C++ `paws`:
  - initial window `840x560`
  - minimum `640x420`
  - restoration fallback `840x560`

The current values are `960x640` and `720x480`.

#### Closer C++ layout

Update `cpp/apps/controls/paws/src/app.cpp` to match Rust more closely:

- Header:
  - `paws #<id>` on the left.
  - `Controller` label and combo on the right.
  - Match the Rust spacing and combo width as closely as practical.
- Body:
  - Use the Rust aspect ratio and centering:
    - body width constrained by available height times 1.45;
    - body height = width / 1.45.
  - Rounded body with `body_height * 0.22` radius and a 3 px border.
  - Match Rust body colors.
- Controls:
  - Use circular or pill-shaped controls instead of rectangular ImGui buttons.
  - Match Rust’s relative positions and sizes from `paws.slint`:
    - LT/RT, LB/RB;
    - left/right sticks;
    - D-pad U/D/L/R;
    - Y/X/B/A diamond;
    - Back/Start;
    - R3 rounded toggle.
  - Keep the existing hit areas and down-state behavior.
  - Use the Rust active color `#6fc2ff` for pressed controls.
- Footer:
  - Center the Virtual/Controller status text as Rust does.
- Left stick:
  - Use the new `0.30` vertical fraction in both Rust and C++.

Do not attempt pixel-perfect Slint reproduction; match proportions, shapes, and
spacing closely.

#### Rust left stick

In `rust/ui/paws/ui/paws.slint`:

- Change the left stick `y` from `body.height * 0.36` to `body.height * 0.30`.
- Keep the right stick where it is.
- Verify the left stick no longer crowds the D-pad at both default and minimum
  sizes.

## Tests

`paws-tests` covers:

- state round-trip and Rust-shaped state loading;
- stick clamping and invalid input;
- control deadzone, SDL Y-axis normalization, button mapping, trigger thresholds;
- saved window-size selection and preservation;
- remembered-controller matching, zero UUID, changed names, ambiguity;
- selection and dropdown index mapping;
- command state:
  - neutral input sends nothing;
  - cursor throttling;
  - right-stick rest sends one zero move;
  - D-pad press edges;
  - A press/release;
  - B, Back, Start press edges;
  - R3 checkbox changes.

Expected CTest count: 14/14.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
```

Manual verification with a temporary `LOCALAPPDATA`:

1. Run `launch-cpp.ps1` with display ids `0,1`.
2. Confirm all launched components are C++; no Rust `paws`, hello services, or format services remain.
3. Confirm two C++ `paws` windows open with independent display ids and independent state.
4. In Virtual mode:
   - drag both sticks;
   - click controller buttons;
   - toggle R3;
   - open the menu, change formats, rotate the globe, and use Lux Color/Lux Pos.
5. Verify R3 behavior:
   - outside overlays toggles arcball;
   - in Lux Color switches ring/triangle focus when able;
   - in Lux Pos does nothing.
6. Verify right-stick return-to-rest emits the zero cursor move and resets the Lux Color wheel latch.
7. With a physical controller selected:
   - both stick Y axes must move the Paws knobs and display controls in the same direction as Virtual mode;
   - with Paws unfocused, stick, D-pad, A, B, Back, Start, and R3 input must continue to reach the display.
8. Window size:
   - both Paws implementations default to `840x560`;
   - both can resize down to `640x420`;
   - resize Rust Paws, restart C++ Paws, and verify C++ uses the Rust-saved size;
   - resize C++ Paws, restart Rust Paws, and verify Rust uses the C++-saved size;
   - restart either implementation and verify its own size restores.
9. Layout:
   - C++ controls are circular/pill-shaped and proportioned like Rust;
   - left stick sits clearly above the D-pad cluster in both implementations at default and minimum sizes.
10. Verify the controller dropdown:
   - lists Virtual plus Xbox-style controllers;
   - selecting a real controller mirrors it;
   - disconnect falls back to Virtual;
   - reconnect of the remembered controller auto-selects it;
   - selecting Virtual clears the remembered controller.
11. Restart and verify window position/size and remembered controller restore.
12. Verify `launch-rust.ps1` still starts the Rust configuration.
13. Remove only the temporary `LOCALAPPDATA`; never touch the real runtime state.

## Risks

- SDL gamepad GUIDs and gilrs UUIDs are not guaranteed identical; cross-language controller auto-restore is best-effort.
- The state schema must be nested exactly like Rust; using `support::save` would erase the controller.
- Position saves must preserve controller state and vice versa.
- SDL gamepad handles must be closed on refresh/removal.
- ImGui hit areas must not accept mouse input in controller mode.
- R3 must be an observable checkbox state, not a momentary command.
- Cursor move and D-pad edge handling must not duplicate commands.
- `launch-cpp.ps1` must not start Rust `paws`.
- The app must still run when DDS setup fails, matching Rust `paws`.
- Y-axis normalization must not affect X axes or trigger axes.
- Background gamepad events must not produce duplicate command edges.
- If SDL's background-events hint is ineffective on the current build, use verified `SDL_UpdateGamepads` polling as a fallback.
- C++ size restoration must use the saved inner size, not the default size.
- C++ and Rust share the `paws/<id>.toml` size fields; changing units or schema would break cross-language restoration.
- C++ layout changes must keep controller mode, virtual mode, and hit areas working.
- Raising the Rust left stick must not overlap the D-pad at the minimum size.

## Bookkeeping

- `cpp-lux-pos` is completed at `2026-11-24 14:42:42`.
- `cpp-paws` is `in progress` in `backlog.md`.
- `cpp-paws` has no remaining story prerequisites.
- Do not move `cpp-paws` to `completed_stories.md` until the user asks to plan the next story.
