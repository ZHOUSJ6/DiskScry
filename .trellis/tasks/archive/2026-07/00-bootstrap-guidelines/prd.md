# Bootstrap Project Development Guidelines

## Goal

Replace the generated fullstack spec scaffolding with a minimal, project-specific Trellis spec set that reflects DiskScry's current repository and confirmed product scope.

## Confirmed Facts

- DiskScry is a new Rust CLI/TUI application focused on physical-disk information and SMART monitoring.
- CrystalDiskInfo is a data and behavior reference.
- macOS is the primary target; Linux is supported after the macOS path is established.
- Windows is outside the current scope.
- All discovered external physical disks remain visible; devices without readable SMART data display `SMART unavailable`.
- The repository has no product manifest, source files, fixtures, tests, or established Rust patterns.
- The generated backend/frontend spec files are untouched templates for a fullstack project and do not describe DiskScry.

## Scope

- Remove non-applicable backend/frontend template specs.
- Create a single-repository `application` spec layer for confirmed scope and engineering constraints.
- Replace inherited Trellis implementation examples in the shared thinking guides with DiskScry-specific boundary guidance.
- Keep unsettled product and backend decisions explicit instead of inventing conventions.
- Do not create product source code.

## Acceptance Criteria

- [x] `.trellis/spec/application/index.md` is the application spec entry point and contains a pre-development checklist.
- [x] Application specs record the current source-free repository state and the confirmed macOS/Linux/Rust scope.
- [x] External-disk visibility and `SMART unavailable` behavior are captured as an executable cross-layer contract.
- [x] Non-applicable backend/frontend template files are removed.
- [x] Shared guides describe DiskScry's device-data flow without Trellis-template implementation details.
- [x] No placeholder text, empty headings, hypothetical source paths, or copied fullstack guidance remains.
- [x] Index files match the final spec file set.

## Evidence Limits

No product code examples can be included because no product code exists. The first implementation task must establish real Rust source and test patterns, then refresh these specs with concrete file paths, symbols, and validation commands.
