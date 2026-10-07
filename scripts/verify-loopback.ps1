<#
.SYNOPSIS
Verifies the StationsDDS loopback adapter and its IPv6 prerequisites.

.DESCRIPTION
Checks that the Microsoft KM-TEST loopback adapter named StationsDDS exists, is up,
has IPv6 enabled, and has an IPv6 address. It does not create or modify the adapter;
if the adapter is missing or misconfigured it prints the manual steps and exits
non-zero.

Creating the adapter requires administrator rights and the Device Manager GUI. See
docs/loopback-adapter.md.

.PARAMETER Name
The adapter name to verify. Defaults to StationsDDS.
#>
[CmdletBinding()]
param(
    [string]$Name = 'StationsDDS'
)

$ErrorActionPreference = 'Stop'

function Write-ManualSteps {
    Write-Host ''
    Write-Host 'Create or fix the adapter manually:'
    Write-Host '  1. Press Win+X and open Device Manager (run as administrator).'
    Write-Host '  2. Action > Add legacy hardware > Next.'
    Write-Host '  3. Select "Install the hardware that I manually select from a list (Advanced)" > Next.'
    Write-Host '  4. Select "Network adapters" > Next.'
    Write-Host '  5. Manufacturer "Microsoft"; select "Microsoft KM-TEST Loopback Adapter" > Next > Next > Finish.'
    Write-Host "  6. In Network Connections (ncpa.cpl), rename the new adapter to '$Name'."
    Write-Host '  7. Confirm IPv6 is enabled on it (adapter Properties > Internet Protocol Version 6).'
    Write-Host ''
    Write-Host 'Then re-run this script. See docs/loopback-adapter.md.'
}

$adapter = Get-NetAdapter -Name $Name -ErrorAction SilentlyContinue

if (-not $adapter) {
    $loopbacks = @(Get-NetAdapter -ErrorAction SilentlyContinue |
        Where-Object { $_.InterfaceDescription -like '*Loopback*' })
    if ($loopbacks.Count -eq 1) {
        Write-Host "Found a loopback adapter named '$($loopbacks[0].Name)', but it is not named '$Name'."
        Write-Host "Rename it to '$Name' in Network Connections (ncpa.cpl) and re-run this script."
        exit 1
    }
    Write-Host "No network adapter named '$Name' was found."
    Write-ManualSteps
    exit 1
}

$ok = $true

if ($adapter.Status -ne 'Up') {
    Write-Host "Adapter '$Name' is not up (Status: $($adapter.Status))."
    $ok = $false
}

$binding = Get-NetAdapterBinding -Name $Name -ComponentID ms_tcpip6 -ErrorAction SilentlyContinue
if (-not $binding -or -not $binding.Enabled) {
    Write-Host "IPv6 is not enabled on adapter '$Name'."
    $ok = $false
}

$addresses = @(Get-NetIPAddress -InterfaceAlias $Name -AddressFamily IPv6 -ErrorAction SilentlyContinue)
if ($addresses.Count -eq 0) {
    Write-Host "Adapter '$Name' has no IPv6 address."
    $ok = $false
} else {
    foreach ($address in $addresses) {
        Write-Host "IPv6 address on '$Name': $($address.IPAddress)/$($address.PrefixLength)"
    }
}

if (-not $ok) {
    Write-Host ''
    Write-Host "Adapter '$Name' exists but does not meet the IPv6 prerequisites."
    Write-ManualSteps
    exit 1
}

Write-Host "Adapter '$Name' is present, up, and has IPv6 configured."
exit 0
