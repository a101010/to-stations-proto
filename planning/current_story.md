# Current story: harfbuzz

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Build and install HarfBuzz 14.6.0 with the experimental GPU library (`libharfbuzz-gpu`) to the external prefix, and add the Rust FFI crate `rust/lib/harfbuzz` (raw bindings plus thin RAII wrappers) that links it. Format text is rendered by the later `font-handling` story using hb-gpu; this story provides the dependency and the binding only.

- **Depends on:** none.
- **Minimal test:** a temporary smoke program under `build/` shapes "Hello, world!" with FiraSans, encodes a glyph with `hb_gpu_draw_encode`, and asserts a non-empty blob and a non-null GLSL fragment shader source.

## Context

`architecture.md` ("Text rendering") records that format text is rendered with HarfBuzz's experimental GPU library (the Slug algorithm): the display library shapes text, encodes glyph outlines into `RGBA16I` blobs, and rasterizes them in a `#version 330` fragment shader supplied by HarfBuzz. `font-handling` is the story that uses this; this story installs HarfBuzz and provides the Rust binding.

HarfBuzz is a C library built and installed to the external prefix, like CycloneDDS and cyclonedds-cxx. The upstream `harfbuzz-sys` bindings do not cover the experimental `hb-gpu` API (newer than the bindings, and the `bundled` build does not enable the GPU library), so this crate provides its own FFI binding.

## Decisions

* **Version.** Pin HarfBuzz `14.6.0`: the GPU draw API is since 14.0, the shader-stage/paint API since 14.2, and the API was declared stabilization-ready in 14.2. Recorded in `docs/versions.md`.
* **Components.** Build only `libharfbuzz` and `libharfbuzz-gpu`; disable subset, raster, vector, and utils. Utils are off because they are not needed and the `hb-gpu` demo utility would pull GLEW, GLFW, and OpenGL.
* **Shared, not static.** `BUILD_SHARED_LIBS=ON`, so the display library needs no C++-runtime static link. The runtime DLLs are deployed to `build/deploy/bin` like `ddsc.dll`, per `architecture.md` ("Build and launch").
* **Install location.** The external prefix (`EXT_PATH_PREFIX`, default `C:\Libraries`): install to `C:\Libraries\harfbuzz`, source at `C:\Libraries\src\harfbuzz`.
* **CMake first, Meson fallback.** The community-maintained CMake port builds and installs `harfbuzz-gpu` (target `harfbuzz-gpu` linking `harfbuzz`), installs `hb-gpu.h` under `include/harfbuzz`, and exports a CMake package (`harfbuzz::harfbuzz`, `harfbuzz::harfbuzz-gpu`). Because it is community-maintained, Meson is the documented fallback.
* **Rust binding.** A new path crate `rust/lib/harfbuzz` with `extern "C"` declarations and thin RAII wrappers; `build.rs` emits the link search path and libraries from `EXT_PATH_PREFIX`. No external crates.
* **Drop `skrifa`.** No crate references it now that text rendering uses hb-gpu; remove it from `rust/Cargo.toml` `[workspace.dependencies]`.

## Build and install

Documented in `README.md` next to the CycloneDDS sections. From a shell with the VS toolchain and CMake:

```
git clone --branch 14.6.0 --depth 1 https://github.com/harfbuzz/harfbuzz.git C:\Libraries\src\harfbuzz
cmake -S C:\Libraries\src\harfbuzz -B C:\Libraries\src\harfbuzz\build -G "Visual Studio 18 2026" -A x64 ^
  -DCMAKE_INSTALL_PREFIX=C:/Libraries/harfbuzz -DBUILD_SHARED_LIBS=ON ^
  -DHB_BUILD_GPU=ON -DHB_BUILD_GPU_DEMO=OFF ^
  -DHB_BUILD_SUBSET=OFF -DHB_BUILD_RASTER=OFF -DHB_BUILD_VECTOR=OFF -DHB_BUILD_UTILS=OFF
cmake --build C:\Libraries\src\harfbuzz\build --config Release --parallel
cmake --install C:\Libraries\src\harfbuzz\build --config Release
```

