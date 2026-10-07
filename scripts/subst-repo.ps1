<#
.SYNOPSIS
Maps the repository root to a short drive letter.

.DESCRIPTION
The repository must live at a short path on Windows because the cyclonedds Rust
crate builds a copy of CycloneDDS with CMake inside the Cargo target directory,
and those nested paths exceed the Windows 260-character limit otherwise. This
script maps the repository root to a short drive (T: by default).

This script is not run by scripts/build.ps1. Run it manually, or install it to
run at logon with scripts/install-subst-startup.ps1.

.PARAMETER Drive
The drive letter to map, without the colon. Defaults to T.
#>
[CmdletBinding()]
param(
    [string]$Drive = 'T'
)

$ErrorActionPreference = 'Stop'

$letter = $Drive.TrimEnd(':', '\')
if ($letter -notmatch '^[A-Za-z]$') {
    throw "Drive must be a single letter, for example T (got '$Drive')."
}

$target = "${letter}:"
$repo = Split-Path -Parent $PSScriptRoot

$mapping = (& subst) | Where-Object { $_ -like "${letter}:\:*" } | Select-Object -First 1
if ($mapping) {
    $current = ($mapping -split '=>', 2)[1].Trim()
    if ($current -ieq $repo) {
        Write-Host "Already mapped: $target -> $repo"
        exit 0
    }
    throw "$target is already mapped to '$current'. Remove it with 'subst $target /d' or pass a different -Drive."
}

& subst $target $repo
if ($LASTEXITCODE -ne 0) {
    throw "subst failed with exit code $LASTEXITCODE."
}
Write-Host "Mapped $target -> $repo"
