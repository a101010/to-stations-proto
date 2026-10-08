# Current story: assets

This file is the detailed, living plan for the one active story. It is rewritten for each story. The backlog in `backlog.md` holds all work and the per-story status.

## Story

Add the earth cube-map faces and a font under `assets/`.

- **Depends on:** none.
- **Minimal test:** both load and render/validate.

## Decisions

* Earth imagery: NASA Blue Marble 2002, public domain. An 8192×4096 equirectangular source is converted to six 1024×1024 cube faces in OpenGL face order `px, nx, py, ny, pz, nz`.
* Faces are committed as **JPEG**, each **50–100 KB** (repo-size requirement on the committed asset, not a tool feature). The chosen `--jpeg-quality` and the resulting per-face sizes are recorded in `assets/earth/CREDITS.md`.
* The converter is a committed Rust tool at `rust/tool/equirect-to-cubemap` (a workspace member); the generated faces are committed, the source image is not.
* Fonts: Fira Code Regular and Fira Sans Regular (both OFL 1.1), with their license files, under `assets/fonts/`.
* Font validation uses `skrifa` (the Google Fonts `fontations` stack). `ttf-parser` is unmaintained (RUSTSEC-2026-0192), and `ab_glyph`/`fontdue` pull it transitively; `skrifa` is what egui already uses and is backend-agnostic.
* Rust programs use `clap`; the converter uses clap derive.
* The rendering backend (glow/OpenGL versus WebGPU) and SDF/MSDF text are deferred (see `architecture.md`, "Deferred decisions"); this story only adds assets and validates that they load.
* During this story the workspace is `members = ["crates/stations-dds", "tool/*"]`; the `crates/` to `lib/` move is deferred to the `stations-dds` story.

## Deliverables

1. `assets/earth/{px,nx,py,ny,pz,nz}.jpg` - six 1024×1024 faces, each 50–100 KB.
2. `assets/earth/CREDITS.md` - NASA attribution, the face-order/orientation convention, and the chosen JPEG quality and per-face sizes.
3. `assets/fonts/FiraCode-Regular.ttf`, `assets/fonts/FiraSans-Regular.ttf`, and their OFL 1.1 license files.
4. `rust/tool/equirect-to-cubemap/` - committed converter tool.
5. `rust/Cargo.toml` - add the tool to `members`; pin `image`, `clap`, and `skrifa` in `[workspace.dependencies]`.
6. `build/assets-smoke/` - temporary validator.

## Converter tool

### Crate
- Path `rust/tool/equirect-to-cubemap/`; library `equirect_to_cubemap` plus binary `equirect-to-cubemap`.
- Dependencies: `image`, `clap` (both `{ workspace = true }`).
- Images are `RgbImage` (no alpha).

```
rust/tool/equirect-to-cubemap/
  Cargo.toml
  src/
    lib.rs         // Error, run()
    cli.rs         // Args, Format (clap derive)
    cubemap.rs     // Face, direction/UV mapping, render_face, convert()
    sampling.rs    // bilinear()
    image_io.rs    // load_source(), save_face()
    main.rs        // Args::parse(), run(), exit code
```

### CLI (clap derive)
```
equirect-to-cubemap <input> <output-dir> [--size N] [--format png|jpg] [--jpeg-quality Q]
```
```
#[derive(clap::Parser)]
#[command(name = "equirect-to-cubemap", about = "Convert an equirectangular image to six cube faces")]
struct Args {
    input: PathBuf,
    output_dir: PathBuf,
    #[arg(long, default_value_t = 1024)] size: u32,
    #[arg(long, value_enum, default_value_t = Format::Jpg)] format: Format,
    #[arg(long, default_value_t = 75)] jpeg_quality: u8,
}
#[derive(clap::ValueEnum, Clone, Copy)] enum Format { Png, Jpg }
```
clap handles `--help` and bad arguments (exit 2). `main` calls `Args::parse()`, then `run(&args)`; on error it prints `error: …` and exits 1.

