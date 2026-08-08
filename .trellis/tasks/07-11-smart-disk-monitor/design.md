# SMART disk monitor design

## System boundaries

DiskScry separates physical-disk discovery from SMART enrichment. Platform code discovers every physical disk first, then attaches one explicit SMART state to each normalized snapshot.

```text
platform inventory event
        |
        v
physical device record --> read-only SMART request
        |                         |
        |                         v
        |                 ATA / NVMe transport
        |                         |
        +-------------------------+
                    |
                    v
             normalized snapshot
                    |
          +---------+---------+
          |                   |
          v                   v
     health evaluator     session samples
          |                   |
          +---------+---------+
                    |
             CLI / JSON / TUI
```

The shared model, protocol parsers, health evaluator, and presentation layers contain no platform system calls. Platform modules contain no CLI or TUI formatting.

## Delivery sequence

1. The macOS developer-preview child establishes the shared contracts and proves them through native Disk Arbitration and IOKit integration.
2. The Linux child implements the same platform traits using sysfs, kernel device events, NVMe ioctls, and SCSI generic ioctls.
3. The parent closes only after both child acceptance suites pass and equivalent fixtures produce equivalent shared states.

## Shared model

The shared data model preserves identity, protocol evidence, availability, errors, and derived health separately.

```rust
struct DeviceSnapshot {
    id: DeviceId,
    device_node: PathBuf,
    identity: DeviceIdentity,
    connection: ConnectionInfo,
    capacity_bytes: u64,
    external: bool,
    smart: SmartState,
    health: HealthState,
    observed_at: SystemTime,
}

enum SmartState {
    Available { snapshot: SmartSnapshot },
    Unavailable { reason: SmartUnavailableReason },
    Failed { error: SmartReadError },
}

enum SmartSnapshot {
    Ata(AtaSmartSnapshot),
    Nvme(NvmeSmartSnapshot),
}

enum HealthState {
    Healthy { evidence: Vec<HealthEvidence> },
    Warning { evidence: Vec<HealthEvidence> },
    Critical { evidence: Vec<HealthEvidence> },
    Unknown { reason: HealthUnknownReason },
}
```

`DeviceId` is stable for the current process and derived from the strongest platform identity available. The CLI selector accepts an exact emitted `id` or exact device node. Ambiguous or stale selectors fail explicitly.

JSON uses a top-level `schema_version` and snake-case internally tagged states. `Available`, `Unavailable`, and `Failed` are never represented by a shared nullable field.

## Protocol parsing

ATA and NVMe parsers operate on byte slices and return typed protocol values or offset-aware parse errors. They do not perform I/O.

- ATA parsing covers IDENTIFY DEVICE, the 512-byte SMART data page, SMART thresholds, checksum validation, device-reported overall status, and preserved raw attribute bytes.
- NVMe parsing covers Identify Controller and the standardized SMART / Health Information log needed for critical warnings, temperature, spare capacity, percentage used, data units, power cycles, power-on hours, unsafe shutdowns, and media errors.
- Vendor-specific ATA raw values remain visible without creating health conclusions.
- Unknown fields are preserved where the public output contract exposes them; missing fields remain absent rather than becoming zero.

Binary parser fixtures contain only recorded or constructed protocol pages. They are test inputs and are never used as runtime success data.

## Health evaluation

Health evaluation is a pure function over a normalized SMART snapshot.

- ATA uses device-reported overall status and documented normalized-value threshold crossings.
- NVMe uses standardized critical-warning bits and documented temperature or endurance evidence.
- Missing SMART, partial evidence, or uninterpreted vendor raw values result in `Unknown`.
- Every non-unknown result contains the evidence that caused it.

Presentation assigns color only after health evaluation. CLI and TUI cannot derive health independently.

## macOS platform design

Disk Arbitration supplies whole-disk appearance and disappearance events. IOKit supplies physical-device identity, connection properties, registry ancestry, and read-only SMART plugin interfaces.

