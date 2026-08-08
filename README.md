# DiskScry

DiskScry is a read-only Rust CLI/TUI for inspecting physical disks and SMART health evidence. The developer preview targets macOS first; Linux support follows after the shared contracts are stable.

## Install

Install the tagged source release with Rust and the macOS Command Line Tools:

```bash
cargo install --git https://github.com/ZHOUSJ6/DiskScry --tag v0.1.0 --locked
```

The [v0.1.0 release](https://github.com/ZHOUSJ6/DiskScry/releases/tag/v0.1.0) also provides archives for Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`). The developer-preview binaries are unsigned and not notarized.

## Commands

```text
diskscry                         Start the TUI
diskscry list [--json]           List physical disks
diskscry show <device> [--json]  Show one disk by id or device node
diskscry watch [<device>]        Refresh disk information continuously
```

Global options can appear before or after a subcommand:

```text
--lang <en|zh-CN>    Override the human interface language
--interval <seconds> Set the SMART refresh interval; zero disables scheduled refresh
```

## Language

The CLI, TUI, help, and actionable error guidance support English and Simplified Chinese. DiskScry checks `LC_ALL` first, then `LANG`, and defaults to English when the selected locale is unsupported. `--lang` has the highest priority.

```bash
diskscry --lang zh-CN list
LC_ALL=zh_CN.UTF-8 diskscry
```

Command names, arguments, JSON keys, and JSON enum values remain English in every locale. Human-readable Chinese output renders an inaccessible SMART state as `SMART 不可用`; JSON continues to emit the stable `unavailable` or `failed` state.

DiskScry never invokes `sudo` or changes device permissions. When macOS denies access to a supported SMART interface, the disk remains visible with the locale-specific unavailable label. Relaunching with administrator privileges is an explicit user decision.

SMART reads may wake sleeping mechanical disks. TUI and `watch` refresh every 60 seconds by default; `--interval 0` disables scheduled refresh for `watch`.

## TUI controls

```text
j / k, ↑ / ↓       Select a disk
Tab                 Switch Overview, SMART, and Session tabs
v                   Toggle readable SMART and raw JSON views
PageUp / PageDown   Scroll SMART data by one page
Home / End          Jump to the start or end of SMART data
r                   Refresh now
q                   Quit
```

The readable SMART view shows localized ATA/NVMe metric names, human units, and exact device counters or ATA raw bytes. `diskscry show <device>` uses the same projection; JSON output remains unchanged.

## Scope

- Native macOS IOKit discovery and ATA/NVMe SMART interfaces
- Disk Arbitration insertion and removal events
- Explicit `Available`, `Unavailable`, and `Failed` JSON states
- Conservative ATA/NVMe health evaluation with causal evidence
- In-memory session temperature samples only
- English and Simplified Chinese human interfaces with locale detection
- Revision-pinned generic ATA attribute names derived from MIT-licensed CrystalDiskInfo 9.9.1
- Third-party attribution recorded in [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md)
- No smartmontools dependency, device writes, persistence, daemon, or notifications

USB bridges that do not expose SMART through public macOS interfaces remain listed with the locale-specific unavailable label.

## Validate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo build --release
```
