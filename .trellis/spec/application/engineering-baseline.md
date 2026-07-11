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

There are no product validation commands yet. The first Rust implementation task must define the repository's formatting, linting, test, and fixture commands in its implementation plan. After those commands and representative source files exist, replace this bootstrap-level section with exact commands and file-backed examples.
