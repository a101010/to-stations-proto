# Current story: cmake-skeleton

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Create the `cpp/` CMake project with one runnable `cmake-smoke` target and integrate configure/build/CTest into `scripts/build.ps1`. Use the Visual Studio 18 2026 generator, C++20, `EXT_PATH_PREFIX`, the install prefix `build/deploy/cpp`, and the repository root resolved without `..`. No C++ deployment yet.

- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1 -Test` configures `build/cpp`, builds, and runs `cmake-smoke`; the executable also exits successfully when run directly.

## Context

- The repository currently has no C++ source: `cpp/` contains only `.gitkeep`.
- `scripts/build.ps1` currently generates Rust IDL types, builds/test the Rust workspace, validates contracts, and assembles the Rust deploy.
- `planning/architecture.md` already defines `build/cpp` as the CMake binary directory and `scripts/build.ps1` as the single build orchestrator.
- CMake 4.4.3 is installed and the `Visual Studio 18 2026` generator is available.
- MSVC and CMake are already recorded in `docs/versions.md`.
- The C++ runtime sequence starts with this story; SDL3, DDS, and the C++ display library are later stories.

## Decisions

1. **One CMake target proves the skeleton.** `cmake-smoke` is a console executable, not a deployed application.
2. **Use the Visual Studio 18 2026 generator.** Build with `-A x64`; do not require a developer shell.
3. **Use C++20.** Set the standard on the target and keep the project otherwise minimal.
4. **Use CTest for the smoke test.** The target is registered as a test so later C++ stories can add tests without changing the runner.
5. **Keep `REPO_ROOT` explicit.** The top-level CMake file resolves the repository root from its own location and never uses `..` path strings.
6. **Establish `EXT_PATH_PREFIX` now.** Default it to `C:/Libraries` and let `scripts/build.ps1` pass the current prefix, even though this story has no external dependency.
7. **Set the install prefix without deploying.** Configure `CMAKE_INSTALL_PREFIX` as `build/deploy/cpp`, but add no install rules and copy nothing into `build/deploy` in this story.
8. **Run the C++ build in all builds.** `scripts/build.ps1` configures and builds C++ for both normal and `-Test` runs; only `-Test` runs CTest.
9. **No new external components.** SDL3, glad, Dear ImGui, TOML, and image decoding are later stories.
10. **No Rust behavior changes.** The existing Rust build, tests, contract validation, and deploy output remain unchanged.

## Scope

**In scope**

- Create `cpp/CMakeLists.txt`.
- Create `cpp/tools/cmake-smoke/CMakeLists.txt`.
- Create `cpp/tools/cmake-smoke/main.cpp`.
- Remove `cpp/.gitkeep`.
- Extend `scripts/build.ps1` with CMake configure, build, and CTest.
- Update `README.md` for the C++ project and build entry point.
- Update the story status and this plan.

**Out of scope**

- SDL3, glad, Dear ImGui, TOML parsing, or image decoding.
- IDL generation, CycloneDDS-CXX, DDS, contracts, or the display ABI.
- C++ deployment, runtime DLLs, or C++ launch scripts.
- C++ services, displays, `paws`, or tools.
- C++ test framework beyond CTest and the smoke executable.

## CMake project

`cpp/CMakeLists.txt`:

- `cmake_minimum_required(VERSION 3.16)`
- `project(to_stations LANGUAGES CXX)`
- Resolve the repository root with:
  `get_filename_component(REPO_ROOT "${CMAKE_CURRENT_SOURCE_DIR}" DIRECTORY)`
- Define `EXT_PATH_PREFIX` as a cache path with default `C:/Libraries`.
- Enable C++20.
- Add MSVC warnings `/W4` and `/permissive-`.
- `enable_testing()`
- `add_subdirectory(tools/cmake-smoke)`

`cpp/tools/cmake-smoke/CMakeLists.txt`:

- `add_executable(cmake-smoke main.cpp)`
- `target_compile_features(cmake-smoke PRIVATE cxx_std_20)`
- `add_test(NAME cmake-smoke COMMAND cmake-smoke)`

`cpp/tools/cmake-smoke/main.cpp`:

- Print a short success line.
- Return `0`.

The top-level project has no install rule. The configured install prefix is reserved for later C++ deploy stories.

## Build script

In `scripts/build.ps1`, after the Rust build and contract validation and before deploy assembly:

```powershell
$cppSrc = Join-Path $repoRoot 'cpp'
$cppBuild = Join-Path $repoRoot 'build\cpp'
$cppConfig = 'Release'