Expected install under `C:\Libraries\harfbuzz`:

* `bin\harfbuzz.dll`, `bin\harfbuzz-gpu.dll`.
* `lib\harfbuzz.lib`, `lib\harfbuzz-gpu.lib`.
* `include\harfbuzz\hb.h`, `include\harfbuzz\hb-gpu.h`, `include\harfbuzz\hb-features.h`, and the rest.
* `lib\cmake\harfbuzz\` (the `harfbuzzConfig` package).

Meson fallback (needs Python and Ninja):

```
meson setup C:\Libraries\src\harfbuzz\build --prefix=C:/Libraries/harfbuzz ^
  -Dgpu=enabled -Dsubset=disabled -Draster=disabled -Dvector=disabled -Dutilities=disabled
ninja -C C:\Libraries\src\harfbuzz\build
meson install -C C:\Libraries\src\harfbuzz\build
```

## `rust/lib/harfbuzz`

`Cargo.toml`: `publish = false`, edition 2021, no dependencies. `build.rs`:

* Read `EXT_PATH_PREFIX` (default `C:\Libraries`); the prefix is `<EXT_PATH_PREFIX>\harfbuzz`.
* `cargo:rustc-link-search=native=<prefix>\lib`.
* `cargo:rustc-link-lib=harfbuzz-gpu` and `cargo:rustc-link-lib=harfbuzz` (we call symbols from both).

`src/lib.rs`: raw `extern "C"` declarations for exactly what the renderer needs, plus thin RAII wrappers. Categories:

* **Common.** `hb_version_string`; `hb_blob_create` (with `HB_MEMORY_MODE_DUPLICATE` so HarfBuzz copies the bytes), `hb_blob_get_data`, `hb_blob_get_length`, `hb_blob_destroy`; `hb_face_create`, `hb_face_get_upem`, `hb_face_destroy`; `hb_font_create`, `hb_font_destroy`, `hb_ot_font_set_funcs`, `hb_font_set_scale`, `hb_font_get_scale`.
* **Shaping.** `hb_buffer_create`/`hb_buffer_destroy`, `hb_buffer_add_utf8`, `hb_buffer_guess_segment_properties`, `hb_buffer_set_direction`/`hb_buffer_set_script`/`hb_buffer_set_language`, `hb_buffer_get_length`, `hb_buffer_get_glyph_infos`, `hb_buffer_get_glyph_positions`, `hb_shape`. The `hb_glyph_info_t` and `hb_glyph_position_t` structs are declared `#[repr(C)]` to match the C layout.
* **GPU.** `hb_gpu_shader_source`, `hb_gpu_draw_shader_source`, `hb_gpu_draw_create_or_fail`, `hb_gpu_draw_destroy`, `hb_gpu_draw_glyph`, `hb_gpu_draw_encode` (writes an `hb_glyph_extents_t`), `hb_gpu_draw_recycle_blob`, `hb_gpu_draw_get_scale`, `hb_glyph_extents_t`.

Wrappers own their handles and free them on drop; `Blob`, `Face`, `Font`, `Buffer`, `GpuDraw`. The crate exposes just enough for `font-handling` and nothing GL-related.

## Tests

* The temporary smoke program under `build/harfbuzz-smoke/` (the minimal test, below). It uses the installed CMake package: a two-line `CMakeLists.txt` with `find_package(harfbuzz CONFIG REQUIRED)` and `target_link_libraries(smoke PRIVATE harfbuzz::harfbuzz-gpu)`, configured with `-DCMAKE_PREFIX_PATH=C:/Libraries/harfbuzz`.
* The crate's `#[test]`s (durable): load `assets/fonts/FiraSans-Regular.ttf` (located through `REPO_ROOT`, supplied by Cargo), shape "Hello, world!", assert the glyph count is non-zero and that a glyph encodes to a non-empty blob; assert the GLSL vertex and fragment shader sources are non-null and non-empty. These need no GL context.
* No GL test here; the shader is compiled and drawn by `font-handling`.

