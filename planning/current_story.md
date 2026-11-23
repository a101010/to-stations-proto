# Current story: cpp-display-hello

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add the generic C++ format manager/menu and the real hello-world format to the display library: HarfBuzz GPU text, Earth cube map, greeting, Auto/Inspect, arcball and inertia, light/color rendering, and the four Rust-compatible menu slots (`Formats`, `Inspect`/`Auto`, `Lux Color`/`World`, `Lux Pos`/`World`). Use the existing Rust hello and format services as stand-ins. The `Formats` slot publishes its selection but remains a no-op until `cpp-display-chooser`; the Lux labels cycle through the Rust view state but their overlays render in later stories.

- **Depends on:** none.
- **Minimal test:** C++ `lil-d` with the Rust stand-in services shows the live greeting and Earth, rotates with mouse and controller commands, switches Auto/Inspect, and shows all four menu slots with the expected labels.

## Context

- `cpp-lil-d` is complete at `2026-11-20 16:12:02`.
- The C++ display library implements ABI v4, exports the current ABI entry points, and renders the default format.
- `cpp-lil-d` loads the display library dynamically and forwards all mouse buttons, motion, and wheel input.
- C++ `stations-dds` provides contract-aware typed readers and writers over the shared IDL.
- C++ `support` provides deploy-relative paths, guards, and window placement.
- HarfBuzz 14.6.0 and its GPU library are installed in `C:\Libraries\harfbuzz`.
- The assets are already committed: `assets/fonts/FiraSans-Regular.ttf` and the six Earth cube-map faces under `assets/earth`.
- The Rust configuration already has working hello and format services that can act as stand-ins through `launch-cpp.ps1`.
- The format chooser, Lux Color, Lux Pos, C++ services, and C++ `paws` remain future stories.

## Decisions

1. **Add `stb_image` now.** Earth's cube-map faces are JPEGs. Add `stb_image` 2.34 under `cpp/third_party/stb/`, record it in `docs/versions.md`, and reuse it for format icons in `cpp-display-chooser`.
2. **Link HarfBuzz into the C++ display library.** Add imported CMake targets for `harfbuzz` and `harfbuzz-gpu`; copy their runtime DLLs next to the C++ display library.
3. **Keep DDS inside the display library.** The display library owns the hello-world view/input endpoints and the format-manager view/command/input endpoints. The display app remains unaware of topics.
4. **Use `stations_dds::Node` on domain 0.** Load the contracts and DDS configuration from the deploy root derived from the display library path. `Node` sets `CYCLONEDDS_URI` from the deploy configuration before creating the participant.
5. **Show all four menu slots now, matching Rust.** `Formats` publishes the normal `Select` event for `format-chooser`; because the chooser renderer is not implemented yet, the C++ registry ignores the resulting unsupported assignment and remains on the current/hello format. Lux Color and Lux Pos publish their normal toggle actions and their labels switch to `World`, while their overlays render in later stories.
6. **Overlay rendering is out of scope.** While the Rust stand-in view state reports a Lux overlay, C++ continues rendering the ordinary Earth and greeting. Overlay-specific mouse/controller routing, capture, and visuals are added by `cpp-lux-color` and `cpp-lux-pos`.
7. **Inertia remains in the Rust stand-in services.** The C++ display emits arcball actions; the Rust hello-view-service computes the resulting attitude and inertia.
8. **No C++ services or `paws` in this story.**
9. **No real `%LOCALAPPDATA%` in smoke tests.** Use a temporary directory.

## Scope

**In scope**

- Generic C++ format interface, registry, assignments, menu, and virtual cursor.
- Four fixed menu slots matching the Rust menu.
- DDS hello-world view/input endpoints and format-manager endpoints.
- HarfBuzz GPU text rendering.
- Earth cube-map rendering with image decoding.
- Default format retained as a registry renderer.
- Hello-world greeting, lighting, color, Auto/Inspect, arcball input, and controller actions.
- C++ display-library runtime DLL deployment.
- CTest coverage for pure geometry, layout, contract, and image behavior.

**Out of scope**

- The format chooser renderer, icons, grid, and real `Formats` behavior.
- Lux Color and Lux Pos overlay rendering and overlay-specific input.
- C++ services or C++ `paws`.
- `launch-hybrid.ps1`.
- README restructuring.
- New formats beyond default and hello-world.

