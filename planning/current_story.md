# Current story: B4 - DDS configuration and StationsDDS adapter

This file is the detailed, living plan for the one active story. It is rewritten for each story. Increment scope is in `current_increment.md`; per-story status is tracked in `backlog.md`.

## Story

Author `DDS/cyclonedds-config.xml` (IPv6 multicast on `StationsDDS`) and `scripts/verify-loopback.ps1` to verify the loopback adapter and its IPv6 prerequisites. Creating the adapter requires administrator rights and is a documented manual step.

- **Depends on:** B1, B3 (done).
- **Minimal test:** a publisher and subscriber exchange samples bound to the adapter configuration; confirm IPv6 multicast.

## Decisions

* CycloneDDS reads and parses the XML itself; XML is its native configuration format, and to-stations does not parse it. `DDS/cyclonedds-config.xml` is applied by pointing the `CYCLONEDDS_URI` environment variable at it, which `dds_create_participant` consults. The code paths `dds_create_domain` (inline XML string) and `dds_create_domain_with_rawconfig` (unstable C struct) exist, but the safe `cyclonedds` crate exposes neither, so the file is kept and no TOML translation layer is added.
* Adapter creation is manual (Device Manager, "Add Legacy Hardware"). `scripts/verify-loopback.ps1` is verify-only and guidance-first: no creation and no `pnputil` staging.
* IPv6 addressing is link-local (automatic). CycloneDDS runs in link-local mode and listens for multicast only on the explicitly selected interface.
* The script is `scripts/verify-loopback.ps1` (lowercase-hyphenated, matching `subst-repo.ps1` and `install-subst-startup.ps1`). The adapter name remains `StationsDDS`.

## Deliverables

1. `DDS/cyclonedds-config.xml` - IPv6 multicast on `StationsDDS` (authored, committed).
2. `scripts/verify-loopback.ps1` - verify the adapter and its IPv6 prerequisites; guide manual creation (authored, committed).
3. `docs/loopback-adapter.md` - manual fallback and verification (authored, committed).
4. `README.md` - reference the script, config usage, and smoke-test commands (authored, committed).
5. `build/rust-dds-smoke/run-config-smoke.ps1` - temporary smoke harness (never committed).

### `DDS/cyclonedds-config.xml`

```xml
<?xml version="1.0" encoding="UTF-8" ?>
<CycloneDDS xmlns="https://cdds.io/config"
            xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
            xsi:schemaLocation="https://cdds.io/config https://raw.githubusercontent.com/eclipse-cyclonedds/cyclonedds/master/etc/cyclonedds.xsd">
  <Domain id="any">
    <General>
      <Interfaces>
        <NetworkInterface name="StationsDDS" multicast="true" />
      </Interfaces>
      <Transport>udp6</Transport>
      <AllowMulticast>true</AllowMulticast>
      <EnableMulticastLoopback>true</EnableMulticastLoopback>
    </General>
  </Domain>
</CycloneDDS>
```

Rationale:

* `Transport=udp6` selects IPv6 (`General/UseIPv6` is deprecated).
* CycloneDDS on Windows uses `GetAdaptersAddresses.FriendlyName` as the interface name, so `name="StationsDDS"` matches the renamed adapter. Fallback if it does not resolve: `address="<fe80::...>"`.
* `EnableMulticastLoopback=true` is required because all MVP processes share one host.
* `multicast="true"` is the documented workaround for virtual adapters whose OS flags omit multicast.
* `id="any"` applies the configuration to domain 0 and to later domains.
* The committed configuration has no tracing; the smoke test layers a tracing fragment.

### `scripts/verify-loopback.ps1`

