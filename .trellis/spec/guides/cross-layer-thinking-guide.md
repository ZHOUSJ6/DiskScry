# Cross-Layer Data Flow Guide

## Canonical Flow

Disk information crosses these boundaries:

```text
macOS/Linux source
    -> acquisition result
    -> normalized device snapshot
    -> device-reported and derived health
    -> CLI or TUI projection
```

Map every changed field through the complete flow before implementation.

## Boundary Questions

For each field or status, answer:

- Which platform source provides it?
- Can it be absent, unsupported, stale, or only partially available?
- Which layer converts its units and meaning?
- Which evidence supports a derived health classification?
- Do CLI and TUI consume the same normalized value?
- What explicit error reaches the user when acquisition or parsing fails?

## Required Checks

Before implementation:

- Define the raw input and normalized output contracts.
- Separate platform-specific availability from device health.
- Resolve any product decision required by the change in the task PRD or design.
- Identify every CLI, TUI, serialization, and test consumer.

After implementation:

- Test success, missing data, unsupported devices, permission failures, malformed input, and partial results.
- Confirm macOS and Linux produce the same normalized meaning where both support a field.
- Confirm acquisition failures cannot become `Good`, zero, or an empty success result.
- Confirm CLI and TUI do not parse raw platform output independently.

## External-Disk Visibility Check

For every external-disk change, verify the detailed [External Device SMART Visibility](../application/external-device-smart-visibility.md) contract. Device inventory must remain authoritative when SMART is unsupported or acquisition fails.
