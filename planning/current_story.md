# Current story: readme-cleanup

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Rewrite `README.md` to focus on setup, build, run, and tool usage. Remove temporary smoke-test details, implementation history, and duplicated architecture/version notes; keep exact external dependency install commands, build/test commands, launch and cleanup commands, and short pointers to `planning/architecture.md` and `docs/versions.md`. Add `dependencies.md`, a complete Windows x64 build dependency inventory including transitive dependencies, source URLs, and approximate sizes, and link it from the README.

- **Depends on:** none.
- **Minimal test:** follow the README from setup through build, launch, and cleanup without needing another source; `dependencies.md` covers system tools, external C/C++ libraries, and the Windows-target Cargo dependency graph; no temporary smoke-test artifacts are documented.

## Context

- `cpp-paws` is complete at `2026-11-25 11:18:58`.
- All Rust and C++ applications, services, and display functionality described in the first increment exist.
- The previous README mixed setup commands, implementation notes, and deep tool behavior.
- `docs/versions.md` is the concise pinned-version and install-location reference.
- The new dependency document must be complete for the Windows x64 build and include dependencies of dependencies.

## Decisions

1. **Keep README task-oriented.**
   - Quick path setup, toolchain, external dependency setup commands, build/test, run, DDS configuration, tools, and documentation links.
   - Remove temporary smoke-test details and implementation history.
2. **`dependencies.md` is the complete build inventory.**
   - Scope is Windows x64 build-time dependencies.
   - Include system/build applications, external C/C++ libraries, all Windows-target Cargo packages, pinned git dependencies, and runtime-only prerequisites in an appendix.
   - List source URLs and approximate sizes.
   - Distinguish direct from transitive dependencies.
3. **`docs/versions.md` remains the pinned-version source.**
   - `dependencies.md` links to it rather than replacing it.
4. **Rust dependency data is a lockfile snapshot.**
   - Generate from `cargo metadata --locked --filter-platform x86_64-pc-windows-msvc`.
   - Crate download sizes come from the local Cargo cache.
   - Git crates share one checkout size.
5. **README links to the dependency inventory.**
   - A short Dependencies section near the top points to `dependencies.md` and `docs/versions.md`.

## Files

```text
README.md
dependencies.md
planning/backlog.md
planning/completed_stories.md
planning/current_story.md
```

No code, build, contract, or runtime changes.

## Implementation

### 1. Dependency inventory

`dependencies.md` contains:

- Purpose and scope, including the Windows x64 target and approximate-size definition.
- System/build applications:
  - Windows 10/11 x64;
  - Visual Studio Build Tools and MSVC;
  - Windows SDK;
  - CMake;
  - Git;
  - Rustup/Rust/Cargo;
  - PowerShell;
  - OpenGL runtime;
  - optional Python, Meson, and `winget`.
- External C/C++ libraries:
  - Eclipse Cyclone DDS C 11.0.1;
  - cyclonedds-cxx 11.0.1;
  - HarfBuzz 14.6.0 including `libharfbuzz-gpu`;
  - SDL3 3.4.20;
  - CycloneDDS static build for Rust.
- Rust dependency summary:
  - 372 Windows-target third-party packages: 19 direct, 353 transitive;
  - approximately 55.01 MiB of crates.io downloads;
  - an 82.40 MiB shared CycloneDDS-Rust git checkout.
- A generated Rust package table with crate, version, kind, relationship, URL, approximate size, and direct parent names.
- Workspace-owned Rust components.
- The pinned CycloneDDS-Rust git dependency and revision.
- Runtime-only prerequisites.
- Completeness checks against Cargo manifests, `Cargo.lock`, `docs/versions.md`, and CMake.

A temporary generator under `build/` is used for the Rust table; only `dependencies.md` is committed.

### 2. README rewrite

README now contains:

- A top-level Dependencies section linking `dependencies.md` and `docs/versions.md`.
- Repository short-path requirements and helper scripts.
- Toolchain prerequisites.
- Exact setup commands for SDL3, CycloneDDS C/C++, the Rust static CycloneDDS build, and HarfBuzz.
- Build and test commands.
- Launch and cleanup commands for Rust and C++ configurations.
- Deploy layout.
- DDS configuration and loopback adapter verification.
- Tool overview for `dds-echo`, `dds-contracts`, `equirect-to-cubemap`, and traffic scripts.
- Documentation links to dependencies, versions, architecture, and the loopback adapter.

Temporary smoke-test details, implementation history, and duplicated deep architecture/version notes are removed.

## Tests and verification

- `.\scripts\build.ps1 -Test` should still pass; documentation changes do not affect code.
- Verify README links resolve: `dependencies.md`, `docs/versions.md`, `planning/architecture.md`, `docs/loopback-adapter.md`.
- Verify `dependencies.md` includes every external component in `docs/versions.md`.
- Verify the Rust table covers the Windows-target Cargo graph and direct dependencies.
- Verify the setup/build/launch/cleanup flow is complete from the README without another source.
- Verify no temporary smoke-test artifacts or implementation-history notes remain in README.

Expected test count remains 14/14.

## Risks

- Dependency sizes are approximate and depend on installer workload, source/build trees, and compiler/cache state.
- The Rust package table is a snapshot of `Cargo.lock`; regenerate it when dependencies change.
- C++ transitive build prerequisites must be verified from CMake/link/runtime information rather than guessed.
- Do not duplicate the full pinned-version table from `docs/versions.md`.
- Do not remove exact setup commands from README.

## Bookkeeping

- `cpp-paws` is completed at `2026-11-25 11:18:58`.
- `readme-cleanup` is `in progress` in `backlog.md`.
- `readme-cleanup` has no remaining story prerequisites.
- Do not move `readme-cleanup` to `completed_stories.md` until the user asks to plan the next story.