* Verify-only and guidance-first; no creation and no `pnputil`.
* Detect a `Get-NetAdapter` entry named `StationsDDS` (and/or `InterfaceDescription -like '*Loopback*'`).
* Verify IPv6 prerequisites: `Status -eq 'Up'`, IPv6 enabled, and an IPv6 address present; report the link-local address.
* If missing, unnamed, down, or IPv6-absent: print the exact manual Device Manager steps and exit non-zero.
* Parameter `-Name` (default `StationsDDS`); comment-based help, `$ErrorActionPreference='Stop'`, and `$PSScriptRoot` style, matching `subst-repo.ps1`.

### `docs/loopback-adapter.md`

* Manual creation: Device Manager -> Add Legacy Hardware -> Network adapters -> Microsoft -> Microsoft KM-TEST Loopback Adapter, then rename to `StationsDDS`.
* Verification: `Get-NetAdapter`, `Get-NetIPAddress -InterfaceAlias StationsDDS -AddressFamily IPv6`, `netsh interface ipv6 show joins "StationsDDS"`.
* Point CycloneDDS at the configuration: `$env:CYCLONEDDS_URI = 'file://T:/DDS/cyclonedds-config.xml'` (use `file://` with two slashes, or a plain path; `file:///` with three slashes fails).
* Troubleshooting: adapter missing, interface-name mismatch, multicast not looped.

### `README.md`

* Add a "DDS configuration" subsection: run `verify-loopback.ps1`, set `CYCLONEDDS_URI`, and the B4 smoke-test commands.

## Minimal test (temporary, under `build/`)

Reuse the `build/rust-dds-smoke` publisher and subscriber; no crate changes are needed because `dds_create_participant` reads `CYCLONEDDS_URI`.

1. Run `scripts/verify-loopback.ps1`; it must pass (adapter exists, up, IPv6).
2. `$env:CYCLONEDDS_URI = 'file://T:/DDS/cyclonedds-config.xml'`; run `sub.exe`, then `pub.exe`; assert the subscriber prints `Received: id=1, message=Hello World`.
3. Confirm IPv6 multicast:
   * Set `CYCLONEDDS_URI` to the config file, a comma, then a bare inline XML string that enables tracing. `CYCLONEDDS_URI` accepts a comma-separated list parsed in order, so the inline string amends the file without editing it (`ddsi_config.c` splits on commas; a token starting with `<` is parsed as inline XML, otherwise opened as a file path). The committed `DDS/cyclonedds-config.xml` stays free of verbose tracing; tracing applies only to this invocation.
     `"file://T:/DDS/cyclonedds-config.xml,<CycloneDDS><Domain><Tracing><Verbosity>config</Verbosity><OutputFile>stderr</OutputFile></Tracing></Domain></CycloneDDS>"`
     `Verbosity=config` dumps the complete effective configuration to stderr; confirm the dump shows `udp6` and `StationsDDS`.
   * `netsh interface ipv6 show joins "StationsDDS"` shows the DDS multicast group joined on the adapter.

## Result

Done and verified on the development machine.

* The adapter existed as `StationDDS`; it was renamed to `StationsDDS` (matching the architecture), so `scripts/verify-loopback.ps1` now passes.
* `CYCLONEDDS_URI` accepts `file://` with two slashes or a plain path; the three-slash form `file:///...` fails with `can't open configuration file`. The plan and docs use the working form.
* `build/rust-dds-smoke/run-config-smoke.ps1` passes: the subscriber receives `Hello World`; the tracing dump shows `Domain/General/Transport: udp6` and `selected interfaces: StationsDDS`; `netsh interface ipv6 show joins "StationsDDS"` lists the CycloneDDS SPDP group `ff02::ffff:efff:1`.

## Blocker

None remaining.

## Files

* Authored/committed: `DDS/cyclonedds-config.xml`, `scripts/verify-loopback.ps1`, `docs/loopback-adapter.md`, `README.md`.
* Planning: `planning/current_story.md` (this file), `planning/backlog.md` (status), `planning/architecture.md` (script rename).
* Temporary, gitignored: `build/rust-dds-smoke/run-config-smoke.ps1`.
