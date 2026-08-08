# DiskScry

English | [简体中文](./README.zh-CN.md)

[![CI](https://github.com/ZHOUSJ6/DiskScry/actions/workflows/ci.yml/badge.svg)](https://github.com/ZHOUSJ6/DiskScry/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/ZHOUSJ6/DiskScry)](https://github.com/ZHOUSJ6/DiskScry/releases/latest)

DiskScry is a read-only Rust CLI/TUI for inspecting physical disks and SMART health evidence. It presents ATA and NVMe data in a readable CrystalDiskInfo-style view while preserving exact device counters and raw values.

The `v0.1.0` developer preview supports macOS on Apple Silicon and Intel. Linux support is planned; Windows is currently out of scope.

## Features

- Discover internal and external physical disks through native macOS APIs
- Read ATA and NVMe SMART data without invoking smartmontools
- Keep external disks visible when SMART cannot be read
- Distinguish `Available`, `Unavailable`, and `Failed` SMART states
- Show readable metrics, hexadecimal metric codes, human units, and exact raw evidence
- Provide English and Simplified Chinese CLI, TUI, help, and diagnostics
- Monitor temperature samples for the current session without writing to disk
- React to disk insertion and removal through Disk Arbitration

## Install

### Cargo

Rust and the macOS Command Line Tools are required:

```bash
cargo install --git https://github.com/ZHOUSJ6/DiskScry --tag v0.1.0 --locked
```

### Prebuilt macOS binaries

Download the archive matching your Mac from the [v0.1.0 release](https://github.com/ZHOUSJ6/DiskScry/releases/tag/v0.1.0):

| Mac | Archive |
| --- | --- |
| Apple Silicon | `diskscry-v0.1.0-aarch64-apple-darwin.tar.gz` |
| Intel | `diskscry-v0.1.0-x86_64-apple-darwin.tar.gz` |

Extract the archive and place `diskscry` in a directory on your `PATH`:

```bash
tar -xzf diskscry-v0.1.0-aarch64-apple-darwin.tar.gz
mkdir -p "$HOME/.local/bin"
install -m 755 diskscry "$HOME/.local/bin/diskscry"
```

The developer-preview binaries are unsigned and not notarized. Verify the downloaded archive against the release's `SHA256SUMS` file before installing it.

## Usage

Running `diskscry` without a subcommand opens the TUI:

```bash
diskscry
```

| Command | Description |
| --- | --- |
| `diskscry list [--json]` | List physical disks |
| `diskscry show <device> [--json]` | Show one disk by ID or device node |
| `diskscry watch [<device>]` | Refresh disk information continuously |

Global options may appear before or after a subcommand:

| Option | Description |
| --- | --- |
| `--lang <en\|zh-CN>` | Override the human-interface language |
| `--interval <seconds>` | Set the SMART refresh interval; `0` disables scheduled refresh |

Examples:

```bash
diskscry --lang zh-CN
diskscry list --json
diskscry show /dev/disk0
diskscry watch /dev/disk0 --interval 30
```

## SMART behavior

Disk inventory and SMART acquisition are separate. Every discovered physical disk remains visible even when its transport does not expose SMART.

| State | Meaning |
| --- | --- |
| `Available` | SMART data was read and parsed successfully |
| `Unavailable` | The device or public transport does not expose readable SMART data |
| `Failed` | DiskScry attempted a supported read but encountered a permission, native API, or parse error |

Unavailable and failed devices show `SMART unavailable` in English or `SMART 不可用` in Chinese. Missing data never becomes a healthy result.

The readable view shows NVMe metric codes from `01` through `0F` and device-reported ATA attribute IDs such as `05` and `C5`. Press `v` in the TUI to inspect the unchanged raw JSON representation.

## Language

DiskScry selects the interface language in this order:

1. `--lang en` or `--lang zh-CN`
2. `LC_ALL`
3. `LANG`
4. English fallback

Command names, option names, device identifiers, JSON keys, and JSON enum values remain language-independent English.

## TUI controls

| Key | Action |
| --- | --- |
| `j` / `k`, `↑` / `↓` | Select a disk |
| `Tab` | Switch Overview, SMART, and Session tabs |
| `v` | Toggle readable SMART and raw JSON views |
| `PageUp` / `PageDown` | Scroll SMART data by one page |
| `Home` / `End` | Jump to the start or end of SMART data |
| `r` | Refresh now |
| `q` | Quit |

## Safety and limitations

- DiskScry uses read-only native interfaces and never issues device write commands.
- DiskScry never invokes `sudo`, changes permissions, or silently switches acquisition backends.
- SMART reads may wake sleeping mechanical disks. TUI and `watch` refresh every 60 seconds by default.
- Some USB bridges do not expose SMART through public macOS interfaces; these disks remain listed as SMART unavailable.
- History persistence, notifications, self-tests, privileged helpers, signing, and notarization are not included in the developer preview.

CrystalDiskInfo 9.9.1 is the reference for the revision-pinned generic ATA attribute-name catalog. DiskScry does not link or bundle CrystalDiskInfo code. See [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md) for attribution.

## Development

Pass application arguments after Cargo's `--` separator:

```bash
cargo run -- --lang zh-CN
```

Run the quality gate from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --release --target aarch64-apple-darwin --locked
cargo build --release --target x86_64-apple-darwin --locked
```
