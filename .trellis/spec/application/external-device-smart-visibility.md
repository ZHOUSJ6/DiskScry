# External Device SMART Visibility

## 1. Scope / Trigger

This contract applies whenever macOS or Linux discovers an external physical disk and SMART acquisition is attempted. SMART support enriches a discovered device; it does not determine whether the device is listed.

## 2. Signatures

The shared Rust model must preserve three distinct SMART states:

```rust
enum SmartState {
    Available {
        snapshot: Box<SmartSnapshot>,
        warnings: Vec<SmartReadError>,
    },
    Unavailable { reason: SmartUnavailableReason },
    Failed { error: SmartReadError },
}

struct DeviceSnapshot {
    device: DeviceRecord,
    smart: SmartState,
    health: HealthState,
    observed_at_unix_seconds: u64,
}
```

`SmartUnavailableReason` represents a device or platform that does not expose readable SMART data. `SmartReadError` represents an attempted read that failed, including permission, execution, parsing, or partial-output failures.

`Available.warnings` preserves optional identify, threshold, or status read failures without discarding successfully parsed core SMART data.

## 3. Contracts

- Device inventory returns every discovered external physical disk, regardless of `SmartState`.
- `SmartState::Unavailable` and `SmartState::Failed` both render `SMART unavailable` in English and `SMART 不可用` in Simplified Chinese CLI and TUI views.
- A concise reason may follow the label, but it must not replace it.
- JSON output must preserve the distinction between `Unavailable` and `Failed`; it must not encode both as an absent object or `null` without a reason.
- `Unavailable` and `Failed` produce an unknown health result unless independent, documented evidence supports another state.
- A SMART reader must enrich an existing device record and must not remove a device from inventory.

## 4. Validation & Error Matrix

| Condition | `SmartState` | Device visible | User-facing state |
| --- | --- | --- | --- |
| SMART data read successfully | `Available` | Yes | SMART values and health |
| Device or transport does not expose SMART | `Unavailable` | Yes | Localized SMART-unavailable label plus reason |
| Platform cannot access SMART for the device | `Unavailable` | Yes | Localized SMART-unavailable label plus reason |
| Permission, execution, or parse failure | `Failed` | Yes | Localized SMART-unavailable label plus explicit error |
| Device inventory itself fails before a device is identified | No snapshot | Not applicable | Propagate the inventory error |

## 5. Good / Base / Bad Cases

- Good: An external disk exposes SMART. It is listed with `Available`, its values, and supported health evidence.
- Base: An external disk is detected but macOS does not expose SMART through its transport. It remains listed with `Unavailable` and the locale-specific unavailable label.
- Bad: SMART acquisition returns malformed output or a permission error. The disk remains listed with `Failed`; the error is visible and is not converted to `Unavailable` or `Good`.

## 6. Tests Required

- Inventory test: an external disk fixture without SMART support remains in the device list.
- Projection test: CLI and TUI render exact `SMART unavailable` and `SMART 不可用` labels for `Unavailable` in their respective locales.
- Failure test: a permission or parse failure produces `Failed`, retains the device, and exposes the error stage.
- Health test: `Unavailable` and `Failed` cannot produce `Good` from missing or zero values.
- Serialization test: JSON distinguishes `Available`, `Unavailable`, and `Failed` and retains the diagnostic reason.
- Cross-platform test: equivalent macOS and Linux fixtures produce the same shared-state meaning.

## 7. Wrong vs Correct

### Wrong

```rust
devices.retain(|device| device.smart.is_some());
```

This hides external disks when SMART is unsupported or unreadable.

### Correct

```rust
for device in &mut devices {
    device.smart = smart_reader.read(device);
}
```

Inventory remains authoritative; SMART acquisition records `Available`, `Unavailable`, or `Failed` on the existing device.
