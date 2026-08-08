# Project Scope

## Product

DiskScry is a terminal application written in Rust. It provides CLI and TUI views for reading and monitoring physical-disk information, with SMART data as the primary source of health information. CrystalDiskInfo is a data and behavior reference, not a codebase or platform target.

## Platform Priority

1. macOS is the primary implementation and validation platform.
2. Linux is supported after the macOS path is established.
3. Windows is outside the current scope.

Platform-specific behavior must remain visible in product behavior and tests. A value that is unavailable on macOS or Linux must be represented as unavailable or unsupported; it must not be fabricated from a default value.

## External Disk Visibility

All external physical disks discovered by macOS or Linux must remain visible in CLI and TUI device lists. SMART support is enrichment data and must never be used as a device-list filter.

When SMART data cannot be read:

- Keep the external disk visible.
- Display `SMART unavailable` in English and the exact human label `SMART 不可用` in Simplified Chinese.
- Preserve an unsupported or failed reason for diagnostics.
- Do not derive a healthy status from missing SMART data.

The executable cross-layer contract is defined in [External Device SMART Visibility](./external-device-smart-visibility.md).

## Repository Structure

The Rust package is defined by `Cargo.toml` and `Cargo.lock`. Product code lives under `src/`:

- `src/domain/` owns normalized device, SMART, and health types.
- `src/protocol/` owns ATA and NVMe byte parsing.
- `src/platform/` owns operating-system discovery, events, and transport calls.
- `src/app.rs` owns SMART enrichment and normalized snapshots.
- `src/cli.rs` and `src/presentation/` own user-facing projections.

Do not add database, web frontend, TypeScript, API server, or ORM rules without an approved task that introduces them.

## Decisions Requiring A Product Task

The current product decisions are:

- SMART acquisition uses native Rust platform backends and does not invoke or bundle smartmontools.
- The first release is read-only and keeps monitoring samples in memory for the process lifetime.
- Alerts, persistence, self-tests, privileged helpers, Homebrew publication, signing, and notarization remain outside the developer preview.
- Source installation and macOS Apple Silicon/Intel build artifacts are the initial distribution path.
- English and Simplified Chinese are the supported human interface languages. CLI syntax and JSON vocabulary remain language-independent English.
- Readable ATA names may derive from a revision-pinned, attributed CrystalDiskInfo MIT metadata snapshot. CrystalDiskInfo transport, Windows code, runtime resources, and automatic updates remain excluded.

An implementation task must resolve any choice it depends on in its PRD or design before code is written.