- ATA reads use `IOATASMARTInterface` methods for identify, SMART data, thresholds, log reads when required, and overall status.
- NVMe reads use `IONVMeSMARTInterface` methods for identify, SMART data, and standardized log pages.
- Rust uses `objc2-io-kit` and Core Foundation bindings where coverage exists. Target-gated minimal FFI definitions cover public SMART C interface tables that are not exposed by those crates.
- Unsafe calls, retained object ownership, interface acquisition, release order, pointer validation, and buffer lengths are confined to the macOS platform module.
- No write-capable SMART method is exposed through the safe Rust transport trait.
- A USB bridge that is visible as a disk but does not expose a supported public SMART interface produces `Unavailable`, not a failed inventory and not a fabricated healthy result.

## Linux platform design

Linux discovery reads physical block-device identity and topology from sysfs and receives kernel device events for insertion and removal.

- Native NVMe devices use the Linux NVMe admin ioctl for Identify and SMART / Health log pages.
- ATA devices reachable through the SCSI layer use `SG_IO` with read-only ATA PASS-THROUGH commands.
- Unsupported or non-passthrough USB bridges produce `Unavailable` with a transport reason.
- ioctl request structures and unsafe calls remain target-gated inside the Linux platform module.
- Permission failures remain `Failed` and include the operation and device node.

Proprietary USB bridge command sets are outside the first release.

## Application and refresh model

The application owns one in-memory device store keyed by `DeviceId`.

- Inventory callbacks send typed insert/remove/change events.
- SMART requests run on blocking worker threads so terminal input and redraw remain responsive.
- The scheduler reads all current devices once at startup and every 60 seconds by default.
- An interval of zero disables scheduled reads; manual refresh still submits an explicit request.
- Results include device identity and request generation so a late result cannot be attached to a removed or replaced disk.
- Session history is a bounded in-memory series owned by the application state and discarded on exit.

No acquisition mechanism silently substitutes for a failed mechanism.

## CLI and JSON

`clap` derives the command model with an optional subcommand. Absence of a subcommand enters the TUI.

```text
diskscry
diskscry list [--json]
diskscry show <device> [--json]
diskscry watch [<device>] [--interval <seconds>]
```

Human-readable output and JSON project the same `DeviceSnapshot`. JSON output is deterministic for fixtures and includes `schema_version`, acquisition state, health evidence, timestamps, and diagnostic details.

## TUI

Ratatui owns rendering and Crossterm owns terminal events. The UI is an immediate projection of application state.

- Left pane: physical-disk table with device, model, capacity, connection, health, temperature, and SMART state.
- Right pane: Overview, SMART, and Session tabs for the selected disk.
- Footer: last refresh, active error, interval, and keyboard shortcuts.
- Session renders a temperature sparkline only when samples exist.
- Terminal restoration runs on normal exit and propagated error paths.
- Ratatui `TestBackend` snapshots verify narrow, normal, unavailable-SMART, permission-error, and hotplug layouts.

## Privilege and safety model

DiskScry never changes privileges or permissions. An unprivileged inventory remains useful even when SMART access fails. Permission diagnostics describe the failed operation and allow the user to choose whether to relaunch with administrator privileges.

All transports are read-only by construction. Safe traits contain no operation for self-tests, SMART enable/disable, log writes, firmware, power management, or device settings.

## Build and release model

The developer preview supports source installation through Cargo. CI performs formatting, linting, unit tests, fixture tests, target-specific builds, and artifact production for:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-unknown-linux-gnu`

Homebrew publication, signing, notarization, credentialed release automation, Windows code, persistent storage, and background services remain outside this design.

## Validation strategy

- Pure parser and health tests cover ATA and NVMe good, threshold, critical, truncated, invalid-checksum, and unknown-field cases.
- Contract tests verify that inventory retains external disks for every SMART state.
- Serialization tests lock the tagged JSON schema and diagnostic distinction.
- TUI rendering tests use deterministic application state and a test terminal backend.
- Platform tests isolate public system-call wrappers and include permission and unsupported-interface failures.
- Hardware validation records model, connection, protocol, privilege level, and observed availability without storing serial numbers in committed fixtures.
