# Current story: cleanup-paths

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Make the Rust and C++ deploy trees symmetric: move Rust deploy artifacts from `build/deploy/bin` to `build/deploy/rust/bin`, keep C++ under `build/deploy/cpp/bin`, and keep shared `assets/` and `config/` at the deploy root. Add a shared Rust `deploy-paths` crate for the deploy-root rule and migrate the Rust display libraries, services, `paws`, and `dds-echo` to it. Update the build and launch scripts, path tests, and path documentation.

- **Depends on:** none.
- **Minimal test:** `scripts/build.ps1` produces no `build/deploy/bin`; `launch-rust.ps1` and `launch-cpp.ps1` both run from `rust/bin` and `cpp/bin`; Rust and C++ displays, services, `paws`, and `dds-echo` resolve shared assets/config from the deploy root; relocated deploy works.

## Context

- `cpp-display-hello` is complete at `2026-11-23 10:16:54`.
- Rust currently deploys to `build/deploy/bin`; C++ deploys to `build/deploy/cpp/bin`.
- Both configurations share `build/deploy/assets` and `build/deploy/config`.
- The C++ `support` library already derives the deploy root by walking three parent levels from the main executable, which works for both `rust/bin` and `cpp/bin`.
- Rust has duplicated two-level parent logic in `display-lib`, the five services, `paws`, and `dds-echo`; it assumes the executable lives directly under `build/deploy/bin`.
- `lil-d` resolves its display library relative to its own executable directory, so its DLL lookup does not need a deploy-root change.
- `cleanup.ps1` stops processes by image name, so it needs no functional path change.
- The broad README rewrite belongs to `readme-cleanup`; this story only corrects deploy-path references.

## Decisions

1. **Use `rust/bin` and `cpp/bin`.** Both languages live under the same deploy root and share `assets/` and `config/`.
2. **Add a Rust `deploy-paths` crate.** It owns the deploy-root rule and exposes config/assets helpers.
3. **Keep C++ `support` functionally unchanged.** Its three-level `deploy_root_for` rule already fits both layouts; add a Rust-layout test.
4. **No new `lib/` deploy directory.** Binaries and their immediate runtime DLLs stay in each language's `bin`.
5. **No hybrid-launch work.** This story only corrects the layout and path resolution.
6. **No README rewrite.** Update only the deploy-path references needed for correctness.
7. **No real `%LOCALAPPDATA%` in smoke tests.** Use a temporary directory.

## Target deploy layout

```text
build/deploy/
  rust/bin/
    dds-echo.exe
    display_lib.dll
    format-ctl-service.exe
    format-view-service.exe
    harfbuzz.dll
    harfbuzz-gpu.dll
    hello-ctl-service.exe
    hello-service.exe
    hello-view-service.exe
    lil-d.exe
    paws.exe
  cpp/bin/
    ddsc.dll
    ddscxx.dll
    display_lib.dll
    harfbuzz.dll
    harfbuzz-gpu.dll
    lil-d.exe
  assets/
  config/
  dds-echo-discovery.log
```

## Scope

**In scope**

- Add `rust/lib/deploy-paths`.
- Migrate all Rust deploy-root/config/assets resolution to `deploy-paths`.
- Move Rust deploy artifacts to `build/deploy/rust/bin`.
- Update `scripts/build.ps1`, `scripts/launch-rust.ps1`, and `scripts/launch-cpp.ps1`.
- Update targeted deploy-path references in `README.md`.
- Update architecture and path tests.
- Remove stale `build/deploy/bin` during the build.

**Out of scope**

- Hybrid launch configuration.
- Moving Rust or C++ build output directories.
- C++ services or C++ `paws`.
- Broad README restructuring.
- A shared `deploy/lib` directory.
- Changing C++ CMake install-prefix semantics.

## Files

```text
rust/lib/deploy-paths/
  Cargo.toml
  src/lib.rs
```

