# Current story: assets

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add the earth cube-map faces and a font under `assets/`.

- **Depends on:** none.
- **Minimal test:** both load and render/validate.

## Decisions

* Earth imagery: NASA Blue Marble 2002, public domain. An 8192×4096 equirectangular source is converted to six 1024×1024 cube faces, PNG, in OpenGL face order `px, nx, py, ny, pz, nz`. The face order and orientation are documented in `assets/earth/CREDITS.md` so `earth-rendering` samples them identically.
* The converter is a committed Rust tool at `rust/tool/equirect-to-cubemap` (a workspace member), not a throwaway; the generated faces are committed, the source image is not.
* Fonts: Fira Code Regular and Fira Sans Regular (both OFL 1.1), with their license files, under `assets/fonts/`.
* Font validation uses `skrifa` (the Google Fonts `fontations` stack). `ttf-parser` is unmaintained (RUSTSEC-2026-0192), and `ab_glyph`/`fontdue` pull it transitively; `skrifa` is what egui already uses and is backend-agnostic.
* The rendering backend (glow/OpenGL versus WebGPU) and SDF/MSDF text are deferred (see `architecture.md`, "Deferred decisions"); this story only adds assets and validates that they load.
* During this story the workspace is `members = ["crates/stations-dds", "tool/*"]`; the `crates/` to `lib/` move is deferred to the `stations-dds` story.

## Deliverables

1. `assets/earth/{px,nx,py,ny,pz,nz}.png` - six 1024×1024 faces.
2. `assets/earth/CREDITS.md` - NASA attribution and the face-order/orientation convention.
3. `assets/fonts/FiraCode-Regular.ttf`, `assets/fonts/FiraSans-Regular.ttf`, and their OFL 1.1 license files.
4. `rust/tool/equirect-to-cubemap/` - committed converter tool.
5. `rust/Cargo.toml` - add the tool to `members`; pin `image` and `skrifa` in `[workspace.dependencies]`.
6. `build/assets-smoke/` - temporary validator.

## Converter tool

`rust/tool/equirect-to-cubemap`, CLI `equirect-to-cubemap <input-equirect> <output-dir> [--size 1024] [--format png]`, writing `px.png, nx.png, py.png, ny.png, pz.png, nz.png`. For each face pixel it computes a direction vector, maps it to equirectangular longitude/latitude, bilinear-samples the source, and writes the pixel. Uses `image = { workspace = true }`.

## Fonts

Download Fira Code Regular (tonsky/FiraCode) and Fira Sans Regular (mozilla/Fira), both OFL 1.1; keep the OFL license text alongside each.

## Minimal test

`build/assets-smoke/` (temporary, standalone): decode the six faces with `image` and assert they are 1024×1024 and equal; load both fonts with `skrifa` and read units-per-em, glyph count, a codepoint-to-glyph mapping, and a glyph outline; print the results and OK; exit 0.

## Files

* Authored/committed: `assets/earth/`, `assets/fonts/`, `rust/tool/equirect-to-cubemap/`, `rust/Cargo.toml`.
* Temporary, gitignored: `build/assets-smoke/`, and the downloaded source image.
* Planning: `planning/current_story.md`, `planning/backlog.md`, `planning/architecture.md`.
