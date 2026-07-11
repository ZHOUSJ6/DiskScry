# Application Development Guidelines

DiskScry is a new Rust CLI/TUI project for reading and monitoring disk information, with SMART data as its primary domain.

## Guidelines Index

| Guide | Use it for |
| --- | --- |
| [Project Scope](./project-scope.md) | Product boundaries, supported platforms, and decisions that are not yet settled |
| [Engineering Baseline](./engineering-baseline.md) | Evidence rules, platform boundaries, failure behavior, and verification expectations |
| [External Device SMART Visibility](./external-device-smart-visibility.md) | Required visibility and error behavior when an external disk has no readable SMART data |

## Pre-Development Checklist

Before planning or implementing product code:

1. Read [Project Scope](./project-scope.md).
2. Read [Engineering Baseline](./engineering-baseline.md).
3. Read [External Device SMART Visibility](./external-device-smart-visibility.md) for device enumeration, SMART acquisition, status, serialization, CLI, or TUI changes.
4. Read the shared [Thinking Guides](../guides/index.md) when a change crosses device acquisition, normalization, health evaluation, CLI, or TUI boundaries.
5. Inspect the current `Cargo.toml`, `src/`, and tests before introducing a directory, dependency, error, or test convention. These paths do not exist yet, so the first implementation task must establish and document them.

## Evidence Status

As of 2026-07-11, the repository contains Trellis workflow files and `AGENTS.md`, but no product manifest, Rust source, fixtures, or tests. These guidelines record confirmed project scope and repository-wide constraints only. They intentionally do not claim that an implementation pattern already exists.