## Minimal test

```
# configure, build, and run the temporary smoke program
cmake -S build\harfbuzz-smoke -B build\harfbuzz-smoke\out -G "Visual Studio 18 2026" -A x64 -DCMAKE_PREFIX_PATH=C:/Libraries/harfbuzz
cmake --build build\harfbuzz-smoke\out --config Release
build\harfbuzz-smoke\out\Release\smoke.exe assets\fonts\FiraSans-Regular.ttf
```

It prints the HarfBuzz version, the shaped glyph count and advances for "Hello, world!", and the encoded blob length, then exits 0. Negative check: passing a missing font path fails with a non-zero exit.

## Risks and follow-ups

* **Experimental API.** `hb-gpu` is marked experimental; pinning `14.6.0` and recording it in `docs/versions.md` bounds the churn. If a later story upgrades it, re-check the shader-stage and encode signatures.
* **CMake port.** Community-maintained and warns as such; if the GPU target does not build, use the Meson fallback.
* **Linking and runtime.** Link both import libraries; deploy `harfbuzz.dll` and `harfbuzz-gpu.dll` to `build/deploy/bin` in `font-handling` so the loaded `display_lib.dll` resolves them (the app exe is in the same directory).
* **`EXT_PATH_PREFIX`.** `build.rs` defaults to `C:\Libraries`; `scripts/build.ps1` must export `EXT_PATH_PREFIX` before invoking Cargo so a custom prefix is honored.
* **Struct layout.** `hb_glyph_info_t`/`hb_glyph_position_t` are declared in the crate and must match the C layout; the crate's test guards the positions used for layout.
* **`skrifa` removal.** Confirm nothing else uses it before deleting the workspace entry.

## Result

Done and verified. HarfBuzz `14.6.0` is built with the CMake port and installed to `C:\Libraries\harfbuzz` (shared, GPU on, subset/raster/vector/utils off); `bin\harfbuzz.dll`, `bin\harfbuzz-gpu.dll`, `lib\harfbuzz.lib`, `lib\harfbuzz-gpu.lib`, and `include\harfbuzz\hb-gpu.h` are present. The crate `rust/lib/harfbuzz` links them from `EXT_PATH_PREFIX` and passes its two tests (shaping "Hello, world!" with FiraSans and encoding a glyph; shader sources present). The temporary smoke program (`build/harfbuzz-smoke/`) also passed: 13 glyphs shaped, a 1528-byte encoded blob, and non-empty GLSL shader sources. `scripts/build.ps1` now exports `EXT_PATH_PREFIX` and prepends the HarfBuzz `bin` to the process `PATH`; `scripts/build.ps1 -Test` and the deploy build both pass.

Findings: the CMake port's exported targets carry a broken `INTERFACE_INCLUDE_DIRECTORIES` (an absolute `/harfbuzz`), so the smoke program links the installed libraries directly rather than via `find_package`; the Rust binding uses `build.rs` and is unaffected. The smoke program also needs `<hb-ot.h>` for `hb_ot_font_set_funcs` (not pulled in by `<hb.h>`).

## Files

* Authored: `rust/lib/harfbuzz/{Cargo.toml,build.rs,src/lib.rs}`, `rust/lib/harfbuzz/tests/smoke.rs`.
* Changed: `rust/Cargo.toml` (member plus the `harfbuzz` workspace dependency; dropped `skrifa`), `scripts/build.ps1`, `README.md`, `docs/versions.md`, `planning/{architecture,backlog,current_story}.md`.
* Temporary: `build/harfbuzz-smoke/` (CMake project plus `smoke.cc`).
