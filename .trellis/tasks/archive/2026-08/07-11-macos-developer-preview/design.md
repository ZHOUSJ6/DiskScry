# macOS developer preview design

## Proposed source layout

```text
src/
├── main.rs
├── cli.rs
├── app.rs
├── domain/
│   ├── device.rs
│   ├── health.rs
│   └── smart.rs
├── protocol/
│   ├── ata.rs
│   └── nvme.rs
├── platform/
│   ├── mod.rs
│   └── macos/
│       ├── inventory.rs
│       ├── iokit.rs
│       ├── smart_ata.rs
│       └── smart_nvme.rs
└── presentation/
    ├── locale.rs
    ├── output.rs
    ├── smart_catalog.rs
    ├── smart_view.rs
    └── tui.rs
tests/
├── fixtures/
├── contract.rs
├── cli.rs
└── tui.rs
```

The exact file split may be collapsed while modules are small, but dependency direction remains `platform -> protocol/domain -> application -> presentation` with presentation consuming application snapshots rather than platform values.

## macOS discovery

Disk Arbitration callbacks identify whole disks appearing and disappearing. Each event is enriched from the IOKit registry with device node, model, serial when exposed, capacity, protocol, removable/ejectable/external properties, and registry identity.

Volumes and partitions are not separate monitored devices. The inventory maps them to their whole physical parent and emits one `DeviceSnapshot` per disk.

## SMART interface acquisition

The IOKit service ancestry is inspected for a public ATA or NVMe SMART-capable service. The platform module acquires the corresponding plugin interface, calls only read methods, copies data into owned Rust buffers, and releases every Apple object in reverse acquisition order.

Minimal FFI definitions are target-gated and ABI-tested for structure sizes and offsets. Safe wrappers accept fixed-size mutable buffers and return typed `Result` values. No safe wrapper exists for write logs, enable/disable operations, self-tests, or other mutating methods.

An identified disk with no supported interface returns `Unavailable`. Failure after selecting and attempting a supported read returns `Failed` with device, stage, operation, and native error code.

## Application execution

The terminal thread owns UI state. Inventory callbacks and SMART worker threads send typed messages through bounded channels. Each request contains `DeviceId` and inventory generation. The reducer discards results whose generation no longer matches the current device.

CLI `list` and `show` perform a finite inventory/read cycle. `watch` and TUI retain the event loop and scheduler. Ctrl-C, `q`, errors, and panic-aware terminal cleanup restore terminal mode before returning control to the shell.

## Dependency roles

- `clap`: optional subcommand parsing and help.
- `serde` and `serde_json`: stable tagged JSON projections.
- `ratatui` with Crossterm: rendering and keyboard events.
- `objc2-io-kit` and Core Foundation bindings: supported Apple framework types and ownership helpers.
- Minimal target-gated FFI: public SMART interfaces absent from high-level bindings.

Protocol parsing and health evaluation remain independent of those dependencies.

## Localization

One presentation-layer locale module owns every translatable human-facing label. Locale selection uses the explicit `--lang` value first, then `LC_ALL`, then `LANG`, with English as the final default. Values beginning with `zh` select Simplified Chinese for the developer preview.

Command names, option names, device identifiers, JSON keys, JSON enum values, protocol names, and raw diagnostic data remain language independent. CLI output and TUI widgets receive the selected locale explicitly; domain and platform modules do not translate strings.

## Readable SMART projection

`src/presentation/smart_view.rs` projects a borrowed `DeviceSnapshot` into presentation-only summary fields and protocol-specific rows. The projection owns names, units, approximate human formatting, exact raw formatting, and row severity. It does not mutate normalized snapshots, serialize JSON, read devices, or derive a new health state.

```rust
struct SmartView {
    summary: Vec<SmartField>,
    body: SmartViewBody,
}

enum SmartViewBody {
    Ata(Vec<AtaSmartRow>),
    Nvme(Vec<NvmeSmartRow>),
    Unavailable(String),
    Failed(SmartFailureView),
}

fn project_smart(snapshot: &DeviceSnapshot, locale: Locale) -> SmartView;
fn render_smart_text(snapshot: &DeviceSnapshot, locale: Locale) -> String;
```

The TUI and human `show` output consume this projection. `show --json` continues to serialize `SnapshotEnvelope` directly and therefore cannot receive localized names or formatted values.

ATA rows contain ID, localized generic name, current, worst, threshold, a reliable interpreted value when one is defined, and the original six-byte value in fixed-width hexadecimal. A threshold crossing is warning-styled. Unknown or vendor-specific IDs are labeled as unknown with the hexadecimal ID and are never assigned a guessed meaning.

NVMe rows contain a localized metric name with a CrystalDiskInfo-style two-digit display code from `01` through `0F`, a human value with units, and the original decimal counter or exact integer. ATA metric names append the device-reported hexadecimal attribute ID. Data Units are approximately formatted using the NVMe 512,000-byte unit while retaining the exact Data Units counter. Critical-warning, spare-below-threshold, and endurance-used rows reuse the same evidence boundaries as `domain::health` for severity.

## CrystalDiskInfo-derived catalog

`src/presentation/smart_catalog.rs` contains the reviewed bilingual generic `[Smart]` ATA name snapshot derived from CrystalDiskInfo 9.9.1 commit `fdc8bce73ab0355c513c758ebf0f0f22662830e2`. It contains no Windows transport, MFC, UI, health, vendor detection, or runtime loading code.

The source file records upstream file paths and revision. `THIRD_PARTY_LICENSES.md` retains the CrystalDiskInfo copyright and MIT license. Catalog updates are manual source changes with review and tests; the executable performs no network access. Vendor-specific sections remain excluded until DiskScry has an explicit vendor-classification contract and representative fixtures.

## SMART TUI behavior

The SMART tab defaults to the readable view. Pressing `v` toggles the selected snapshot between readable and raw JSON views. The raw view is diagnostic presentation only and does not change serialization.

`TuiState` stores a SMART viewport offset, the last rendered page length, and the raw/readable toggle. `PageUp` and `PageDown` move by one visible page; `Home` and `End` move to the first and last valid offset. Disk selection and snapshot replacement reset the offset to zero. Rendering clamps offsets to the current row or line count.

Ratatui `TableState` is stored in application state and its `offset_mut()` controls the first visible SMART row. Row severity maps to neutral, yellow warning, or red critical styling. The raw JSON `Paragraph` uses the same logical viewport controls.

The disk list remains visible. The SMART tab gives the detail pane more width than the overview/session tabs while preserving the selected disk and existing `j`/`k` and arrow-key behavior.

Wide terminals use bounded table columns instead of assigning all remaining width to the metric or attribute column. Narrow layouts keep a flexible name column so values remain visible without changing the underlying projection.

## Test seams

Inventory and SMART transports implement traits consumed by the application service. Unit tests supply deterministic trait implementations that return explicit results; production builds never select them as runtime backends.

Parser fixtures test raw bytes. Application contract tests test state transitions. Ratatui `TestBackend` tests rendering. macOS integration tests are separately marked because they require real IOKit services and sometimes elevated privileges.

Readable SMART tests use deterministic ATA and NVMe snapshots. Assertions cover bilingual names, exact raw values, human units, evidence-aligned severity, unavailable/failed diagnostics, raw toggle, viewport reset/clamping, CLI/TUI shared projection, and unchanged JSON.