### Interfaces
```
// cubemap.rs
pub enum Face { PosX, NegX, PosY, NegY, PosZ, NegZ }
impl Face { pub const ALL: [Face; 6]; pub fn stem(self) -> &'static str; } // "px".."nz"
pub fn face_direction(face: Face, s: f32, t: f32) -> [f32; 3]
pub fn direction_to_equirect(d: [f32; 3]) -> (f32, f32)
pub fn render_face(source: &RgbImage, face: Face, size: u32) -> RgbImage
pub fn convert(source: &RgbImage, size: u32, on_face: &mut impl FnMut(Face)) -> Vec<(Face, RgbImage)>

// sampling.rs
pub fn bilinear(image: &RgbImage, x: f32, y: f32) -> Rgb<u8>   // x wraps, y clamps

// image_io.rs
pub fn load_source(path: &Path) -> Result<RgbImage, image::ImageError>
pub fn save_face(image: &RgbImage, path: &Path, format: Format, quality: u8) -> Result<(), image::ImageError>

// lib.rs
pub enum Error { Io(io::Error), Image(image::ImageError) }
pub fn run(args: &Args) -> Result<(), Error>
```

### Pseudocode
```
run(args):
    source = load_source(args.input)?
    if source.width != 2 * source.height: warn "source is not 2:1 (equirectangular)"
    create_dir_all(args.output_dir)?
    for (face, image) in convert(source, args.size, |f| print "rendering {f}"):
        path = output_dir / (face.stem() + ext(args.format))
        save_face(image, path, args.format, args.jpeg_quality)?
        print "wrote {path}"
    print "done: 6 faces at {size}x{size}"

render_face(source, face, size):
    dst = new RgbImage(size, size)
    for j in 0..size:
        t = 2*((j+0.5)/size) - 1
        for i in 0..size:
            s = 2*((i+0.5)/size) - 1
            (u, v) = direction_to_equirect(face_direction(face, s, t))
            dst[i,j] = bilinear(source, u*source.width, v*source.height)
    return dst

face_direction(face, s, t):
    match face:
        PosX: normalize( 1, -t, -s)
        NegX: normalize(-1, -t,  s)
        PosY: normalize( s,  1,  t)
        NegY: normalize( s, -1, -t)
        PosZ: normalize( s, -t,  1)
        NegZ: normalize(-s, -t, -1)

direction_to_equirect(d):           // d normalized; +Y is north
    lon = atan2(d.x, d.z)           // [-pi, pi]
    lat = asin(clamp(d.y, -1, 1))   // [-pi/2, pi/2]
    return ( lon/(2*pi) + 0.5 , 0.5 - lat/pi )

bilinear(image, x, y):
    x0 = floor(x - 0.5); fx = (x - 0.5) - x0
    y0 = floor(y - 0.5); fy = (y - 0.5) - y0
    s(px, py) = image[ wrap_x(px), clamp_y(py) ]
    return lerp2( s(x0,y0), s(x0+1,y0), s(x0,y0+1), s(x0+1,y0+1), fx, fy )
```

### Conventions (recorded in `assets/earth/CREDITS.md`)
* Face order `px, nx, py, ny, pz, nz`, standard OpenGL cube-map layout, `+Y` up.
* Equirectangular: `u = 0` at longitude `-π`, `u = 0.5` at longitude `0` (front `+Z`), `v = 0` at north pole; longitude wraps in x, latitude clamps in y.
* Absolute orientation is arbitrary; `earth-rendering` applies the attitude quaternion and must match this convention.

### Tests
* `direction_to_equirect(face_direction(PosZ, 0.0, 0.0)) == (0.5, 0.5)`.
* `face_direction(PosY, 0.0, 0.0)` maps to `v ≈ 0` (north).
* All `direction_to_equirect` outputs within `[0,1]²`.
* `bilinear` at pixel centers returns the exact pixel and wraps across the seam.

## Fonts

Download Fira Code Regular (tonsky/FiraCode) and Fira Sans Regular (mozilla/Fira), both OFL 1.1; keep the OFL license text alongside each.

## Minimal test

`build/assets-smoke/` (temporary, standalone): decode the six faces with `image` and assert they are 1024×1024 and equal; report each face's file size (must be 50–100 KB); load both fonts with `skrifa` and read units-per-em, glyph count, a codepoint-to-glyph mapping, and a glyph outline; print the results and OK; exit 0.

## Files

* Authored/committed: `assets/earth/`, `assets/fonts/`, `rust/tool/equirect-to-cubemap/`, `rust/Cargo.toml`.
* Temporary, gitignored: `build/assets-smoke/`, and the downloaded source image.
* Planning: `planning/current_story.md`, `planning/backlog.md`, `planning/architecture.md`.
