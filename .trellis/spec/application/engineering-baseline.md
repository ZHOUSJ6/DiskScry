# Engineering Baseline

## Evidence Before Convention

Repository conventions must come from merged source, tests, configuration, or an approved project decision. Search the repository before adding a new abstraction or rule. When no local pattern exists, record the decision in the active task instead of presenting it as established practice.

## Platform Boundaries

macOS and Linux expose storage devices through different commands and system interfaces. Platform-specific acquisition must be isolated from the shared device model, health evaluation, CLI formatting, and TUI state.

The shared layers must not:

- Execute platform commands directly.
- Infer platform support from an empty or zero value.
- Reinterpret an acquisition failure as a healthy device.
- Add a Windows branch while Windows remains outside project scope.

## Failure Behavior

Failures must remain explicit and diagnosable:

- Propagate command, permission, parse, unsupported-device, and partial-data failures with their stage and device identity.
- Do not return mock data, templated success, or a healthy status when acquisition fails.
- Do not silently switch acquisition mechanisms after an error.
- Keep device-reported health distinct from application-derived health when both are available.
- Preserve unknown vendor fields until a documented normalization rule owns them.
- Keep successful device enumeration separate from SMART availability so an unreadable external disk remains visible under the [external-device contract](./external-device-smart-visibility.md).

## Data Flow Ownership

Each boundary has one owner:

```text
platform source -> acquisition result -> normalized snapshot -> health result -> CLI/TUI projection
```

- Platform acquisition owns system calls, command execution, and raw output capture.
- Normalization owns conversion into shared Rust types.
- Health evaluation owns derived status and its evidence.
- CLI and TUI own presentation only; they must consume the same normalized snapshot rather than parse raw platform data independently.

## Verification

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --doc --workspace
cargo build --release
```

macOS release validation also builds `x86_64-apple-darwin` and runs the ignored Disk Arbitration tests outside restricted sandboxes:

```bash
cargo build --release --target x86_64-apple-darwin
cargo test platform::macos::events::tests -- --ignored
cargo install --path . --root /tmp/diskscry-install-check --locked --force
```

Parser, health, JSON, CLI, scheduler, stale-refresh, and Ratatui rendering tests live beside their owning modules. macOS hardware-dependent tests are ignored in the default suite because restricted process sandboxes can deny Disk Arbitration and IOKit user clients.