## Files

```text
cpp/libs/display-lib/
  CMakeLists.txt
  src/display_lib.cpp
  src/format_list.hpp/.cpp
  src/formats.hpp/.cpp
  src/formats/default_format.hpp/.cpp
  src/formats/hello_world_impl.hpp/.cpp
  src/format_manager_impl.hpp/.cpp
  src/format_menu.hpp/.cpp
  src/format_source.hpp/.cpp
  src/view.hpp/.cpp
  src/text.hpp/.cpp
  src/earth.hpp/.cpp
  src/image.hpp/.cpp
  src/shader.hpp/.cpp
  src/ui.hpp/.cpp
  tests/main.cpp
cpp/third_party/
  CMakeLists.txt
  stb/stb_image.h
  stb/stb_image.cpp
```

Other changes:

- `scripts/build.ps1`
- `docs/versions.md` (stb_image)
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

No `cpp/apps/displays/lil-d/` source change is expected. `scripts/cleanup.ps1` needs no change because the C++ display keeps the `lil-d.exe` image name and reuses the Rust stand-ins.

## Build and dependencies

### CMake

- Add a `stb_image` static target under `cpp/third_party/`.
- Add imported `harfbuzz` and `harfbuzz-gpu` targets under `cpp/third_party/` using:
  - `${EXT_PATH_PREFIX}/harfbuzz/include`
  - `${EXT_PATH_PREFIX}/harfbuzz/lib/harfbuzz.lib`
  - `${EXT_PATH_PREFIX}/harfbuzz/lib/harfbuzz-gpu.lib`
- Link `display_lib` against `support`, `stations-dds`, `hello_world_types`, `tomlplusplus`, `stb_image`, `harfbuzz-gpu`, `harfbuzz`, and `glad`.
- Extend `display-lib-tests` with the new pure modules and define `REPO_ROOT` for asset-based tests.

### Deploy

Extend `scripts/build.ps1` to copy into `build/deploy/cpp/bin`:

- `harfbuzz.dll`
- `harfbuzz-gpu.dll`
- `ddsc.dll`
- `ddscxx.dll`

Verify with `dumpbin /dependents` and a relocated deploy smoke run. Copy any other non-system DLLs the display library requires.

The shared `assets/` and `config/` directories already deploy to the deploy root; the C++ display library resolves them from its own location.

## Display library behavior

### Initialization and lifetime

- Keep ABI v4 and its exported symbol names unchanged.
- `display_lib_create` loads the format list and creates the DDS node, view/input sources, and format manager.
- DDS failure is non-fatal: create sources when possible, otherwise render without a view.
- Missing or malformed `formats.toml`, font, Earth faces, or shader compilation is fatal and is reported through `display_lib_last_error`.
- Store the DDS node before the readers/writers that depend on it; destroy endpoints before the node.
- `display_lib_set_identity` sets the display id used for DDS filtering and outgoing samples.
- `display_lib_render` polls DDS, then renders the active format, menu, and crosshair.
- `display_lib_mouse_button` routes input and publishes any resulting format selection.
- `display_lib_mouse_motion` and `display_lib_mouse_wheel` forward to the manager.
- All GL calls assume the caller's context is current.

### Paths and contracts

- Resolve `assets_root` and `config_root` from the display library's module path using `support`.
- Load:
  - `config/formats.toml`
  - `config/topics.toml`
  - `config/cyclonedds-config.xml`
  - `assets/fonts/FiraSans-Regular.ttf`
  - `assets/earth/*.jpg`
- The C++ display library must not use `REPO_ROOT` at runtime. `REPO_ROOT` remains test/build-only.

## Format list and registry

- Parse `formats.toml` into an ordered, validated list.
- Reject empty names and duplicates.
- Build `hello-world` and `default` renderers.
- Skip `format-chooser` in this story; remember that it is unavailable.
- Unknown names other than `format-chooser` are errors.
- The first available format is the fallback for an unassigned window.
- The default format remains assignable.

## Format manager and menu

Port the C++ subset of the Rust manager and menu.

### Manager

