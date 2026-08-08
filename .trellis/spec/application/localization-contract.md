# Localization Contract

## 1. Scope / Trigger

This contract applies to changes in locale detection, CLI help, human-readable CLI output, TUI text, actionable error guidance, JSON projection, or domain labels.

## 2. Signatures

The presentation locale boundary is defined in `src/presentation/locale.rs`:

```rust
enum Locale {
    En,
    ZhCn,
}

impl Locale {
    fn detect(args: &[OsString]) -> Self;
    fn detect_from(args: &[OsString], lc_all: Option<&OsStr>, lang: Option<&OsStr>) -> Self;
    fn from_tag(value: &str) -> Option<Self>;
    fn messages(self) -> &'static Messages;
}
```

Human-readable projections accept an explicit locale:

```rust
fn render_list(devices: &[DeviceSnapshot], locale: Locale) -> String;
fn render_detail(snapshot: &DeviceSnapshot, locale: Locale) -> String;
fn smart_label(snapshot: &DeviceSnapshot, locale: Locale) -> &'static str;
```

The command-line override is global and limited to these stable values:

```text
--lang <en|zh-CN>
```

## 3. Contracts

- Supported human interface languages are English and Simplified Chinese.
- Locale priority is `--lang`, then `LC_ALL` when present, then `LANG`, then English. An unsupported selected environment locale resolves to English and does not fall through to a lower-priority variable.
- Locale tags beginning with `zh`, `zh_`, or `zh-` select Simplified Chinese. English tags beginning with `en`, `en_`, or `en-` select English.
- CLI command names, positional arguments, option names, device identifiers, protocol names, and connection enum names stay in English.
- `src/presentation/locale.rs` owns general human-interface text. The only additional translatable catalog is the revision-pinned CrystalDiskInfo-derived ATA metadata in `src/presentation/smart_catalog.rs`. Domain, protocol, platform, and application modules do not translate strings.
- The English human label and canonical internal label for missing or failed SMART data is `SMART unavailable`. Simplified Chinese human output uses `SMART 不可用`.
- JSON serialization never receives a locale. Schema keys and enum values such as `unavailable`, `failed`, and `smart_unavailable` remain byte-stable across interface languages.
- Low-level operation names, native errors, and diagnostic payloads remain verbatim. Human-facing headings, status labels, and corrective actions are localized around them.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| `--lang zh-CN` with an English environment | Simplified Chinese human interface |
| `--lang en` with a Chinese environment | English human interface |
| `LC_ALL=zh_CN.UTF-8` | Simplified Chinese human interface |
| `LC_ALL=C` and `LANG=zh_CN.UTF-8` | English human interface |
| `LC_ALL` unset and `LANG=zh_CN.UTF-8` | Simplified Chinese human interface |
| Unsupported `--lang` value | Clap validation error; no silent substitution |
| Unsupported selected environment locale | English human interface |
| SMART interface unavailable in Chinese CLI/TUI | Disk retained and shown as `SMART 不可用` |
| `list --json` under either language | Identical schema vocabulary and enum values |

## 5. Good / Base / Bad Cases

- Good: `diskscry --lang zh-CN list` translates headings, health, and SMART availability while keeping `/dev/disk4` and protocol data unchanged.
- Base: No locale override or supported environment locale is present. The English interface is used.
- Bad: A translated domain enum or JSON serializer emits Chinese JSON values. This breaks automation and the versioned schema contract.

## 6. Tests Required

- Locale unit tests assert explicit override, `LC_ALL` precedence, `LANG` selection, and English fallback.
- CLI tests assert Chinese root and subcommand help while command and option spellings remain English.
- Output tests assert exact English `SMART unavailable` and Simplified Chinese `SMART 不可用` labels.
- TUI rendering tests assert Chinese table headings, status labels, and key footer text.
- JSON tests assert stable English keys and enum values independent of the selected human interface locale.
- Manual macOS validation runs Chinese `list`, Chinese help, and an actionable error against real device inventory.

## 7. Wrong vs Correct

### Wrong

```rust
#[serde(rename = "SMART 不可用")]
Unavailable,
```

This couples the machine-readable schema to one human language.

### Correct

```rust
match snapshot.smart {
    SmartState::Available { .. } => locale.messages().smart_available,
    SmartState::Unavailable { .. } | SmartState::Failed { .. } => {
        locale.messages().smart_unavailable
    }
}
```

Serialization keeps the stable domain state while the presentation layer selects a localized label.
