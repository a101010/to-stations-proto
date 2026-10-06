# AGENTS.md

## Planning vs implementing

- Permission to write planning documents is not permission to implement the plan. Writing or editing `planning/` (or `docs/`) allows only those documents to be written.
- Implementing the plan - creating `DDS/`, `contracts/`, `rust/`, `cpp/`, `scripts/`, `build/`, or any source, build, or generated file - is a separate step and must not be combined with a planning step.
- Only permission to implement the plan is permission to implement the plan.

## Repo status

- Design and increment planning live in `planning/`: `architecture.md`, `current_increment.md`, `backlog.md`. Read all three before writing code.
- Locked stack (see `planning/current_increment.md`): Eclipse Cyclone DDS, SDL3+glad+Dear ImGui for C++, winit+glutin/glow+egui for Rust, Cargo + CMake with `scripts/build.ps1`.
- The plan is not implemented yet, so no toolchain, sources, build, test, or lint commands exist. Do not invent commands or run build tools until the plan has been implemented.
- IDs, topic names, and file paths are specified in `planning/backlog.md` and `planning/architecture.md`; use them rather than inventing your own.

## Files and folders

- **Authored files** (written by people or agents): `planning/`, `docs/`, `DDS/`, `contracts/`, `scripts/`, `assets/`, `rust/`, `cpp/`.
- **Build artifacts** only under `build/`, which is gitignored: `build/scratch/` for build-only files (IDL-generated sources, generated `topics.md`), `build/deploy/` for the deployable layout. Never put generated or build files in `planning/` or `docs/`.
