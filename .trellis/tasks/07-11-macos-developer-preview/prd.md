# Build macOS developer preview

## Goal

Deliver a source-installable Rust CLI/TUI that discovers every macOS physical disk, reads ATA or NVMe SMART data through public native interfaces when available, and preserves explicit unavailability and failures.

## Parent contract

This child implements the first deliverable of `.trellis/tasks/07-11-smart-disk-monitor`. The parent PRD owns cross-platform product semantics; this child establishes the shared contracts that the Linux backend must reuse.

## Requirements

- M1: Create the Rust workspace, executable, shared device model, protocol parsers, health evaluator, application state, CLI, JSON, and TUI.
- M2: Discover all internal and external whole physical disks through macOS Disk Arbitration and IOKit without filtering on SMART support.
- M3: Read ATA identify, SMART data, thresholds, and overall status through the public `IOATASMARTInterface` read methods.
- M4: Read NVMe identify and standardized SMART / Health data through the public `IONVMeSMARTInterface` read methods.
- M5: Isolate unsafe FFI and Apple object ownership inside the macOS platform module; safe shared traits expose read-only operations only.
- M6: Represent supported data as `Available`, unsupported public transports as `Unavailable`, and attempted permission/acquisition/parse errors as `Failed`.
- M7: Implement the parent CLI/TUI, 60-second default refresh, manual refresh, current-session samples, English/Simplified Chinese interface, and language-independent JSON contracts.
- M8: Do not invoke or bundle smartmontools, elevate privileges, persist history, issue write commands, or implement proprietary USB bridge protocols.
- M9: Support source installation and CI builds for Apple Silicon and Intel macOS.
- M10: Select the default human language from `LC_ALL` then `LANG`, support `--lang en|zh-CN` override, and keep command names and machine-readable values in English.
- M11: Replace the SMART tab's raw JSON presentation with a CrystalDiskInfo-style readable summary and protocol-specific metric table for ATA and NVMe devices.
- M12: Retain raw SMART JSON in the TUI and use `v` to switch between the readable SMART view and the raw diagnostic view.
- M13: Selectively port CrystalDiskInfo's MIT-licensed ATA attribute naming and interpretation rules into pure Rust without linking its Windows C++ implementation or changing the native macOS acquisition backend.
- M14: Vendor CrystalDiskInfo-derived rules as a reviewed, revision-pinned Rust data snapshot with no runtime network access or automatic upstream updates.
- M15: Keep existing disk-selection keys and use `PageUp`/`PageDown` plus `Home`/`End` for SMART-table scrolling; reset scroll when selection or snapshot changes.
- M16: Show formatted and exact values together: NVMe metrics include human units plus original counters, while ATA rows retain ID, normalized values, threshold, interpreted value when reliable, and raw bytes.
- M17: Reuse one readable SMART projection in the TUI and human `show` output; `show --json` retains the existing versioned machine contract.
- M18: Limit the first CrystalDiskInfo-derived catalog to its generic `[Smart]` ATA attributes and reliable common interpretations. Vendor-specific sections remain future, sample-driven work.

## Acceptance Criteria

- [x] Bare `diskscry` opens the split-pane TUI and lists every discovered whole physical disk.
- [x] `list`, `show`, and `watch` operate non-interactively and use the shared normalized snapshots.
- [x] `list --json` and `show --json` preserve tagged SMART and health states with a schema version.
- [x] An external disk without a readable public SMART interface remains visible with `SMART unavailable` and an `Unavailable` reason.
- [x] A permission or read failure remains visible with `SMART unavailable` and a distinct `Failed` error.
- [x] Supported ATA and NVMe fixtures parse into typed evidence and conservative health states.
- [x] Hotplug updates inventory and stale asynchronous reads cannot attach to a removed device.
- [x] Default, disabled, overridden, and manual refresh paths behave as defined without blocking terminal input.
- [x] TUI test rendering covers available, unavailable, failed, empty-session, and sampled-session states.
- [x] The source tree contains no smartmontools code and the executable works when `smartctl` is absent.
- [x] `cargo install --path .` succeeds on macOS and CI builds both declared macOS architectures.
- [x] English and Simplified Chinese CLI help, output, TUI, and actionable errors render from one centralized text catalog.
- [x] Locale environment selection and `--lang` override are tested; JSON output is identical across languages.
- [x] The SMART tab shows localized health, temperature, identity, and protocol-specific SMART metrics with readable names, values, units, and warning emphasis.
- [x] Unavailable and failed SMART states remain readable and retain their diagnostic reason or error.
- [x] Pressing `v` on the SMART tab toggles between the readable view and raw JSON without changing the selected disk or snapshot.
- [x] Vendored CrystalDiskInfo-derived rules record their upstream revision and MIT attribution; unmatched attributes remain explicitly unknown.
- [x] Builds remain reproducible and SMART interpretation never changes without a reviewed DiskScry source update.
- [x] Long SMART tables scroll without changing the selected disk, and selection or refresh cannot leave the table at an invalid offset.
- [x] Human formatting never replaces exact device evidence; users can inspect original NVMe counters and ATA raw bytes.
- [x] `show <device>` and the TUI use identical names, units, and status semantics while JSON remains unchanged.
- [x] The first ATA catalog covers the pinned CrystalDiskInfo generic `[Smart]` names in English and Simplified Chinese; vendor-only meanings are not guessed.

## Technical Notes

- CrystalDiskInfo is MIT-licensed, but its SMART implementation is a Windows C++ application component rather than a portable library. It depends on the Visual Studio solution, MFC/Windows types, Windows storage transports, and separately downloaded resource data.
- DiskScry cannot directly link CrystalDiskInfo on macOS. It can port selected MIT-licensed attribute naming and interpretation rules into Rust while retaining the CrystalDiskInfo copyright and license notice.
- The approved boundary keeps macOS IOKit acquisition, Rust ATA/NVMe parsing, normalized JSON, and conservative health evaluation owned by DiskScry. Only human-readable attribute metadata and interpretation rules are derived from CrystalDiskInfo.
- The approved update model vendors a fixed upstream revision into the repository, requires manual review for refreshes, and performs no runtime downloads.
- The approved navigation keeps `j`/`k` and arrow keys for disk selection; `PageUp`/`PageDown` and `Home`/`End` control the SMART-table viewport.
- The approved value projection shows formatted units and exact raw evidence together rather than replacing device-reported counters.
- The approved projection is shared by TUI and human CLI output; JSON serialization does not consume it.
- The approved first catalog ports the generic `[Smart]` table only. CrystalDiskInfo vendor sections require future model classification and hardware fixtures.
- The pinned upstream is CrystalDiskInfo 9.9.1 commit `fdc8bce73ab0355c513c758ebf0f0f22662830e2`.

## Out of Scope

- Linux implementation and validation.
- Proprietary USB bridge pass-through.
- Privileged helpers and automatic elevation.
- Languages beyond English and Simplified Chinese, persistence, notifications, device writes, package-manager publication, signing, and notarization.
