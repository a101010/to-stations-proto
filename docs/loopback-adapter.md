# StationsDDS loopback adapter

All DDS traffic in this prototype uses IPv6 multicast on a Microsoft KM-TEST Loopback
Adapter named `StationsDDS`. This document covers creating that adapter, verifying it,
and pointing CycloneDDS at the shared configuration.

Creating the adapter requires administrator rights and the Device Manager GUI; it is a
one-time manual step. `scripts/verify-loopback.ps1` verifies an existing adapter and its
IPv6 prerequisites but does not create one.

## Create the adapter

1. Press `Win+X` and open **Device Manager** (run as administrator).
2. **Action > Add legacy hardware > Next**.
3. Select **Install the hardware that I manually select from a list (Advanced)** > Next.
4. Select **Network adapters** > Next.
5. Manufacturer **Microsoft**; select **Microsoft KM-TEST Loopback Adapter** >
   Next > Next > Finish.
6. In **Network Connections** (`ncpa.cpl`), rename the new adapter to `StationsDDS`.
7. Open the adapter's **Properties** and confirm **Internet Protocol Version 6
   (TCP/IPv6)** is enabled.

No static address is required: the adapter uses its automatic IPv6 link-local address.
CycloneDDS runs in link-local mode and, because the configuration selects the interface
by name, listens for multicast only on this adapter.

## Verify

```
.\scripts\verify-loopback.ps1
```

The script checks that the adapter exists, is up, has IPv6 enabled, and has an IPv6
address, and prints the address. It exits non-zero with manual steps otherwise.

Equivalent manual checks:

```
Get-NetAdapter -Name StationsDDS
Get-NetIPAddress -InterfaceAlias StationsDDS -AddressFamily IPv6
netsh interface ipv6 show joins "StationsDDS"
```

## Point CycloneDDS at the configuration

CycloneDDS reads its configuration from the `CYCLONEDDS_URI` environment variable.
Point it at the shared, committed configuration:

```
$env:CYCLONEDDS_URI = 'file://T:/DDS/cyclonedds-config.xml'
```

Use `file://` followed by the path (two slashes), or a plain path. A `file:///` prefix
(three slashes) is not accepted and fails with `can't open configuration file`.

`CYCLONEDDS_URI` also accepts a comma-separated list of file paths and inline XML
strings, parsed in order into a single configuration. This is used by the smoke test to
add tracing without editing the committed file.

## Troubleshooting

* **`StationsDDS: does not match an available interface`** - the adapter is missing or
  not named exactly `StationsDDS`. Run `scripts/verify-loopback.ps1` and follow its
  guidance. CycloneDDS matches the interface by its Windows name (`Get-NetAdapter`
  name), which is the adapter's `FriendlyName`.
* **`can't open configuration file ...`** - the `CYCLONEDDS_URI` path is wrong, or it
  uses a `file:///` (three-slash) prefix. Use `file://T:/...` or a plain path.
* **Participants do not discover each other on one host** - confirm
  `EnableMulticastLoopback` is set (it is in the shared configuration) and that IPv6 is
  enabled on the adapter.
