<#
.SYNOPSIS
Installs scripts/subst-repo.ps1 to run at logon.

.DESCRIPTION
Writes a small wrapper into the current user's Startup folder that runs
scripts/subst-repo.ps1 at logon, so the repository is mapped to a short drive
(T: by default) automatically. This is not run by scripts/build.ps1.

.PARAMETER Drive
The drive letter to map, without the colon. Defaults to T.

.PARAMETER Uninstall
Removes the Startup entry instead of installing it.
#>
[CmdletBinding()]
param(
    [string]$Drive = 'T',
    [switch]$Uninstall
)

$ErrorActionPreference = 'Stop'

$startup = [Environment]::GetFolderPath('Startup')
if (-not $startup) {
    throw "Could not locate the Startup folder."
}
$wrapper = Join-Path $startup 'to-stations-subst.cmd'

if ($Uninstall) {
    if (Test-Path -LiteralPath $wrapper) {
        Remove-Item -LiteralPath $wrapper -Force
        Write-Host "Removed startup entry: $wrapper"
    } else {
        Write-Host "No startup entry to remove: $wrapper"
    }
    exit 0
}

$substScript = Join-Path $PSScriptRoot 'subst-repo.ps1'
if (-not (Test-Path -LiteralPath $substScript)) {
    throw "Cannot find $substScript."
}

$content = @"
@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "$substScript" -Drive $Drive
"@
Set-Content -LiteralPath $wrapper -Value $content -Encoding ASCII
Write-Host "Installed startup entry: $wrapper"

& $substScript -Drive $Drive
