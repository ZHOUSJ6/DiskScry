# Native SMART Architecture

## 1. Scope / Trigger

This contract applies to changes in physical-disk discovery, Disk Arbitration events, IOKit SMART transport, ATA/NVMe parsing, refresh scheduling, normalized snapshots, JSON, CLI, or TUI presentation.

## 2. Signatures

Platform boundaries are defined in `src/platform/mod.rs`:

```rust
trait DeviceInventory {
    fn list(&self) -> Result<Vec<DeviceRecord>, PlatformError>;
}

trait SmartReader {
    fn read(&self, device: &DeviceRecord) -> SmartState;
}

trait DeviceEventSource {
    type Subscription: DeviceEventSubscription;
    fn subscribe(&self) -> Result<Self::Subscription, PlatformError>;
}
```

The normalized read flow is defined in `src/app.rs`:

```rust
fn collect_snapshots<I: DeviceInventory, R: SmartReader>(
    inventory: &I,
    reader: &R,
) -> Result<Vec<DeviceSnapshot>, AppError>;
```

## 3. Contracts

- `src/platform/macos/inventory.rs` enumerates whole physical `IOMedia` objects and filters APFS synthesized media and HDIX disk images.
- `src/platform/macos/events.rs` uses Disk Arbitration appeared/disappeared callbacks. Event bursts are coalesced before `watch` refreshes.
- `src/platform/macos/smart.rs` traverses IOKit parents to `SMART Capable` or `NVMe SMART Capable`, acquires the public plugin interface, and exposes read methods only.
- `src/protocol/ata.rs` and `src/protocol/nvme.rs` parse owned byte buffers without performing I/O.
- `src/domain/health.rs` is the only owner of derived health.
- `src/app.rs` attaches one explicit SMART state to every discovered device.
- `src/cli.rs` and `src/presentation/` consume `DeviceSnapshot`; they do not inspect raw platform data.
- A Disk Arbitration event received during an in-flight refresh marks that result stale. The stale batch is discarded and a new inventory/read cycle is scheduled.
- Restricted process sandboxes can return IOKit `0xe00002be` or a null Disk Arbitration session even when normal user execution succeeds. These failures remain explicit; production code does not switch backends.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Physical disk with readable core SMART data | `Available`, warnings retained separately, evaluated health |
| Visible physical disk without a public SMART interface | `Unavailable::InterfaceNotExposed`, unknown health |
| Plugin/interface acquisition fails | `Failed` at `InterfaceAcquisition`, native code retained |
| Core SMART read fails | `Failed` at `SmartData`, disk retained |
| Optional identify/threshold/status read fails after core data succeeds | `Available` with `warnings` |
| ATA checksum or NVMe length validation fails | `Failed` at `Parse` |
| APFS synthesized media or HDIX image | Excluded from physical inventory |
| Hotplug occurs during a read | Current result discarded, fresh batch scheduled |

## 5. Good / Base / Bad Cases

- Good: Native NVMe exposes identify and health data. CLI and TUI show temperature, counters, evidence, and `Healthy` when standardized evidence supports it.
- Base: An external bridge exposes a physical disk but no public SMART service. The disk remains visible with `SMART unavailable` and an unavailable reason.
- Bad: IOKit advertises SMART capability but plugin acquisition or data parsing fails. The disk remains visible with a failed state and native or parse diagnostics.

## 6. Tests Required

- ATA parser: word-swapped identity, attributes, thresholds, raw value, checksum failure.
- NVMe parser: identify, temperature, counters, and truncated log.
- Health: ATA missing status, ATA threshold crossing, NVMe warning bits, endurance warning.
- Model/JSON: external unavailable disk retained; unavailable and failed tags remain distinct.
- FFI: 64-bit Apple vtable sizes and method offsets.
- Inventory: bus/protocol classification and virtual-media exclusion logic.
- Events: subscription start/stop and initial appearance callback outside restricted sandboxes.
- Refresh: interval zero, elapsed interval, worker refresh, and stale in-flight result discard.
- TUI: available session sample, unavailable, failed, and empty inventory.
- Hardware: unsandboxed internal NVMe, external NVMe, insertion, removal, and virtual image filtering.

## 7. Wrong vs Correct

### Wrong

```rust
let mut devices = inventory.list()?;
devices.retain(|device| {
    matches!(reader.read(device), SmartState::Available { .. })
});
```

This hides external disks and couples inventory to SMART support.

### Correct

```rust
let snapshots = collect_snapshots(&inventory, &reader)?;
```

Physical inventory stays authoritative, acquisition state remains explicit, and every presentation consumes the same normalized snapshot.