- Own format renderers, assignments, the menu, the last format region, pointer state, virtual cursor, and cursor source.
- Apply only supported assignments from `FormatManagerViewTopic`.
- Ignore unsupported `format-chooser` assignments for now.
- Route left-click and controller commands to the menu first, then to the active format.
- Return logical `Select` actions for publication.
- Render the active format, then the menu, then the crosshair.
- Do not render the menu when there are no items; with the required `Formats` slot this normally will not occur.

### Menu

Use the Rust top-band layout:

- Three-dot button at the format window's top-left.
- One horizontal row of equal fixed-width slots.
- No wrap, shrink-to-fit, or overflow fallback.
- Selected item highlight while open.
- `Formats` is always first.
- Hello-world contributes `Inspect`/`Auto`, `Lux Color`/`World`, and `Lux Pos`/`World`.
- Local left-click and controller right-pointer activation.
- Start toggles the menu; B cancels it.
- D-pad left/right changes the selection.
- `Formats` sends the normal `Select` for `format-chooser`.
- Lux actions send their normal `HelloWorldInputTopic` toggle actions.
- No overlay is drawn in response to Lux state yet.

### Crosshair

- Draw one crosshair at the last mouse or controller position.
- Mouse motion sets the crosshair source to mouse.
- Controller cursor movement sets it to controller when the active format does not consume the movement.
- Keep the crosshair inside the format region.
- The native cursor remains hidden by the display app over the format region.

## DDS endpoints

Use `stations_dds::Node` on domain 0 and the deployed contracts.

### Hello-world view

- `HelloWorldViewTopic`: best-effort reader.
- Filter samples by display id.
- Keep the latest sample for this display.
- Retain the last valid attitude so an invalid sample does not erase the globe orientation.
- Map greeting, attitude, valid, mode, arcball, light, color, hue, saturation, value, and overlay.
- Ignore samples before the display identity is set.

### Hello-world input

- `HelloWorldInputTopic`: reliable writer.
- Stamp every sample with the display id.
- Send the action, x/y/z, and pressed values.

### Format manager

- `FormatManagerViewTopic`: transient-local reader; apply per-display assignments.
- `FormatManagerCommandCtlTopic`: reliable reader; filter by display id; map commands exactly like Rust.
- `FormatManagerInputTopic`: reliable writer for `Select` events.
- Command mapping includes cursor move, navigate, pointer, arcball toggle, menu toggle/cancel, and overlay cancel.
- Local format selections are published; controller-driven selections are published from the polling path.

## Text rendering

Port the Rust HarfBuzz GPU text implementation.

- Load the font from `assets/fonts/FiraSans-Regular.ttf`.
- Shape with `hb_shape` at the face's units per em.
- Encode glyph outlines with `hb_gpu_draw_encode`.
- Upload the resulting `RGBA16I` blobs into a growable `GL_TEXTURE_BUFFER`.
- Assemble the supplied GLSL shader sources from `hb_gpu_shader_source` and `hb_gpu_draw_shader_source`.
- Match Rust's vertex layout, dilation, orthographic projection, atlas lookup, and coverage shader.
- Provide centered drawing, top-left drawing, text width, and font-size fitting.
- Use separate text renderers for the greeting and the menu.
- No font hinting.

## Earth rendering

Port the Rust Earth renderer.

- Decode six committed JPEG faces with the image wrapper.
- Upload an RGB cube map with linear mipmapping and clamped wrapping.
- Generate a unit-direction UV sphere with 32 stacks and 64 slices and CCW outward winding.
- Use the committed cube-map face order and orientation.
- Apply the view attitude quaternion.
- Light with the view light direction and tint with the view color.
- Use the same perspective projection and silhouette fraction as Rust.
- Enable depth testing and back-face culling for the globe, then restore the state expected by text and menu rendering.

## Image decoding

- Add a small wrapper over `stb_image` that decodes file bytes from memory.
- Request RGB for Earth faces.
- Keep the API suitable for RGBA format icons in the chooser story.
- Do not flip rows; the committed Earth assets document the expected orientation.

## Hello-world format

Port the non-overlay behavior from Rust.

- Black until the first view sample.
- Dark background once a view sample arrives.
- Render Earth from the latest attitude, light, and color.
- Render the greeting centered using HarfBuzz, with the same size fractions and maximum-text width as Rust.
- Do not render the greeting in Inspect mode.
- Menu labels:
  - `Inspect` when Auto.
  - `Auto` when Inspect.
  - `World` for an active Lux item, otherwise `Lux Color` or `Lux Pos`.
