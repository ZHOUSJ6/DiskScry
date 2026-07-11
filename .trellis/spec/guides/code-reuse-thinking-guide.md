# Code Reuse Thinking Guide

## Search Before Adding

Before creating a new type, parser, health rule, command runner, or formatter, search for the concept and the source field:

```bash
rg -n "DeviceSnapshot|HealthStatus|field_name|command_name" .
```

No product source exists yet, so there are no reusable implementation examples during bootstrap. Once code is introduced, update this guide with the actual owning files and symbols.

## Single Ownership Targets

Avoid parallel definitions of the same disk semantics:

- One shared representation for normalized device identity and capabilities.
- One owner for each raw-to-normalized field conversion.
- One health evaluator for derived status.
- One platform adapter per supported operating system.
- One normalized snapshot consumed by both CLI and TUI projections.

## Review Triggers

Stop and search the full data flow when:

- A SMART or device field is added to more than one output.
- macOS and Linux branches compute the same normalized value separately.
- CLI and TUI code format or classify the same field differently.
- A constant, threshold, status, or unsupported condition appears in multiple modules.
- A new parser reads raw platform data outside the acquisition boundary.

Share stable domain meaning, not incidental syntax. Two platform adapters may need different code even when they produce the same shared type.
