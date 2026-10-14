# Builds the project: generates Rust types from the shared IDL in DDS/ into
# build/gen/rust, then builds the Rust workspace. Pass -Test to run the tests.
#
# It never cleans the Cargo target directory, so dependencies (including the
# pinned fork of cyclonedds-rust) are not rebuilt unless build/ is removed.

[CmdletBinding()]
param(
    [switch]$Test
)

$ErrorActionPreference = 'Stop'

# Repository root, resolved from this script's own location (no '..' paths).
$repoRoot = Split-Path -Parent $PSScriptRoot

# The external prefix holds installed components (CycloneDDS, cyclonedds-cxx,
# SDL3). Not needed yet; kept for the steps added later.
$extPathPrefix = if ($env:EXT_PATH_PREFIX) { $env:EXT_PATH_PREFIX } else { 'C:\Libraries' }

# The fork's cyclonedds-idlc is Rust codegen (it wraps cyclonedds-build). The C
# idlc.exe emits C/C++, not Rust.
$forkUrl = 'https://github.com/a101010/cyclonedds-rust'
$forkRev = 'acef68438ef3c8eae56c45afbecdbb89f3d3e115'

$codegenTargetDir = Join-Path $repoRoot 'build\rust\codegen'
$installRoot      = Join-Path $repoRoot 'build'
$genDir           = Join-Path $repoRoot 'build\gen\rust'
$ddsDir           = Join-Path $repoRoot 'DDS'

New-Item -ItemType Directory -Force -Path $codegenTargetDir, $genDir | Out-Null

# Build/install the tool from the pinned rev. --target-dir caches the compile,
# so re-running is a no-op unless the rev changes; it is only rebuilt when
# build/ is removed. cargo install places the binary in <root>\bin.
& cargo install --git $forkUrl --rev $forkRev cyclonedds-idlc --locked --force `
    --target-dir $codegenTargetDir --root $installRoot
if ($LASTEXITCODE -ne 0) { throw "cargo install cyclonedds-idlc failed ($LASTEXITCODE)" }

$cycloneddsIdlc = Join-Path $installRoot 'bin\cyclonedds-idlc.exe'

# Run with --no-idlc so it never invokes the C idlc; the built-in parser produces
# the Rust. #[dds_typename] is emitted by default.
& $cycloneddsIdlc `
    --input (Join-Path $ddsDir 'hello_world.idl') `
    --output-dir $genDir `
    --include-dir $ddsDir `
    --no-idlc
if ($LASTEXITCODE -ne 0) { throw "IDL code generation failed ($LASTEXITCODE)" }

Write-Host "Generated $(Join-Path $genDir 'hello_world.rs')"

# Build the Rust workspace. Cargo reads .cargo/config.toml at the repository root
# (target-dir and REPO_ROOT), so it must be invoked from there.
Push-Location $repoRoot
try {
    if ($Test) {
        & cargo test --manifest-path 'rust\Cargo.toml'
    } else {
        & cargo build --release --manifest-path 'rust\Cargo.toml'
    }
    if ($LASTEXITCODE -ne 0) { throw "cargo build/test failed ($LASTEXITCODE)" }
}
finally {
    Pop-Location
}

# Assemble the deployable layout. Only build/deploy is durable; everything else
# under build/ is temporary. Each story that adds an executable or library
# extends this step. The release build is deployed so the display is a GUI
# application with no console window (see lil-d's windows_subsystem attribute).
if (-not $Test) {
    $deployBin = Join-Path $repoRoot 'build\deploy\bin'
    New-Item -ItemType Directory -Force -Path $deployBin | Out-Null
    Copy-Item -Path (Join-Path $repoRoot 'build\rust\release\lil-d.exe') -Destination $deployBin -Force
    Write-Host "Deployed $(Join-Path $deployBin 'lil-d.exe')"
}
