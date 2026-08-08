# Build macOS-first SMART disk monitor

## Goal

Build DiskScry as a Rust CLI/TUI application that makes physical-disk identity, SMART data, health evidence, and monitoring state easy to inspect from a terminal, with macOS as the primary platform and Linux as a supported secondary platform.

## Background

The repository contains the macOS Rust developer preview, shared domain contracts, protocol parsers, CLI/TUI, and native platform boundaries. CrystalDiskInfo is the reference for useful disk data and health presentation, not a source-code or Windows compatibility target.

The project-wide platform and failure constraints are defined in:

- `.trellis/spec/application/project-scope.md`
- `.trellis/spec/application/engineering-baseline.md`
- `.trellis/spec/application/external-device-smart-visibility.md`

## Delivery Map

- `.trellis/tasks/07-11-macos-developer-preview` owns the shared Rust model, protocol parsers, CLI/TUI, native macOS backend, tests, and developer-preview build pipeline.
- `.trellis/tasks/07-11-linux-native-smart-backend` owns Linux discovery, hotplug events, native NVMe and SG_IO acquisition, Linux tests, and cross-platform validation after the macOS contracts are stable.
- This parent task owns the cross-platform requirements and final integration review; implementation occurs in the child tasks.

## Requirements

- R1: The product is implemented in Rust and provides both CLI and interactive TUI workflows.
- R2: macOS is the primary implementation and validation platform.
- R3: Linux is supported after the macOS acquisition path is established; Windows is not supported in this task.
- R4: Device inventory includes all discovered internal and external physical disks.
- R5: For devices with readable SMART data, expose identity, temperature, health evidence, lifetime counters, and protocol-specific SMART attributes when the device provides them.
- R6: For an external disk without readable SMART data, keep the disk visible and display the exact primary label `SMART unavailable`.
- R7: Permission, acquisition, parsing, unsupported-device, and partial-data failures remain explicit and cannot be converted into healthy status or fabricated values.
- R8: CLI and TUI present consistent device identity, SMART availability, and health semantics.
- R9: The application refreshes disk information for ongoing monitoring without blocking terminal interaction.
- R10: The first release is strictly read-only and cannot start SMART self-tests or change SMART, APM, AAM, power, firmware, or other device settings.
- R11: Monitoring samples exist only in memory for the current process lifetime; the first release does not persist history, run a background service, or emit notifications.
- R12: The first release acquires SMART data through native Rust platform backends and does not invoke, bundle, link, or copy code from `smartctl` or smartmontools.
- R13: The macOS backend uses public Disk Arbitration and IOKit interfaces for device discovery and read-only ATA/NVMe SMART access. A device or transport that those interfaces cannot read remains visible with `SMART unavailable` and a diagnostic reason.
- R14: The Linux backend uses native kernel interfaces after the macOS acquisition path is established; it must preserve the same shared SMART availability and failure semantics.
- R15: Health evaluation is conservative and evidence-based. ATA health uses the device-reported SMART overall result and documented threshold crossings; NVMe health uses standardized critical-warning, temperature, and endurance fields. Missing or insufficient evidence produces `Unknown`.
- R16: Vendor-specific ATA raw values remain visible as evidence when available but cannot independently produce a warning or failure state without an approved, documented interpretation rule.
- R17: Running `diskscry` without a subcommand starts the interactive TUI. Non-interactive workflows are exposed through `list`, `show <device>`, and `watch [<device>]` subcommands.
- R18: `list` and `show` support stable machine-readable JSON output in addition to human-readable terminal output. `watch` continuously refreshes terminal output without persisting samples.
- R19: Disk insertion and removal update inventory through platform events rather than periodic full-device scanning.
- R20: SMART data is read once at startup and refreshes every 60 seconds by default in the TUI and `watch`. Users can override the interval; an interval of zero disables automatic SMART refresh.
- R21: The TUI provides an immediate manual refresh action bound to `r`. Product documentation and help text state that reading SMART may wake a sleeping mechanical disk.
- R22: DiskScry never invokes `sudo`, requests credentials, installs a privileged helper, or changes device-permission rules. It runs with the caller's privileges and preserves permission failures as explicit SMART read failures.
- R23: When elevated access is required, CLI and TUI diagnostics explain that the user may relaunch DiskScry with administrator privileges; elevation remains an explicit user action.
- R24: The first release is a developer preview installable from source with Cargo. CI produces build artifacts for macOS Apple Silicon, macOS Intel, and Linux x86_64, but end-user package-manager publication and macOS signing/notarization are separate release work.
- R25: The TUI uses a single-screen split layout with a disk list, a selected-disk detail area, and a status/footer area. Detail views cover overview, protocol-specific SMART data, and current-session monitoring.
- R26: The current-session view includes an in-memory temperature sparkline when temperature samples are available. It starts empty on each process launch and never implies persisted history.
- R27: Keyboard controls support arrow keys or `j`/`k` for selection, `Tab` for detail-view navigation, `r` for refresh, and `q` for exit. Mouse input and complex configuration screens are outside the first release.
- R28: The developer-preview CLI and TUI support English and Simplified Chinese user-facing text. `LC_ALL`/`LANG` select the default language and `--lang en|zh-CN` explicitly overrides it.
- R29: Command names, arguments, JSON keys, JSON enum values, and the canonical internal label `SMART unavailable` remain English-only stable interfaces. The Chinese human-readable projection renders `SMART 不可用` without changing serialized values.