- Menu actions:
  - Mode toggle.
  - Lux Color toggle.
  - Lux Pos toggle.
- Input:
  - Left-press inside the globe starts an arcball drag and sends `Grab`.
  - Motion while dragging sends `Drag`.
  - Release sends `Release`.
  - Leaving the globe releases the drag even while the button is held.
  - Controller `ToggleArcball` sends its action.
  - Controller `CursorMove` sends `Rotate` while arcball is enabled.
- Ignore overlay visuals, capture, focus, and overlay-specific input until their stories.

## Tests

Expand `display-lib-tests` with pure CTest coverage:

- `formats.toml` parsing: order, duplicate/empty rejection, and skipped `format-chooser`.
- Assignment application: supported formats only, unsupported chooser ignored, unassigned window fallback.
- Menu layout: four fixed slots, top band, hit testing, scaling, and selection highlight behavior.
- Menu labels in Auto and Inspect, and with Lux overlay state.
- `Formats` action produces a `Select` for `format-chooser`.
- Lux menu action IDs map to the correct hello-world actions.
- Arcball normalization, globe hit testing, and stick-to-rotation scaling.
- Earth sphere vertex/index counts and winding, quaternion matrix, projection, and silhouette.
- Text width/font-size helpers and centered/top-left vertex placement.
- Image loading of one Earth face and one icon from the committed assets.

Live DDS and GL behavior remains manual.

## Manual verification

1. Run `scripts/build.ps1 -Test`.
2. Run `scripts/build.ps1`.
3. Launch C++ `lil-d` with a temporary `LOCALAPPDATA`.
4. Verify:
   - Window opens with title `lil-d - <id>`.
   - Console shows display id and power button.
   - Greeting changes from the Rust hello service.
   - Earth renders and is lit/tinted by view data.
   - Left-drag rotates the globe; release starts inertia.
   - Paws R3/arcball, D-pad, Start, and B commands reach the C++ display.
   - Auto/Inspect switches and hides/shows the greeting.
   - Menu shows all four slots with the expected labels.
   - `Formats` is visible and publishes its selection; the view remains hello-world until the chooser story.
   - Lux labels switch to `World`; overlays are not rendered yet.
   - Crosshair follows mouse/controller input.
   - Native cursor is hidden over the format and shown over the console.
   - Relocated deploy runs without repository paths or external PATH changes.
   - Duplicate display id is rejected.
5. Run `launch-cpp.ps1 -DisplayId 0` and verify the C++ display plus Rust stand-ins start.
6. Run `launch-rust.ps1` and verify the Rust configuration still works.
7. Clean up only the temporary directory and any processes started for the test.

## Verification commands

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\scripts\launch-cpp.ps1 -DisplayId 0
.\scripts\cleanup.ps1
```

## Risks

- **HarfBuzz C++ ABI:** shader sources, vertex stride, atlas encoding, and coverage math must match the Rust implementation.
- **Runtime DLLs:** CycloneDDS and HarfBuzz must be adjacent to the C++ display library in the deploy tree.
- **GL state:** Earth uses depth/culling; text and menu use blending/scissoring; each stage must restore what the next expects.
- **Coordinate systems:** the ABI uses physical pixels with a top-left origin; GL scissor/viewport must flip Y consistently.
- **DDS identity timing:** readers are created before the display identity is set; filtering begins only after it is set.
- **Temporary menu behavior:** `Formats` is visible but has no chooser yet; Lux labels cycle without overlay rendering. Document this as temporary and complete it in the later stories.
- **Unsupported assignments:** ignore `format-chooser` cleanly instead of failing or rendering an unknown format.
- **No real runtime state:** all GUI smoke tests use a temporary `LOCALAPPDATA`.

## Bookkeeping

- `cpp-lil-d` is completed at `2026-11-20 16:12:02`.
- `cpp-display-hello` is `in progress` in `backlog.md`.
- `cpp-display-hello` has no remaining story prerequisites.
- Do not move `cpp-display-hello` to `completed_stories.md` until the user asks to plan the next story.