Other Rust changes:

```text
rust/Cargo.toml
rust/lib/display-lib/Cargo.toml
rust/lib/display-lib/src/lib.rs
rust/service/hello-service/Cargo.toml
rust/service/hello-service/src/main.rs
rust/service/hello-view-service/Cargo.toml
rust/service/hello-view-service/src/main.rs
rust/service/hello-ctl-service/Cargo.toml
rust/service/hello-ctl-service/src/main.rs
rust/service/format-ctl-service/Cargo.toml
rust/service/format-ctl-service/src/main.rs
rust/service/format-view-service/Cargo.toml
rust/service/format-view-service/src/main.rs
rust/tool/dds-echo/Cargo.toml
rust/tool/dds-echo/src/main.rs
rust/ui/paws/Cargo.toml
rust/ui/paws/src/commands.rs
```

Scripts and docs:

```text
scripts/build.ps1
scripts/launch-rust.ps1
scripts/launch-cpp.ps1
cpp/libs/support/tests/main.cpp
README.md
planning/architecture.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No `docs/versions.md` change is needed.

## Implementation

### 1. Add `deploy-paths`

Create `rust/lib/deploy-paths` with:

```rust
pub fn deploy_root_for(executable: &Path) -> PathBuf;
pub fn deploy_root() -> Option<PathBuf>;
pub fn config_dir() -> Option<PathBuf>;
pub fn assets_dir() -> Option<PathBuf>;
```

Rules:

- Deploy root is three parent levels above the executable:
  - `deploy/rust/bin/tool.exe` -> `deploy`
  - `deploy/cpp/bin/tool.exe` -> `deploy`
- `deploy_root()` uses `std::env::current_exe()`.
- `config_dir()` returns `<deploy_root>/config`.
- `assets_dir()` returns `<deploy_root>/assets`.
- Short or incomplete paths fall back safely without panicking.
- Add unit tests for both language layouts, config paths, and assets paths.

Workspace changes:

- Add `lib/deploy-paths` to `rust/Cargo.toml`.
- Add `deploy-paths = { path = "lib/deploy-paths" }` to workspace dependencies.
- Add the dependency to `display-lib`, the five services, `paws`, and `dds-echo`.

### 2. Migrate Rust consumers

#### Display library

- Replace the local `assets_root` and `config_root` parent arithmetic with `deploy_paths::assets_dir()` and `deploy_paths::config_dir()`.
- Preserve the development fallback to `<cwd>/contracts` when the deployed config is unavailable.
- Update comments from `build/deploy/bin` to `build/deploy/rust/bin`.

#### Services

Update each service's `contracts_dir` to:

1. Honor an explicit `--config`.
2. Try `deploy_paths::config_dir()` and use it when `topics.toml` exists.
3. Fall back to `<cwd>/contracts`.

Update CLI help/comments from `<exe_dir>/../config` to the deploy-root meaning.

#### `paws`

- Replace its local two-level lookup in `commands.rs` with `deploy_paths::config_dir()`.
- Keep explicit `--config` and development fallback behavior.

#### `dds-echo`

- Replace the local `deploy_root()` with `deploy_paths::deploy_root()`.
- Use `config_dir()` for deployed config/registry discovery.
- Keep the discovery log at the deploy root.
- Preserve explicit options and current-directory fallbacks.

#### `lil-d`

No deploy-root change. Its display library remains next to the executable in `rust/bin`.

### 3. Build and deploy

Update `scripts/build.ps1`:

- Rename `$deployBin` to `$deployRustBin`.
- Set:

```powershell
$deployRustBin = Join-Path $deployRoot 'rust\bin'
$deployCppBin = Join-Path $deployRoot 'cpp\bin'
```

- Create both directories.
- Remove the legacy `build/deploy/bin` directory if it exists.
- Copy all Rust artifacts to `$deployRustBin`.
- Copy HarfBuzz DLLs to `$deployRustBin`.
- Keep C++ artifacts and runtime DLLs in `$deployCppBin`.
- Keep `assets/` and `config/` at `$deployRoot`.
- Update the completion message.

No CMake install-prefix change is required.

### 4. Launch scripts

#### `launch-rust.ps1`

- Change `$bin` to `build/deploy/rust/bin`.

#### `launch-cpp.ps1`

- Rename the Rust service path to `$rustBin` = `build/deploy/rust/bin`.
- Keep `$cppBin` = `build/deploy/cpp/bin`.
- Start Rust stand-ins from `$rustBin` and C++ `lil-d` from `$cppBin`.
- Update the script description.

#### `cleanup.ps1`

- No functional change; image-name stopping still covers both configurations.
- Optional comment update only.

### 5. C++ support test

No functional C++ code change.

Extend `cpp/libs/support/tests/main.cpp` to verify both:

```text
deploy/rust/bin/tool.exe -> deploy
deploy/cpp/bin/tool.exe  -> deploy
```

### 6. Documentation

- Update `planning/architecture.md` with the `rust/bin` + `cpp/bin` layout, shared assets/config, runtime DLL placement, and the `deploy-paths` crate.
- Update only the README deploy-path references:
  - `build/deploy/bin` -> `build/deploy/rust/bin`
  - explain the symmetric `rust/bin` and `cpp/bin` layout
  - reconcile runtime-DLL placement for each language
- Leave the broader README rewrite to `readme-cleanup`.

## Tests

Add or update:

- `deploy-paths` unit tests for deploy roots, config paths, and asset paths.
- C++ `support-tests` for both `rust/bin` and `cpp/bin`.
- Existing Rust and CTest suites remain green.

Verification must also confirm there is no leftover `build/deploy/bin`.

## Verification

```powershell
.\scripts\build.ps1 -Test
.\scripts\build.ps1
Test-Path .\build\deploy\bin                 # false
Test-Path .\build\deploy\rust\bin\lil-d.exe  # true
Test-Path .\build\deploy\cpp\bin\lil-d.exe   # true
.\scripts\launch-rust.ps1 -DisplayId 0
.\scripts\cleanup.ps1
.\scripts\launch-cpp.ps1 -DisplayId 0
.\scripts\cleanup.ps1
```

Manual checks with a temporary `LOCALAPPDATA`:

- Rust `lil-d` renders greeting/Earth and loads assets from the shared deploy root.
- C++ `lil-d` renders greeting/Earth and loads assets/config from the shared deploy root.
- Rust services, `paws`, and `dds-echo` find `deploy/config`.
- `dds-echo` starts from `rust/bin` and keeps its discovery log at the deploy root.
- Relocated deploy trees work for both Rust and C++ without repository paths or external `PATH`.
- The old `build/deploy/bin` is not recreated.
- The Rust configuration and C++ configuration both remain functional.
- Remove only temporary state; never touch the real `%LOCALAPPDATA%`.

## Risks

- Missing a Rust consumer leaves it looking in `deploy/rust/config` or `deploy/rust/assets`.
- `std::env::current_exe()` returns the main executable, not the display DLL; this is exactly what the three-level rule needs.
- `launch-cpp.ps1` must point Rust stand-ins at `rust/bin`, not `cpp/bin`.
- Stale `build/deploy/bin` artifacts could mask layout mistakes; the build script removes the legacy directory.
- Relocated deploy must remain independent of the repository root and current working directory.
- GUI smoke tests must use temporary `LOCALAPPDATA`.

## Bookkeeping

- `cpp-display-hello` is completed at `2026-11-23 10:16:54`.
- `cleanup-paths` is `in progress` in `backlog.md`.
- `cleanup-paths` has no remaining story prerequisites.
- Do not move `cleanup-paths` to `completed_stories.md` until the user asks to plan the next story.
