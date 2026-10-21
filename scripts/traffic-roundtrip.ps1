# Generates a brief burst of DDS traffic by running the stations-dds round_trip
# test (a participant that publishes and receives one sample). Use -Repeat to run
# it several times.
#
# Prerequisite: the workspace builds (scripts/build.ps1).

[CmdletBinding()]
param(
    [int]$Repeat = 1
)

$ErrorActionPreference = 'Stop'

# Repository root, resolved from this script's own location (no '..' paths).
$repoRoot = Split-Path -Parent $PSScriptRoot

# The test sets CYCLONEDDS_URI itself; set it here too so any incidental logging
# uses the shared StationsDDS configuration.
$config = (Join-Path $repoRoot 'DDS\cyclonedds-config.xml') -replace '\\', '/'
$env:CYCLONEDDS_URI = "file://$config"

Write-Host "Running the stations-dds round_trip test $Repeat time(s)."
Write-Host "Start dds-echo and press Start first (each run is brief)."
Push-Location $repoRoot
try {
    for ($i = 1; $i -le $Repeat; $i++) {
        & cargo test --manifest-path 'rust\Cargo.toml' -p stations-dds --test round_trip --quiet
        if ($LASTEXITCODE -ne 0) { throw "cargo test round_trip failed ($LASTEXITCODE)" }
    }
}
finally {
    Pop-Location
}
