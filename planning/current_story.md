# Current story: sdl3-imgui

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Install SDL3 3.4.20 static to the external prefix and copy glad 2.0.8 and Dear ImGui 1.93.0 sources into `cpp/third_party/`. Add CMake static library targets for glad and ImGui and verify the stack with a temporary SDL3 window that renders one Dear ImGui frame over an OpenGL 3.3 clear color. The temporary window is a build artifact and is not committed.

- **Depends on:** none.
- **Minimal test:** a temporary window project under `build/` opens an SDL3 window, renders one Dear ImGui frame, and closes cleanly; `scripts/build.ps1` builds the committed glad and ImGui libraries without smoke-test code.

## Context

- `cmake-skeleton` is complete. `cpp/` has the Visual Studio 18 2026 generator, C++20, `REPO_ROOT`, `EXT_PATH_PREFIX`, and the `build/deploy/cpp` install prefix.
- The C++ project has no executable yet; `cpp-lil-d` adds the first real C++ application.
- SDL3, glad, and Dear ImGui are not installed or present in the repository.
- CMake 4.4.3 and Visual Studio Build Tools 2026 are installed.
- The README already has a short C++ project section; the broad README rewrite remains `readme-cleanup`.
- The C++ display apps and GUIs use SDL3 + glad + Dear ImGui. C++ `paws` will use SDL gamepad input later.

## Decisions

1. **Pin SDL3 3.4.20.** Install a static build under `C:\Libraries\sdl3`; source stays at `C:\Libraries\src\sdl3`.
2. **Use static SDL3.** Avoid an SDL3 runtime DLL in the C++ deploy.
3. **Copy glad 2.0.8 generated sources into the repository.** Generate a C loader for `gl:core=3.3` and commit the generated source and license.
4. **Copy Dear ImGui 1.93.0 sources into the repository.** Include the core sources, SDL3 and OpenGL3 backends, and license; exclude the demo.
5. **Commit library targets only.** Add static CMake targets `glad` and `imgui`; do not commit a window executable.
6. **Use ImGui's embedded OpenGL3 loader for ImGui.** glad remains the loader for application and display-library GL calls.
7. **Verify the window with a temporary project under `build/`.** Do not commit the window source or add smoke-test code to `scripts/build.ps1`.
8. **No C++ deployment.** The C++ deploy and runtime DLLs arrive with `cpp-lil-d` and later stories.
9. **No README smoke-test details.** Add only the SDL3 setup needed to build the C++ project.
10. **No Rust behavior changes.** Rust build, tests, contracts, and deploy output remain unchanged.

## Scope

**In scope**

- Install SDL3 3.4.20 static to the external prefix.
- Add `cpp/third_party/glad/` sources and license.
- Add `cpp/third_party/imgui/` sources and license.
- Add `cpp/third_party/CMakeLists.txt` with `glad` and `imgui` static targets.
- Update `cpp/CMakeLists.txt` for OpenGL, SDL3, and `third_party`.
- Update `scripts/build.ps1` to pass the SDL3 prefix.
- Add SDL3 setup to `README.md`.
- Record SDL3, glad, and Dear ImGui in `docs/versions.md`.
- Update `planning/architecture.md` for static SDL3 and third-party sources.
- Verify with a temporary SDL3/ImGui window under `build/`.

**Out of scope**

- A committed SDL3/ImGui executable or smoke target.
- C++ display library, `lil-d`, services, `paws`, or tools.
- C++ deployment or launch scripts.
- CycloneDDS-CXX, IDL generation, TOML parsing, or image decoding.
- Broad README cleanup.

## SDL3 setup

Document and run:

```powershell
git clone --branch release-3.4.20 --depth 1 https://github.com/libsdl-org/SDL.git C:\Libraries\src\sdl3
cmake -S C:\Libraries\src\sdl3 -B C:\Libraries\src\sdl3\build `
  -G "Visual Studio 18 2026" -A x64 `
  -DCMAKE_INSTALL_PREFIX=C:/Libraries/sdl3 `
  -DSDL_SHARED=OFF -DSDL_STATIC=ON `
  -DSDL_TEST_LIBRARY=OFF -DSDL_TESTS=OFF
cmake --build C:\Libraries\src\sdl3\build --config Release --parallel
cmake --install C:\Libraries\src\sdl3\build --config Release
```

The install contains the SDL3 headers, `SDL3-static.lib`, and the CMake package under `lib/cmake/SDL3`.

## Third-party sources

### glad

Generate a glad 2.0.8 C loader for OpenGL 3.3 core into `cpp/third_party/glad/`:

- `include/glad/gl.h`
- `include/KHR/khrplatform.h`
- `src/glad.c`
- license/readme files

The committed CMake target `glad` is a static library with a public include directory.

### Dear ImGui

Copy the Dear ImGui 1.93.0 source into `cpp/third_party/imgui/`:

- `imgui.cpp`, `imgui_draw.cpp`, `imgui_tables.cpp`, `imgui_widgets.cpp`
- `imgui.h`, `imgui_internal.h`, `imconfig.h`
- `imstb_rectpack.h`, `imstb_textedit.h`, `imstb_truetype.h`
- `backends/imgui_impl_sdl3.cpp`, `backends/imgui_impl_sdl3.h`
- `backends/imgui_impl_opengl3.cpp`, `backends/imgui_impl_opengl3.h`
- `backends/imgui_impl_opengl3_loader.h`
- `LICENSE.txt`