## Acceptance Criteria

- [ ] A macOS user can launch DiskScry and see every discovered internal and external physical disk.
- [ ] A disk with readable SMART data shows its available identity, health, temperature, lifetime, and protocol-specific attributes.
- [ ] An external disk without readable SMART data remains visible with `SMART unavailable`.
- [ ] A SMART permission, acquisition, or parse failure remains attached to the device and does not produce a healthy status.
- [ ] CLI and TUI output agree on device identity, SMART availability, and health meaning.
- [ ] Monitoring refreshes do not block keyboard input or terminal redraws.
- [ ] Any trend or change view uses only samples collected during the current process and starts empty after restart.
- [ ] No CLI command or TUI action issues a device-setting change, self-test start, firmware operation, or other write-capable storage command.
- [ ] The application acquires SMART data without installing, invoking, bundling, or linking `smartctl` or smartmontools.
- [ ] The macOS acquisition layer uses only public read-only system interfaces and isolates unsafe FFI from shared parsing, health, CLI, and TUI code.
- [ ] ATA and NVMe fixtures produce health results traceable to the exact device-reported status, threshold crossing, or standardized NVMe field that caused them.
- [ ] Missing SMART data and uninterpreted vendor-specific raw attributes cannot produce `Healthy`, `Warning`, or `Critical`; they remain `Unknown` unless separate documented evidence exists.
- [ ] Bare `diskscry` starts the TUI, while `list`, `show`, and `watch` operate without entering the interactive interface.
- [ ] `diskscry list --json` and `diskscry show <device> --json` serialize the same normalized snapshot and SMART-state distinctions consumed by the TUI.
- [ ] Device insertion or removal updates the active inventory without waiting for a periodic rescan.
- [ ] Automatic SMART refresh defaults to 60 seconds, accepts a configured interval including zero, and manual refresh updates the current snapshot without blocking terminal input.
- [ ] User-facing help discloses that SMART reads may wake sleeping mechanical disks.
- [ ] DiskScry performs no automatic privilege escalation or permission-rule modification.
- [ ] A permission-denied SMART read keeps the disk visible, renders `SMART unavailable`, preserves a failed state in JSON, and provides actionable elevation guidance.
- [ ] `cargo install --path .` installs a working `diskscry` executable on supported development targets.
- [ ] CI builds the macOS Apple Silicon, macOS Intel, and Linux x86_64 targets without requiring release signing credentials.
- [ ] The TUI keeps the complete disk list visible while showing overview, SMART, or session details for the selected disk.
- [ ] Session temperature visualization uses only samples collected by the current process and handles unavailable temperature without fabricated points.
- [ ] The documented keyboard controls work without requiring mouse input.
- [ ] CLI help, human-readable output, TUI labels, and actionable errors render in English or Simplified Chinese according to locale selection.
- [ ] `--lang en|zh-CN` overrides environment locale selection, while command names and JSON output remain byte-for-byte language independent.
- [ ] The supported macOS and Linux targets build and pass their defined test suites.
- [ ] No Windows-specific implementation is required.

## Out of Scope

- Windows support.
- Web UI, API server, database, and mobile interfaces.
- Claims of failure prediction beyond the evidence reported by the disk and documented application rules.
- Starting or stopping SMART self-tests.
- Changing SMART, APM, AAM, power, firmware, or other device settings.
- Persisted monitoring history, databases, background daemons, and system notifications.
- Bundling or requiring `smartctl` or smartmontools.
- Copying smartmontools source code, device tables, or other GPL-derived implementation material.
- First-release support for proprietary USB bridge pass-through protocols that are not exposed through the selected public platform interfaces.
- Automatic privilege escalation, credential prompts, privileged helpers, and automatic Linux udev-rule installation.
- Homebrew Formula publication, macOS Developer ID signing, notarization, and production release-channel management.
- Mouse interaction and complex in-application configuration screens.
- Languages other than English and Simplified Chinese, including Traditional Chinese.
