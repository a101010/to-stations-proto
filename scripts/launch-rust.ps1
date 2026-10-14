<#
.SYNOPSIS
Launches the Rust configuration.

.DESCRIPTION
Starts the applications of the Rust configuration from build/deploy. In this
increment that is only the small display (lil-d); later stories add the model
service, the view service, and the other displays.

Run scripts/build.ps1 first to populate build/deploy.

.PARAMETER Station
Station number passed to the display. Defaults to 0.
#>
[CmdletBinding()]
param(
    [int]$Station = 0
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $repoRoot 'build\deploy\bin\lil-d.exe'
if (-not (Test-Path $exe)) {
    throw "Not built: $exe. Run scripts\build.ps1 first."
}

Start-Process -FilePath $exe -ArgumentList '--station', $Station
Write-Host "Started lil-d (station $Station)"
