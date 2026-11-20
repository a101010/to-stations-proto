# Current story: cpp-display-abi

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Implement the C++ display library DLL against ABI v4 with the default format. Cover version, create/destroy, identity, render, mouse button/motion/wheel, error reporting, and glad loaded through the ABI's GL loader. Add a CTest unit test for default-format geometry; verify the DLL with Rust `lil-d`.

- **Depends on:** none.
- **Minimal test:** Rust `lil-d` loads the C++ `display_lib.dll`, passes ABI v4, and renders the C++ default format.

## Context

- `cpp-support` is complete. The C++ project has CMake, SDL3, glad, Dear ImGui, toml++, generated IDL types, DDS support, and support libraries.
- `include/display_lib.h` defines ABI v4.
- The Rust display library is the behavioral reference for the default format, handle lifetime, identity, and last-error behavior.
- The Rust `lil-d` app already loads a display library dynamically and checks ABI v4.
- The C++ display library does not need DDS, assets, fonts, icons, or the format manager in this story.
- No C++ display application or deployment is part of this story.

## Decisions

1. **Implement only the default format.** Format manager, chooser, menu, crosshair, hello-world, and overlays are later stories.
2. **Use glad from `cpp/third_party/glad`.** The application’s ABI loader supplies GL function addresses.
3. **Use a short-lived thread-local adapter for `gladLoadGL`.** The ABI loader takes `(name, userdata)`; glad expects a one-argument loader.
4. **Export the ABI functions with a `.def` file.** Keep the C-linkage definitions matching `include/display_lib.h` without MSVC linkage conflicts.
5. **Build into `build/cpp`.** Do not deploy the DLL yet.
6. **Keep default-format geometry testable without GL.** Put vertex generation in a small internal module with a committed CTest test.
7. **Verify the real ABI path with Rust `lil-d`.** Load the C++ DLL with a temporary `LOCALAPPDATA`.
8. **No production C++ app changes.** No C++ `lil-d`, services, deployment, or launch scripts.
9. **No build-script smoke code.** `build.ps1` builds the DLL and CTest test normally.

## Scope

**In scope**

- Add `cpp/libs/display-lib/`.
- Implement ABI v4 and the default format.
- Add an internal default-format vertex module and CTest test.
- Update `cpp/CMakeLists.txt`.
- Update planning bookkeeping.
- Run the Rust `lil-d` integration check with the C++ DLL.

**Out of scope**

- Format manager, chooser, menu, crosshair, input behavior, or DDS.
- Fonts, Earth, icons, Lux Color, Lux Pos, or hello-world.
- C++ display application.
- Deployment or runtime DLL copying.
- README restructuring.

## Files

```text
cpp/libs/display-lib/
  CMakeLists.txt
  src/default_format.hpp
  src/default_format.cpp
  src/display_lib.cpp
  src/display_lib.def
  tests/default_format_tests.cpp
```

Changes also include:

- `cpp/CMakeLists.txt`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Default format

Mirror the Rust default format:

- background: amber `[0.90, 0.62, 0.10, 1.0]`
- X: green `[0.05, 0.45, 0.15, 1.0]`
- two bars from corner to corner
- bar half-thickness `0.05`
- 12 vertices, six floats per vertex

`default_format.hpp/.cpp`:

- expose `std::vector<float> default_vertices()`
- expose the background and X color constants
- keep vertex generation independent of OpenGL

Unit test:

- 12 vertices / 72 floats
- coordinates stay within the expected NDC overshoot
- visible geometry spans both diagonals

## ABI implementation

`display_lib.cpp` defines an internal `Display` state behind the opaque `display_lib` handle:

- `uint32_t display_id`
- `GLuint program`
- `GLuint vao`
- `GLuint vbo`
- `GLsizei vertex_count`
- last pointer position and button bits, stored but unused by the default format

The `.def` file exports:

- `display_lib_abi_version` returns `4`
- `display_lib_create`
- `display_lib_set_identity`
- `display_lib_render`
- `display_lib_mouse_button`
- `display_lib_mouse_motion`
- `display_lib_mouse_wheel`
- `display_lib_destroy`
- `display_lib_last_error`

