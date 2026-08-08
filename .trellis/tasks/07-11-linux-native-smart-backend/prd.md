# Add Linux native SMART backend

## Goal

Extend the macOS-established shared contracts with Linux physical-disk discovery, hotplug events, and native read-only NVMe and ATA SMART acquisition.

## Dependency

This child begins after `.trellis/tasks/07-11-macos-developer-preview` stabilizes the shared model, parsers, health semantics, JSON schema, and presentation contracts.

## Requirements

- L1: Discover every Linux whole physical block device without filtering on SMART availability.
- L2: Preserve physical identity, device node, capacity, connection, removable/external classification, and hotplug changes from native Linux sources.
- L3: Read native NVMe Identify and SMART / Health data through Linux NVMe admin ioctls.
- L4: Read ATA SMART data through read-only `SG_IO` ATA PASS-THROUGH where the kernel and transport expose it.
- L5: Keep ioctl definitions and unsafe calls isolated inside the Linux platform module.
- L6: Reuse the shared ATA/NVMe parsers, health evaluator, application model, CLI, JSON, and TUI without Linux-specific presentation branches.
- L7: Preserve unsupported bridge and permission failures as explicit `Unavailable` and `Failed` states.
- L8: Support Linux x86_64 CI and source installation without smartmontools.

## Acceptance Criteria

- [ ] Every discovered whole physical Linux disk appears in CLI and TUI inventory.
- [ ] Native NVMe fixtures and integration paths populate the shared NVMe snapshot and health evidence.
- [ ] Supported SG_IO ATA paths populate the shared ATA snapshot and health evidence.
- [ ] An unsupported external USB bridge remains visible with `SMART unavailable` and a transport reason.
- [ ] A permission or ioctl failure remains visible with `SMART unavailable` and a distinct failed state.
- [ ] Hotplug uses native device events and stale reads cannot attach to removed devices.
- [ ] Existing macOS, shared parser, JSON, CLI, and TUI tests continue to pass unchanged.
- [ ] Linux x86_64 CI passes formatting, linting, tests, release build, and source-install validation.
- [ ] The Linux executable works without `smartctl` or smartmontools installed.

## Out of Scope

- Windows and non-Linux Unix platforms.
- Proprietary USB bridge protocols and RAID-controller command sets.
- Automatic udev-rule installation, privileged helpers, persistence, notifications, and device writes.
