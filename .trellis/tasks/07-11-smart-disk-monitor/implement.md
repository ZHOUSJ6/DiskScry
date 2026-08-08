# SMART disk monitor execution plan

## Task order

1. Complete and activate `.trellis/tasks/07-11-macos-developer-preview`.
2. Establish the shared data, error, parser, health, JSON, CLI, and TUI contracts in the macOS child.
3. Validate macOS native discovery and SMART reads against internal NVMe, supported ATA when available, an external disk without readable SMART, and a permission-denied path.
4. Complete the macOS quality gate and update project specs with real source paths and validation commands.
5. Complete and activate `.trellis/tasks/07-11-linux-native-smart-backend` after the shared contracts are stable.
6. Add Linux discovery, events, NVMe ioctl, and SG_IO transports without changing shared availability semantics.
7. Run cross-platform fixture, JSON, CLI, and CI validation.
8. Perform the parent integration review, archive both children, then archive this parent task.

## Shared validation gate

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cargo build --release
```

CI additionally builds the declared macOS and Linux targets on native runners.

## Integration checks

- Every discovered external physical disk survives SMART enrichment.
- `Unavailable` and `Failed` both render `SMART unavailable` while remaining distinct in JSON.
- ATA and NVMe health results expose their causal evidence.
- CLI and TUI consume the same normalized snapshots.
- Hotplug and late SMART results cannot attach data to a removed or replaced disk.
- Scheduled and manual refresh do not block terminal input.
- No public safe API exposes a write-capable storage operation.
- The executable runs without `smartctl` or smartmontools installed.

## Rollback points

- Shared model and JSON contract before platform FFI integration.
- macOS inventory before SMART plugin integration.
- macOS developer preview before Linux implementation.
- Linux discovery before ioctl transport integration.

At each point, a failing platform transport remains visible through explicit `Unavailable` or `Failed` state; it is not replaced with fixture data or another acquisition backend.
