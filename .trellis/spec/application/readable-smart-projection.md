# Readable SMART Projection

## 1. Scope / Trigger

This contract applies to changes in human SMART formatting, ATA attribute names, NVMe units, CLI `show` details, the TUI SMART tab, raw/readable view switching, or SMART viewport controls.

## 2. Signatures

The shared human projection is defined in `src/presentation/smart_view.rs`:

```rust
fn project_smart(snapshot: &DeviceSnapshot, locale: Locale) -> SmartView;
fn render_smart_text(snapshot: &DeviceSnapshot, locale: Locale) -> String;

struct SmartView {
    kind: SmartViewKind,
    summary: Vec<SmartField>,
    columns: Vec<&'static str>,
    rows: Vec<SmartRow>,
    diagnostics: Vec<String>,
}
```

The revision-pinned ATA catalog is defined in `src/presentation/smart_catalog.rs`:

```rust
const CRYSTAL_DISK_INFO_VERSION: &str = "9.9.1";
const CRYSTAL_DISK_INFO_REVISION: &str =
    "fdc8bce73ab0355c513c758ebf0f0f22662830e2";

fn ata_attribute_name(id: u8, locale: Locale) -> Option<&'static str>;
```

TUI viewport state is owned by `TuiState` and changed through `toggle_smart_view`, `smart_page_up`, `smart_page_down`, `smart_home`, and `smart_end`.

## 3. Contracts

- `DeviceSnapshot` is the only projection input. Presentation code does not reread hardware or parse platform bytes.
- `project_smart` owns readable names, units, exact-value formatting, diagnostics, and evidence-aligned row severity. It does not derive or mutate `HealthState`.
- Human `show` output and the TUI SMART tab consume the same projection. TUI layout code must not reimplement metric meaning.
- `list --json` and `show --json` serialize `SnapshotEnvelope` directly. Locale, formatted units, and CrystalDiskInfo names never enter JSON.
- NVMe Data Units use 512,000 bytes for approximate human formatting; the original decimal Data Units counter remains visible.
- Every readable metric appends a two-digit hexadecimal code in parentheses. NVMe uses the CrystalDiskInfo-style `01` through `0F` display sequence; ATA uses the device-reported attribute ID.
- ATA rows retain the ID in the metric label, current, worst, threshold, reliable common interpretation, and the original six bytes in high-to-low fixed-width hexadecimal.
- Unknown or vendor-specific ATA IDs render as `Unknown attribute (XX)` or `未知属性 (XX)`. They are not assigned a generic meaning.
- The first catalog contains only CrystalDiskInfo's generic `[Smart]` English and Simplified Chinese tables from the pinned revision. Vendor sections require a future vendor-classification contract and hardware fixtures.
- CrystalDiskInfo attribution and the MIT text live in `THIRD_PARTY_LICENSES.md`. Updates are reviewed source changes; the executable performs no catalog downloads.
- The SMART tab defaults to readable mode. `v` toggles raw JSON, `PageUp`/`PageDown` move one visible page, and `Home`/`End` move to valid bounds. Device selection and snapshot replacement reset the viewport.
- Wide terminals use bounded SMART column widths so metric names, readable values, and raw values stay visually grouped. Narrow terminals retain flexible metric columns to preserve usable space.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Available NVMe | Localized summary and standard metric/value/raw table |
| Available ATA with known generic ID | Localized catalog name and exact normalized/raw columns |
| Available ATA with unknown ID | Explicit unknown label containing `XX` |
| ATA normalized value crosses threshold | Warning row style; existing health evidence remains authoritative |
| NVMe critical-warning bits nonzero | Critical row style matching health evidence |
| NVMe spare below threshold or used percentage at least 100 | Warning row style matching health evidence |
| SMART unavailable | Localized primary unavailable label and localized reason |
| SMART failed | Localized error heading, raw operation/message, and permission action when applicable |
| Decimal counter cannot be formatted | Visible `N/A`/`不可用`; original field remains visible |
| SMART content shorter than a page | Offset clamps to zero |
| Selection or refreshed snapshot changes | SMART offset resets to zero |

## 5. Good / Base / Bad Cases

- Good: An NVMe disk shows `62.9 TB` beside exact Data Units `122809997`, and health severity follows standardized evidence.
- Base: An ATA attribute is not in the pinned generic table. The row still exposes ID, normalized values, threshold, and raw bytes with an explicit unknown name.
- Bad: TUI code assigns a vendor meaning from an ID alone or serializes translated metric names into JSON.

## 6. Tests Required

- Catalog: assert pinned version/revision, bilingual known ID, and unknown ID behavior.
- Projection: assert ATA and NVMe metric codes, ATA names, unknown labels, raw byte order, NVMe units, exact counters, and common duration formatting.
- Severity: assert ATA threshold, NVMe critical warning, spare, and endurance rows match domain evidence boundaries.
- Diagnostics: assert unavailable reason, failed operation/message, and permission action remain visible.
- CLI: assert human `show` contains the readable projection and JSON remains free of localized/catalog text.
- TUI: assert readable default, `v` raw JSON toggle, bilingual metrics, bounded wide-terminal column spacing, page movement, end clamping, and selection/refresh reset.
- Hardware: validate internal NVMe, external NVMe, and a visible SMART-unavailable external device outside restricted sandboxes.

## 7. Wrong vs Correct

### Wrong

```rust
let name = match attribute.id {
    0xAA => "Available Reserved Space",
    _ => "Unknown",
};
```

An ID can be vendor-specific, so this invents meaning without vendor evidence.

### Correct

```rust
let name = ata_attribute_name(attribute.id, locale)
    .unwrap_or(messages.unknown_attribute);
let metric = format!("{name} ({:02X})", attribute.id);
```

The pinned generic catalog supplies known names, while unowned meanings remain explicit and raw evidence stays available.
