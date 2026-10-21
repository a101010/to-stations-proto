# Generates continuous DDS traffic with the CycloneDDS throughput example so
# dds-echo can capture it. The example has two processes: the publisher waits
# for a matching reader, so this script starts the subscriber in the background
# and the publisher in the foreground. Both use the shared StationsDDS
# configuration via CYCLONEDDS_URI (IPv6 multicast on that adapter).
#
# Prerequisite: CycloneDDS built with -DBUILD_EXAMPLES=ON (see README), which
# places the example binaries in the source build tree under the external prefix.

[CmdletBinding()]
param(
    [int]$Seconds = 30,
    [int]$Payload = 1024,
    [int]$IntervalMs = 100,
    [int]$Burst = 10
)

$ErrorActionPreference = 'Stop'

# Repository root, resolved from this script's own location (no '..' paths).
$repoRoot = Split-Path -Parent $PSScriptRoot

# The external prefix holds CycloneDDS; the examples live in its source build tree.
$extPathPrefix = if ($env:EXT_PATH_PREFIX) { $env:EXT_PATH_PREFIX } else { 'C:\Libraries' }
$cycloneBin = Join-Path $extPathPrefix 'src\cyclonedds\build\bin\Release'
$subExe = Join-Path $cycloneBin 'ThroughputSubscriber.exe'
$pubExe = Join-Path $cycloneBin 'ThroughputPublisher.exe'
if (-not (Test-Path $pubExe) -or -not (Test-Path $subExe)) {
    throw "The throughput example is missing under $cycloneBin. Build CycloneDDS with -DBUILD_EXAMPLES=ON (see README)."
}

# Point the examples at the shared configuration (IPv6 multicast on StationsDDS)
# and make ddsc.dll available without changing the user PATH.
$config = (Join-Path $repoRoot 'DDS\cyclonedds-config.xml') -replace '\\', '/'
$env:CYCLONEDDS_URI = "file://$config"
$env:PATH = "$cycloneBin;$env:PATH"

Write-Host "Publishing DDS traffic for $Seconds s (payload=$Payload, interval=${IntervalMs}ms, burst=$Burst)."
Write-Host "Start dds-echo and press Start to capture it."

# The subscriber must be running for the publisher to match a reader and write.
$subscriber = Start-Process -FilePath $subExe -ArgumentList '0', '0' -PassThru -WindowStyle Hidden
try {
    & $pubExe $Payload $IntervalMs $Burst $Seconds
}
finally {
    if ($subscriber -and -not $subscriber.HasExited) {
        Stop-Process -Id $subscriber.Id -Force -ErrorAction SilentlyContinue
    }
}
