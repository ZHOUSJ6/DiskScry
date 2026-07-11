# Thinking Guides

Use these guides when a change affects more than one application boundary or risks duplicating device semantics.

| Guide | Use it when |
| --- | --- |
| [Code Reuse](./code-reuse-thinking-guide.md) | Adding another parser, device field, health rule, platform branch, or presentation of existing data |
| [Cross-Layer Data Flow](./cross-layer-thinking-guide.md) | Moving data from macOS/Linux acquisition through normalization and health evaluation into CLI or TUI output |

## Pre-Modification Rule

Search before changing a field, status, command argument, or platform rule. Use `rg` for repository searches. Verify every affected consumer and test before considering a cross-layer change complete.
