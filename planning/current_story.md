# Current story: cmake-skeleton

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the `cpp/` CMake project and integrate configure/build into `scripts/build.ps1`. Use the Visual Studio 18 2026 generator, C++20, `EXT_PATH_PREFIX`, the install prefix `build/deploy/cpp`, and the repository root resolved without `..`. The project has no executable yet; `sdl3-imgui` adds the first real target. No C++ deployment.

- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1 -Test` configures `build/cpp`; the existing Rust tests still pass and no C++ smoke test is generated or committed.

## Context

- The repository currently has no C++ source: `cpp/` contains only `.gitkeep`.
- `scripts/build.ps1` currently generates Rust IDL types, builds/test the Rust workspace, validates contracts, and assembles the Rust deploy.
- `planning/architecture.md` already defines `build/cpp` as the CMake binary directory and `scripts/build.ps1` as the single build orchestrator.
- CMake 4.4.3 is installed and the `Visual Studio 18 2026` generator is available.
- MSVC and CMake are already recorded in `docs/versions.md`.
- The C++ runtime sequence starts with this story; SDL3, DDS, and the C++ display library are later stories.
- The broader README rewrite is tracked separately as `readme-cleanup`.

## Decisions

1. **The `cpp/` project commits no executable.** It establishes project settings for later C++ targets.
2. **The first real C++ target comes in `sdl3-imgui`.**
3. **Use the Visual Studio 18 2026 generator.** Build with `-A x64`; do not require a developer shell.
4. **Use C++20.** Set the standard project-wide and keep the project otherwise minimal.
5. **Keep `REPO_ROOT` explicit.** The top-level CMake file resolves the repository root from its own location and never uses `..` path strings.
6. **Establish `EXT_PATH_PREFIX` now.** Default it to `C:/Libraries` and let `scripts/build.ps1` pass the current prefix, even though this story has no external dependency.
7. **Set the install prefix without deploying.** Configure `CMAKE_INSTALL_PREFIX` as `build/deploy/cpp`, but add no install rules and copy nothing into `build/deploy` in this story.
8. **No smoke tests in `scripts/build.ps1`.** The C++ step only configures and builds the authored project.
9. **No new external components.** SDL3, glad, Dear ImGui, TOML, and image decoding are later stories.
10. **No Rust behavior changes.** The existing Rust build, tests, contract validation, and deploy output remain unchanged.

## Scope

**In scope**

- Create `cpp/CMakeLists.txt`.
- Remove `cpp/.gitkeep`.
- Extend `scripts/build.ps1` with CMake configure/build.
- Update `README.md` only to remove temporary smoke-target details; the broad README rewrite is `readme-cleanup`.
- Update the story status and this plan.

**Out of scope**

- SDL3, glad, Dear ImGui, TOML parsing, or image decoding.
- IDL generation, CycloneDDS-CXX, DDS, contracts, or the display ABI.
- C++ deployment, runtime DLLs, or C++ launch scripts.
- C++ services, displays, `paws`, or tools.
- C++ smoke tests.

## CMake project

`cpp/CMakeLists.txt`:

- `cmake_minimum_required(VERSION 3.16)`
- `project(to_stations LANGUAGES CXX)`
- Resolve the repository root with:
  `get_filename_component(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}" DIRECTORY)`
- Define `EXT_PATH_PREFIX` as a cache path with default `C:/Libraries`.
- Set C++20 and disable compiler extensions.
- Add MSVC warnings `/W4` and `/permissive-`.
- `enable_testing()` and `add_subdirectory()` are deferred to the first real target.

The top-level project has no install rule and no executable. The configured install prefix is reserved for later C++ deploy stories.

## Build script

In `scripts/build.ps1`, after the Rust build and contract validation and before deploy assembly:

```powershell
$cppSrc = Join-Path $repoRoot 'cpp'
$cppBuild = Join-Path $repoRoot 'build\cpp'
$cppConfig = 'Release'
$cppExtPath = $extPathPrefix -replace '\\', '/'
$cppInstall = (Join-Path $repoRoot 'build\deploy\cpp') -replace '\\', '/'

$cmakeArgs = @(
    '-S', $cppSrc,
    '-B', $cppBuild,
    '-G', 'Visual Studio 18 2026',
    '-A', 'x64',
    "-DEXT_PATH_PREFIX=$cppExtPath",
    "-DCMAKE_INSTALL_PREFIX=$cppInstall"
)
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed ($LASTEXITCODE)" }

& cmake --build $cppBuild --config $cppConfig --parallel
if ($LASTEXITCODE -ne 0) { throw "CMake build failed ($LASTEXITCODE)" }
```

The C++ step runs in both normal and `-Test` builds. It has no smoke target, no generated source, and no CTest invocation. `build/cpp` builds no target until `sdl3-imgui` adds one.

## README

Remove the temporary smoke-target details from the C++ section. The broad README rewrite is tracked separately as `readme-cleanup`; this story only keeps the README from documenting a smoke target.

## Tests

Automated:

- CMake configures the `cpp/` project into `build/cpp`.
- `scripts/build.ps1 -Test` completes the existing Rust tests and contract validation.
- No C++ smoke project or smoke source is created.

Manual:

- Re-run `scripts/build.ps1 -Test` to confirm repeatability.
- Confirm `build/cpp-smoke` and `build/cpp-smoke-build` do not exist.
- Confirm `build/deploy` contains no C++ artifact.

## Acceptance

- The `cpp/` CMake project configures with the intended settings.
- `scripts/build.ps1` configures/builds the C++ project without smoke tests.
- The existing Rust build and deploy output are unchanged.
- No external C++ dependency, deployment, runtime behavior, or smoke test is introduced.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
git diff --check
```

## Risks

- **Generator name:** the Visual Studio 18 2026 generator is required; if it is missing, use `cmake -E capabilities` to report the available generators.
- **No target:** `build/cpp` builds no target until `sdl3-imgui`; this is intentional.
- **No smoke coverage:** toolchain coverage comes from configuring and building the real C++ project in later stories.
- **Path handling:** pass absolute PowerShell paths and forward-slash CMake cache paths.

## Files touched

- `cpp/CMakeLists.txt` (new)
- `cpp/.gitkeep` (removed)
- `scripts/build.ps1`
- `README.md`
- `planning/backlog.md`
- `planning/current_story.md`

`docs/versions.md` needs no change; CMake and MSVC are already recorded.

## Bookkeeping

- `cmake-skeleton` is `in progress` in `backlog.md`.
- `cmake-skeleton` has no remaining story prerequisites.
- `readme-cleanup` is a separate backlog story, not part of this story.
- Do not move `cmake-skeleton` to `completed_stories.md` until the user asks to plan the next story.
