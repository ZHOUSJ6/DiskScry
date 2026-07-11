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
- Display the exact primary label `SMART unavailable`.
- Preserve an unsupported or failed reason for diagnostics.
- Do not derive a healthy status from missing SMART data.

The executable cross-layer contract is defined in [External Device SMART Visibility](./external-device-smart-visibility.md).

## Current Repository State

The repository has no `Cargo.toml`, `src/`, test suite, release configuration, or established Rust module layout. The generated backend/frontend Trellis templates did not describe this project and were removed during bootstrap.

Until product code exists:

- Do not document hypothetical modules as existing conventions.
- Do not add database, web frontend, TypeScript, API server, or ORM rules without an approved task that introduces them.
- Treat the first Rust implementation task as the source of truth for initial module and test structure, then update these specs with real file paths and symbols.

## Decisions Requiring A Product Task

The following choices are not settled by the current repository:

- The SMART acquisition backend and whether an external tool is a runtime dependency.
- Persistence, history retention, alerting, self-tests, and privileged write operations.
- Packaging outside the initial macOS-first distribution path.

An implementation task must resolve any choice it depends on in its PRD or design before code is written.