& cmake -S $cppSrc -B $cppBuild `
    -G 'Visual Studio 18 2026' -A x64 `
    "-DEXT_PATH_PREFIX=$($extPathPrefix -replace '\\','/')" `
    "-DCMAKE_INSTALL_PREFIX=$((Join-Path $repoRoot 'build\deploy\cpp') -replace '\\','/')"
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed ($LASTEXITCODE)" }

& cmake --build $cppBuild --config $cppConfig --parallel
if ($LASTEXITCODE -ne 0) { throw "CMake build failed ($LASTEXITCODE)" }
```

When `-Test` is set, run CTest:

```powershell
Push-Location $cppBuild
try {
    & ctest -C $cppConfig --output-on-failure
    if ($LASTEXITCODE -ne 0) { throw "CTest failed ($LASTEXITCODE)" }
}
finally {
    Pop-Location
}
```

Use `Push-Location` rather than `ctest --test-dir` so the script remains compatible with CMake 3.16.

The C++ build runs in both modes. Normal builds do not run CTest and do not install or deploy C++ artifacts.

## README

Update the C++ build documentation to state:

- The CMake project lives at `cpp/`.
- `scripts/build.ps1` configures `build/cpp` and builds the C++ project.
- `scripts/build.ps1 -Test` also runs CTest.
- `cmake-smoke` is the current C++ build smoke target.

The existing manual CMake generator instructions remain.

## Tests

Automated:

- CMake configures the `cpp/` project into `build/cpp`.
- `cmake-smoke` compiles and links.
- CTest runs `cmake-smoke` successfully.
- The existing Rust tests and contract validation still pass.

Manual:

- Run `build\cpp\Release\cmake-smoke.exe` directly from an unrelated working directory.
- Re-run `scripts/build.ps1 -Test` to confirm repeatability.
- Confirm `build/deploy` contains no C++ smoke artifact.

## Acceptance

- One authored C++ target configures, builds, and runs.
- `scripts/build.ps1` is the single build entry point for Rust and the C++ skeleton.
- The existing Rust build and deploy output are unchanged.
- No external C++ dependency, deployment, or runtime behavior is introduced.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
.\build\cpp\Release\cmake-smoke.exe
```

Run the executable from an unrelated working directory and check that it exits with `0`. Run `git diff --check`.

## Risks

- **Generator name:** the Visual Studio 18 2026 generator is required; if it is missing, use `cmake -E capabilities` to report the available generators.
- **Build-time growth:** the skeleton is small; consider a skip flag only when later C++ stories make Rust-only iteration expensive.
- **CTest compatibility:** keep the invocation compatible with CMake 3.16.
- **Path handling:** pass absolute PowerShell paths and forward-slash CMake cache paths.
- **Deploy isolation:** do not let the smoke target leak into `build/deploy`.

## Files touched

- `cpp/CMakeLists.txt` (new)
- `cpp/tools/cmake-smoke/CMakeLists.txt` (new)
- `cpp/tools/cmake-smoke/main.cpp` (new)
- `cpp/.gitkeep` (removed)
- `scripts/build.ps1`
- `README.md`
- `planning/backlog.md`
- `planning/current_story.md`

`docs/versions.md` needs no change; CMake and MSVC are already recorded.

## Bookkeeping

- `cmake-skeleton` is `in progress` in `backlog.md`.
- `cmake-skeleton` has no remaining story prerequisites.
- Do not move `cmake-skeleton` to `completed_stories.md` until the user asks to plan the next story.
