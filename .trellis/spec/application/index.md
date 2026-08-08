# Application Development Guidelines

DiskScry is a new Rust CLI/TUI project for reading and monitoring disk information, with SMART data as its primary domain.

## Guidelines Index

| Guide | Use it for |
| --- | --- |
| [Project Scope](./project-scope.md) | Product boundaries, supported platforms, and decisions that are not yet settled |
| [Engineering Baseline](./engineering-baseline.md) | Evidence rules, platform boundaries, failure behavior, and verification expectations |
| [External Device SMART Visibility](./external-device-smart-visibility.md) | Required visibility and error behavior when an external disk has no readable SMART data |
| [Native SMART Architecture](./native-smart-architecture.md) | Rust module boundaries, macOS IOKit/Disk Arbitration contracts, refresh events, and validation |
| [Localization Contract](./localization-contract.md) | Locale precedence, bilingual human interfaces, and language-independent JSON contracts |
| [Readable SMART Projection](./readable-smart-projection.md) | CrystalDiskInfo-derived ATA metadata, NVMe units, shared CLI/TUI formatting, and SMART viewport behavior |
| [macOS Release Contract](./release-contract.md) | Version/tag alignment, dual-architecture archives, checksums, and publication gates |

## Pre-Development Checklist

Before planning or implementing product code:

1. Read [Project Scope](./project-scope.md).
2. Read [Engineering Baseline](./engineering-baseline.md).
3. Read [External Device SMART Visibility](./external-device-smart-visibility.md) for device enumeration, SMART acquisition, status, serialization, CLI, or TUI changes.
4. Read the shared [Thinking Guides](../guides/index.md) when a change crosses device acquisition, normalization, health evaluation, CLI, or TUI boundaries.
5. Read [Native SMART Architecture](./native-smart-architecture.md) before changing acquisition traits, protocol parsers, macOS FFI, refresh behavior, JSON, CLI, or TUI projections.
6. Read [Localization Contract](./localization-contract.md) before changing CLI help, human-readable output, TUI text, errors, locale handling, or JSON projections.
7. Read [Readable SMART Projection](./readable-smart-projection.md) before changing SMART names, units, human `show` output, the TUI SMART tab, or SMART viewport controls.
8. Read [macOS Release Contract](./release-contract.md) before changing package versions, release assets, install instructions, or target builds.
9. Run the validation commands in [Engineering Baseline](./engineering-baseline.md) after cross-layer changes.

## Quality Check

- Run formatting, Clippy, tests, documentation tests, and a release build.
- Run ignored macOS hardware tests outside restricted sandboxes.
- Confirm `diskscry list --json` preserves distinct SMART states.
- Confirm virtual whole disks are not emitted as physical disks.
- Confirm a Disk Arbitration event during an in-flight read discards the stale result and schedules a fresh inventory.
- Confirm English and Simplified Chinese human output preserve identical JSON schema vocabulary.
- Confirm readable SMART output preserves exact counters/raw bytes and unknown ATA IDs remain explicit.