The committed CMake target `imgui` is a static library with public include directories for the core and backend headers. It links `SDL3::SDL3-static` and `OpenGL::GL`; it does not link `glad`, because ImGui uses its embedded OpenGL3 loader.

## CMake

`cpp/CMakeLists.txt`:

- `find_package(OpenGL REQUIRED)`
- `find_package(SDL3 CONFIG REQUIRED COMPONENTS SDL3-static)`
- `add_subdirectory(third_party)`

`cpp/third_party/CMakeLists.txt`:

- define the `glad` static target
- define the `imgui` static target
- link `SDL3::SDL3-static` and `OpenGL::GL`
- keep the targets available for later C++ applications

No executable is added.

## Build script

`scripts/build.ps1`:

- Add `C:/Libraries/sdl3` to the CMake `CMAKE_PREFIX_PATH`.
- Fail with a clear message if the SDL3 CMake package is missing.
- Keep the existing C++ configure/build commands.
- Do not generate smoke files, run a window test, or add CTest smoke logic.

The normal build compiles `glad` and `imgui`. `-Test` continues to run the Rust tests.

## Documentation

`docs/versions.md`:

- SDL3 3.4.20 at `C:\Libraries\sdl3` with source at `C:\Libraries\src\sdl3`.
- glad 2.0.8 generated GL 3.3 core loader at `cpp/third_party/glad`.
- Dear ImGui 1.93.0 at `cpp/third_party/imgui`.

`README.md`:

- Add a short SDL3 setup subsection with the install commands.
- State that `scripts/build.ps1` finds SDL3 through `CMAKE_PREFIX_PATH`.
- Do not document temporary smoke tests.

`planning/architecture.md`:

- Note that SDL3 is installed static and that glad and Dear ImGui are copied into `cpp/third_party/` and built as C++ libraries.

## Temporary window verification

Create only under `build/`:

- `build/sdl3-imgui-smoke/CMakeLists.txt`
- `build/sdl3-imgui-smoke/main.cpp`

The temporary CMake project adds the repository `cpp/` project as a subdirectory and builds an executable linking `imgui` and `glad`.

The temporary app:

- opens an SDL3 window
- creates an OpenGL 3.3 core context
- initializes glad
- initializes Dear ImGui with the SDL3 and OpenGL3 backends
- renders one clear color and one ImGui frame
- closes cleanly and exits `0`

Run:

```powershell
cmake -S build\sdl3-imgui-smoke -B build\sdl3-imgui-smoke-build `
  -G "Visual Studio 18 2026" -A x64 `
  -DREPO_ROOT=<repository-root> `
  -DCMAKE_PREFIX_PATH=C:/Libraries/sdl3
cmake --build build\sdl3-imgui-smoke-build --config Release --parallel
.\build\sdl3-imgui-smoke-build\Release\sdl3-imgui-smoke.exe
```

Do not commit the temporary project and do not add it to `scripts/build.ps1`.

## Tests

Automated:

- `scripts/build.ps1 -Test` configures `build/cpp`.
- The committed `glad` and `imgui` static libraries compile and link.
- Existing Rust tests and contract validation still pass.
- No C++ executable or smoke source is generated by `scripts/build.ps1`.

Manual:

- Build and run the temporary SDL3/ImGui window under `build/`.
- Verify it opens, renders one frame, and exits successfully.
- Confirm no smoke source exists in `cpp/` or `scripts/build.ps1`.

## Acceptance

- SDL3 3.4.20 is installed statically and found by CMake.
- glad and Dear ImGui sources, licenses, and CMake targets are committed under `cpp/third_party/`.
- `scripts/build.ps1` builds the C++ libraries without smoke-test code.
- A temporary window verifies SDL3 + OpenGL 3.3 + Dear ImGui.
- No C++ executable, smoke source, or deployment is committed.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
git diff --check
```

Run the temporary window smoke project separately, as above.

## Risks

- **Static SDL3:** supported but less common than shared SDL3; avoids a runtime DLL.
- **Third-party source size:** ImGui and glad add committed source; include their licenses.
- **Loader coexistence:** ImGui uses its embedded OpenGL3 loader while application code uses glad; both can coexist.
- **No committed window target:** the first real executable remains `cpp-lil-d`; this story proves the stack with a temporary build artifact.
- **Prefix discovery:** CMake finds SDL3 only when `CMAKE_PREFIX_PATH` includes `C:/Libraries/sdl3`.

## Files touched

- `cpp/CMakeLists.txt`
- `cpp/third_party/CMakeLists.txt` (new)
- `cpp/third_party/glad/` (new)
- `cpp/third_party/imgui/` (new)
- `scripts/build.ps1`
- `README.md`
- `docs/versions.md`
- `planning/architecture.md`
- `planning/backlog.md`
- `planning/completed_stories.md`
- `planning/current_story.md`

## Bookkeeping

- `cmake-skeleton` is completed at `2026-11-18 16:19:53`.
- `sdl3-imgui` is `in progress` in `backlog.md`.
- `sdl3-imgui` has no remaining story prerequisites.
- Do not move `sdl3-imgui` to `completed_stories.md` until the user asks to plan the next story.
