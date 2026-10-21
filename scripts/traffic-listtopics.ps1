# Runs the CycloneDDS listtopics example: a DDS participant that discovers and
# lists topics for about ten seconds, generating discovery traffic for dds-echo.
# It uses the shared StationsDDS configuration via CYCLONEDDS_URI.
#
# Prerequisite: CycloneDDS built with -DBUILD_EXAMPLES=ON and topic discovery
# (see README).

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

# Repository root, resolved from this script's own location (no '..' paths).
$repoRoot = Split-Path -Parent $PSScriptRoot

# The external prefix holds CycloneDDS; the examples live in its source build tree.
$extPathPrefix = if ($env:EXT_PATH_PREFIX) { $env:EXT_PATH_PREFIX } else { 'C:\Libraries' }
$cycloneBin = Join-Path $extPathPrefix 'src\cyclonedds\build\bin\Release'
$exe = Join-Path $cycloneBin 'listtopics.exe'
if (-not (Test-Path $exe)) {
    throw "listtopics.exe not found at $exe. Build CycloneDDS with -DBUILD_EXAMPLES=ON (see README)."
}

# Point the example at the shared configuration (IPv6 multicast on StationsDDS)
# and make ddsc.dll available without changing the user PATH.
$config = (Join-Path $repoRoot 'DDS\cyclonedds-config.xml') -replace '\\', '/'
$env:CYCLONEDDS_URI = "file://$config"
$env:PATH = "$cycloneBin;$env:PATH"

Write-Host "Running listtopics (about ten seconds of discovery)."
Write-Host "Start dds-echo and press Start to capture it."
& $exe
