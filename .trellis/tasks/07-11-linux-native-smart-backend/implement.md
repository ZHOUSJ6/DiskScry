# Linux native SMART backend execution plan

## Preconditions

- The macOS developer-preview child is complete.
- Shared device, SMART, health, error, parser, JSON, CLI, TUI, and application traits are stable.
- The parent integration fixtures pass before Linux changes begin.

## Implementation checklist

1. Add target-gated Linux platform modules without changing shared presentation contracts.
2. Implement sysfs whole-disk enumeration, topology mapping, identity, connection, and external classification.
3. Implement kernel add/remove/change events and inventory generation handling.
4. Define and ABI-test target-gated NVMe admin ioctl structures.
5. Implement Identify Controller and SMART / Health Get Log Page reads into the shared parser.
6. Define and ABI-test `SG_IO` and ATA PASS-THROUGH request structures.
7. Implement read-only ATA identify, SMART data, thresholds, and overall-status requests where supported.
8. Map unsupported transport, permission, kernel, sense, partial-response, and parse outcomes to the shared error model.
9. Add Linux discovery, request-construction, error-mapping, hotplug, CLI, JSON, and cross-platform contract tests.
10. Add Linux x86_64 CI build and source-install validation.
11. Validate representative native NVMe, ATA/SAT, unsupported USB, permission-denied, and hot-remove hardware paths.
12. Run the full cross-platform quality gate and update project specs with verified Linux paths and commands.

## Quality gate

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cargo build --release
cargo install --path . --root /tmp/diskscry-install-check
```

CI runs Linux tests and release builds on an `x86_64-unknown-linux-gnu` runner and keeps the macOS jobs green.

## Review gates

- Linux discovery parity reviewed before ioctl work.
- NVMe request ABI and error mapping reviewed before hardware reads.
- SG_IO request ABI and sense interpretation reviewed before hardware reads.
- Cross-platform JSON and UI snapshots reviewed before integration completion.
