# macOS developer preview execution plan

## Implementation checklist

1. Create the Cargo package and establish formatting, linting, unit-test, integration-test, and target-build commands.
2. Implement shared device identity, SMART states, health states, error stages, timestamps, and versioned JSON serialization.
3. Implement ATA IDENTIFY, SMART data, threshold, checksum, and overall-status parsing with byte fixtures.
4. Implement NVMe Identify Controller and SMART / Health log parsing with byte fixtures.
5. Implement pure conservative health evaluation with causal evidence tests.
6. Implement macOS whole-disk discovery, stable current-session identities, and Disk Arbitration hotplug events.
7. Implement target-gated IOKit ownership wrappers and ABI checks.
8. Implement read-only ATA SMART transport and map unsupported, permission, native, and parse outcomes explicitly.
9. Implement read-only NVMe SMART transport with the same result contract.
10. Implement the application store, inventory generation checks, blocking SMART workers, scheduler, interval zero, and manual refresh.
11. Implement `list`, `show`, and `watch`, including exact selector matching and versioned JSON.
12. Implement the split-pane TUI, tabs, keyboard controls, footer diagnostics, terminal restoration, and session sparkline.
13. Add contract, CLI, JSON, scheduler, hotplug, and Ratatui test-backend coverage.
14. Add a centralized English/Simplified Chinese text catalog, environment/override locale selection, localized Clap help, CLI output, TUI labels, and errors while preserving JSON.
15. Add macOS Apple Silicon and Intel CI builds and source-install validation.
16. Validate representative internal, external-unavailable, permission-denied, and hotplug hardware paths.
17. Run the quality gate and update `.trellis/spec/application/` with verified source paths and commands.
18. Add CrystalDiskInfo 9.9.1 MIT attribution and a revision-pinned generic bilingual ATA attribute catalog without adding C++ or runtime resource loading.
19. Implement a pure readable SMART projection for ATA, NVMe, unavailable, and failed states, including exact raw evidence, human units, and evidence-aligned severity.
20. Reuse the projection in human `show` output and keep JSON serialization on the existing normalized snapshot path.
21. Replace the TUI SMART JSON page with the readable summary/table, add `v` raw view toggle, and add page/home/end viewport state with selection/refresh reset.
22. Add catalog, formatting, CLI, viewport, toggle, bilingual Ratatui, diagnostic-state, and JSON-regression tests.
23. Validate readable output against real macOS NVMe devices and an external SMART-unavailable device, then rerun the complete macOS quality gate.

## Quality gate

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cargo build --release
cargo install --path . --root /tmp/diskscry-install-check
```

CI builds `aarch64-apple-darwin` and `x86_64-apple-darwin` on macOS runners.

## Risk and rollback points

- `src/presentation/smart_catalog.rs` is derived third-party metadata. Review provenance, exact upstream revision, bilingual key coverage, and attribution before merging.
- `src/presentation/smart_view.rs` must remain presentation-only. Any change to `SmartState`, `SmartSnapshot`, health evaluation, or JSON schema is a scope violation and should be reverted rather than accommodated.
- `src/presentation/tui.rs` changes keyboard and viewport state. Keep disk selection, terminal restoration, refresh scheduling, and stale-result handling covered while introducing SMART scrolling.
- `src/presentation/output.rs` changes human `show` output only. The `--json` branches in `src/cli.rs` remain direct `SnapshotEnvelope` serialization and are the rollback boundary for machine compatibility.

## Review gates

- Shared model and JSON schema reviewed before platform integration.
- Parser and health fixtures pass before live IOKit reads.
- Inventory retains unsupported external disks before SMART enrichment is connected.
- Unsafe FFI ownership and ABI reviewed before hardware validation.
- CLI/TUI semantics and terminal restoration pass before developer-preview packaging.
- The CrystalDiskInfo-derived catalog is reviewed independently from health evaluation and includes revision/license provenance.
- Readable SMART and raw JSON views are tested before live-device validation.