Creation:

- reject a null loader
- set up a thread-local two-argument loader adapter
- call `gladLoadGL`
- clear the adapter after loading
- compile and link the default-format shader
- create the VAO/VBO and upload vertices
- return `nullptr` and set `last_error` on failure

Error storage:

- thread-local `std::string`
- `display_lib_last_error` returns its `c_str()`
- valid until the next call

Render:

- null and non-positive-region checks
- flip top-left `y` with `framebuffer_height`
- scissor-clear the amber background
- set the region viewport
- disable depth test
- enable alpha blending
- draw the green X
- reset blending

Destroy:

- null-safe
- delete the program, VAO, and VBO
- delete the display state
- assumes the GL context is current, per the ABI

Mouse events:

- null-safe
- update the stored pointer/button state
- no default-format visual behavior

## CMake

`cpp/libs/display-lib/CMakeLists.txt`:

```cmake
add_library(display_lib SHARED
    src/default_format.cpp
    src/display_lib.cpp
    src/display_lib.def
)
set_target_properties(display_lib PROPERTIES OUTPUT_NAME display_lib)
target_include_directories(display_lib PRIVATE
    ${REPO_ROOT}/include
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)
target_compile_features(display_lib PRIVATE cxx_std_20)
target_link_libraries(display_lib PRIVATE glad)
```

Add:

```cmake
add_executable(display-lib-tests
    tests/default_format_tests.cpp
    src/default_format.cpp
)
target_include_directories(display-lib-tests PRIVATE src)
target_compile_features(display-lib-tests PRIVATE cxx_std_20)
add_test(NAME display-lib-tests COMMAND display-lib-tests)
```

`cpp/CMakeLists.txt`:

- `add_subdirectory(libs/display-lib)`

`scripts/build.ps1` needs no changes. It builds the DLL and runs CTest.

Expected output:

```text
build\cpp\Release\display_lib.dll
```

Do not place it in `build/deploy`.

## Integration test

Use the existing Rust `lil-d`:

1. Build with `scripts/build.ps1`.
2. Launch `build\deploy\bin\lil-d.exe` with:
   - `--display-id 0`
   - `--display-lib <absolute path to build\cpp\Release\display_lib.dll>`
   - a temporary `LOCALAPPDATA`
3. Confirm `lil-d` accepts ABI v4.
4. Capture the window and verify the amber background and green X.
5. Close `lil-d` and remove only the temporary `LOCALAPPDATA` directory.

No services are required because this C++ DLL implements only the default format.

## Verification

1. `scripts/build.ps1 -Test`
   - C++ default-format test passes.
   - Existing C++ and Rust tests still pass.
2. `scripts/build.ps1`
   - Release build produces `build/cpp/Release/display_lib.dll`.
3. Rust `lil-d` loads the C++ DLL successfully.
4. Visual check of the amber/green default format.
5. Confirm no real `%LOCALAPPDATA%\to-stations` state is touched.
6. `git diff --check`.

## Risks

- **GL loader adapter:** `gladLoadGL` expects a one-argument loader; keep the ABI userdata adapter short-lived and thread-local.
- **Symbol exports:** every ABI symbol must be exported with C linkage.
- **Context assumptions:** creation/render/destroy require a current GL context, as the ABI states.
- **Shader failures:** report compile/link logs through `display_lib_last_error`.
- **No GL in CTest:** test only vertex generation in CTest; use Rust `lil-d` for the real GL integration.
- **Output collision:** the C++ DLL stays under `build/cpp`; the Rust deployment’s `display_lib.dll` remains untouched.

## Bookkeeping

- `cpp-support` is completed at `2026-11-19 10:50:50`.
- `cpp-display-abi` is `in progress` in `backlog.md`.
- `cpp-display-abi` has no remaining story prerequisites.
- Do not move `cpp-display-abi` to `completed_stories.md` until the user asks to plan the next story.
