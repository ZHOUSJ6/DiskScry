# macOS Release Contract

## 1. Scope / Trigger

This contract applies when publishing a DiskScry GitHub release or changing its package version, release assets, installation instructions, or macOS target matrix.

## 2. Signatures

The release tag matches `Cargo.toml` exactly with a `v` prefix:

```text
Cargo package version: 0.1.0
Git tag:              v0.1.0
```

Each release builds the locked source for both supported targets:

```bash
cargo build --release --target aarch64-apple-darwin --locked
cargo build --release --target x86_64-apple-darwin --locked
```

Release assets use these names:

```text
diskscry-v<VERSION>-aarch64-apple-darwin.tar.gz
diskscry-v<VERSION>-x86_64-apple-darwin.tar.gz
SHA256SUMS
```

## 3. Contracts

- The release tag points to the exact commit used to build every asset.
- Both archives contain `diskscry`, `README.md`, and `THIRD_PARTY_LICENSES.md` at their root.
- `SHA256SUMS` contains one SHA-256 entry for each archive.
- Developer-preview binaries are unsigned and not notarized; the README and release notes disclose this.
- Source installation uses the same release tag and `--locked` dependency resolution.
- A release is not published from an uncommitted worktree or after a failed quality-gate command.
- `.github/workflows/release.yml` publishes with the repository-scoped `GITHUB_TOKEN` and `contents: write`; no personal token is stored in the repository.
- Tag pushes publish their matching tag. A `main` push that changes `Cargo.toml` or the release workflow resolves the current Cargo version and publishes its existing tag. Existing releases are left unchanged.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Cargo version and tag match | Continue release |
| Either macOS target fails to build | Do not publish |
| Formatting, Clippy, tests, hardware tests, or locked install fails | Do not publish |
| Archive executable reports the wrong version | Do not upload that archive |
| Archive checksum differs from `SHA256SUMS` | Rebuild the archive and checksum |
| Release tag already exists at another commit | Stop and resolve the version conflict |

## 5. Good / Base / Bad Cases

- Good: `v0.1.0` points to the validated commit and publishes both target archives plus matching checksums.
- Base: Users install directly from the tagged source with Cargo instead of downloading an unsigned binary.
- Bad: A binary built before the release commit is uploaded under the current tag.

## 6. Tests Required

- Run the complete quality gate in `engineering-baseline.md`.
- Build both declared macOS targets with `--locked`.
- Execute the native binary with `--version` and inspect the Intel binary architecture with `file`.
- List both archives and assert the required root files are present.
- Verify `SHA256SUMS` with `shasum -a 256 -c` before upload.

## 7. Wrong vs Correct

### Wrong

```bash
cargo build --release
gh release create v0.1.0 target/release/diskscry
```

This publishes only the host architecture and provides no reproducible archive or checksum.

### Correct

```bash
cargo build --release --target aarch64-apple-darwin --locked
cargo build --release --target x86_64-apple-darwin --locked
```

Package both target binaries under the versioned names above, generate `SHA256SUMS`, verify the archives, and upload all three assets to the matching tag.
