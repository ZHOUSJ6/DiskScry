use crate::{
    domain::{
        AtaSmartAttribute, AtaSmartSnapshot, DecimalCounter, DeviceIdentityData, DeviceSnapshot,
        HealthState, NvmeSmartSnapshot, SmartSnapshot, SmartState, SmartUnavailableReason,
    },
    presentation::{
        locale::{Locale, Messages},
        smart_catalog::ata_attribute_name,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Normal,
    Warning,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmartField {
    pub label: &'static str,
    pub value: String,
    pub severity: Severity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmartRow {
    pub cells: Vec<String>,
    pub severity: Severity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmartViewKind {
    Ata,
    Nvme,
    Diagnostic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SmartView {
    pub kind: SmartViewKind,
    pub summary: Vec<SmartField>,
    pub columns: Vec<&'static str>,
    pub rows: Vec<SmartRow>,
    pub diagnostics: Vec<String>,
}

pub fn project_smart(snapshot: &DeviceSnapshot, locale: Locale) -> SmartView {
    let messages = locale.messages();
    let mut view = match &snapshot.smart {
        SmartState::Available {
            snapshot: smart,
            warnings,
        } => {
            let mut view = match smart.as_ref() {
                SmartSnapshot::Ata { data } => project_ata(snapshot, data, locale),
                SmartSnapshot::Nvme { data } => project_nvme(snapshot, data, locale),
            };
            view.diagnostics.extend(warnings.iter().map(|warning| {
                format!(
                    "{}: {}: {}",
                    messages.warning, warning.operation, warning.message
                )
            }));
            view
        }
        SmartState::Unavailable { reason } => SmartView {
            kind: SmartViewKind::Diagnostic,
            summary: common_summary(snapshot, None, None, locale),
            columns: Vec::new(),
            rows: Vec::new(),
            diagnostics: vec![format_unavailable_reason(reason, messages)],
        },
        SmartState::Failed { error } => {
            let mut diagnostics = vec![format!(
                "{}: {:?}: {}: {}",
                messages.error, error.stage, error.operation, error.message
            )];
            if error.permission_denied {
                diagnostics.push(format!("{}: {}", messages.action, messages.elevate_action));
            }
            SmartView {
                kind: SmartViewKind::Diagnostic,
                summary: common_summary(snapshot, None, None, locale),
                columns: Vec::new(),
                rows: Vec::new(),
                diagnostics,
            }
        }
    };

    if view.diagnostics.is_empty() && view.rows.is_empty() {
        view.diagnostics.push(messages.not_available.into());
    }
    view
}

pub fn render_smart_text(snapshot: &DeviceSnapshot, locale: Locale) -> String {
    let messages = locale.messages();
    let view = project_smart(snapshot, locale);
    let mut output = format!("{}:\n", messages.smart_details);
    for field in &view.summary {
        output.push_str(&format!("{}: {}\n", field.label, field.value));
    }
    for diagnostic in &view.diagnostics {
        output.push_str(diagnostic);
        output.push('\n');
    }
    if !view.columns.is_empty() {
        output.push('\n');
        output.push_str(&view.columns.join("\t"));
        output.push('\n');
        for row in view.rows {
            output.push_str(&row.cells.join("\t"));
            output.push('\n');
        }
    }
    output
}

fn project_ata(snapshot: &DeviceSnapshot, data: &AtaSmartSnapshot, locale: Locale) -> SmartView {
    let messages = locale.messages();
    let temperature = ata_temperature(data);
    let mut summary = common_summary(snapshot, Some(&data.identity), temperature, locale);
    let (overall, severity) = match data.overall_passed {
        Some(true) => (messages.passed, Severity::Normal),
        Some(false) => (messages.failed, Severity::Critical),
        None => (messages.not_reported, Severity::Normal),
    };
    summary.push(field(messages.ata_overall_status, overall, severity));

    SmartView {
        kind: SmartViewKind::Ata,
        summary,
        columns: vec![
            messages.attribute,
            messages.current,
            messages.worst,
            messages.threshold,
            messages.interpreted,
            messages.raw_value,
        ],
        rows: data
            .attributes
            .iter()
            .map(|attribute| project_ata_attribute(attribute, locale))
            .collect(),
        diagnostics: Vec::new(),
    }
}

fn project_ata_attribute(attribute: &AtaSmartAttribute, locale: Locale) -> SmartRow {
    let messages = locale.messages();
    let name = ata_attribute_name(attribute.id, locale).unwrap_or(messages.unknown_attribute);
    SmartRow {
        cells: vec![
            coded_metric(name, attribute.id),
            attribute.current.to_string(),
            attribute.worst.to_string(),
            attribute
                .threshold
                .map_or_else(|| "—".into(), |value| value.to_string()),
            interpret_ata(attribute, messages),
            format_ata_raw(attribute.raw_bytes),
        ],
        severity: if attribute.threshold_crossed() {
            Severity::Warning
        } else {
            Severity::Normal
        },
    }
}

fn project_nvme(snapshot: &DeviceSnapshot, data: &NvmeSmartSnapshot, locale: Locale) -> SmartView {
    let messages = locale.messages();
    let summary = common_summary(
        snapshot,
        Some(&data.identity),
        data.temperature_celsius,
        locale,
    );
    let rows = vec![
        nvme_row(
            0x01,
            messages.critical_warning,
            format!("0x{:02X}", data.critical_warning),
            data.critical_warning.to_string(),
            if data.critical_warning == 0 {
                Severity::Normal
            } else {
                Severity::Critical
            },
        ),
        nvme_row(
            0x02,
            messages.temperature,
            data.temperature_celsius.map_or_else(
                || messages.not_available.into(),
                |value| format!("{value} °C"),
            ),
            data.temperature_celsius
                .map_or_else(|| "—".into(), |value| value.to_string()),
            Severity::Normal,
        ),
        nvme_row(
            0x03,
            messages.available_spare,
            format!("{}%", data.available_spare_percent),
            data.available_spare_percent.to_string(),
            if data.available_spare_percent < data.available_spare_threshold_percent {
                Severity::Warning
            } else {
                Severity::Normal
            },
        ),
        nvme_row(
            0x04,
            messages.available_spare_threshold,
            format!("{}%", data.available_spare_threshold_percent),
            data.available_spare_threshold_percent.to_string(),
            Severity::Normal,
        ),
        nvme_row(
            0x05,
            messages.percentage_used,
            format!("{}%", data.percentage_used),
            data.percentage_used.to_string(),
            if data.percentage_used >= 100 {
                Severity::Warning
            } else {
                Severity::Normal
            },
        ),
        decimal_row(
            0x06,
            messages.data_units_read,
            &data.data_units_read,
            |value| format_data_units(value, messages),
        ),
        decimal_row(
            0x07,
            messages.data_units_written,
            &data.data_units_written,
            |value| format_data_units(value, messages),
        ),
        decimal_row(
            0x08,
            messages.host_read_commands,
            &data.host_read_commands,
            |value| format_decimal_count(value, messages),
        ),
        decimal_row(
            0x09,
            messages.host_write_commands,
            &data.host_write_commands,
            |value| format_decimal_count(value, messages),
        ),
        decimal_row(
            0x0A,
            messages.controller_busy_time,
            &data.controller_busy_minutes,
            |value| format_minutes(value, messages),
        ),
        decimal_row(0x0B, messages.power_cycles, &data.power_cycles, |value| {
            format_decimal_count(value, messages)
        }),
        decimal_row(
            0x0C,
            messages.power_on_hours,
            &data.power_on_hours,
            |value| format_hours(value, messages),
        ),
        decimal_row(
            0x0D,
            messages.unsafe_shutdowns,
            &data.unsafe_shutdowns,
            |value| format_decimal_count(value, messages),
        ),
        decimal_row(0x0E, messages.media_errors, &data.media_errors, |value| {
            format_decimal_count(value, messages)
        }),
        decimal_row(
            0x0F,
            messages.error_log_entries,
            &data.error_log_entries,
            |value| format_decimal_count(value, messages),
        ),
    ];

    SmartView {
        kind: SmartViewKind::Nvme,
        summary,
        columns: vec![messages.metric, messages.value, messages.raw_value],
        rows,
        diagnostics: Vec::new(),
    }
}

fn common_summary(
    snapshot: &DeviceSnapshot,
    smart_identity: Option<&DeviceIdentityData>,
    temperature: Option<i32>,
    locale: Locale,
) -> Vec<SmartField> {
    let messages = locale.messages();
    let model = smart_identity
        .and_then(|identity| identity.model.as_deref())
        .or(snapshot.device.identity.model.as_deref())
        .unwrap_or(messages.unknown);
    let firmware = smart_identity
        .and_then(|identity| identity.firmware.as_deref())
        .or(snapshot.device.identity.firmware.as_deref())
        .unwrap_or(messages.not_available);
    let serial = smart_identity
        .and_then(|identity| identity.serial.as_deref())
        .or(snapshot.device.identity.serial.as_deref())
        .unwrap_or(messages.not_available);
    vec![
        field(
            messages.health,
            locale.health_label(&snapshot.health),
            health_severity(&snapshot.health),
        ),
        field(
            messages.smart,
            locale.smart_label(&snapshot.smart),
            health_severity(&snapshot.health),
        ),
        field(
            messages.temperature,
            temperature.map_or_else(
                || messages.not_available.into(),
                |value| format!("{value} °C"),
            ),
            Severity::Normal,
        ),
        field(messages.model, model, Severity::Normal),
        field(messages.firmware, firmware, Severity::Normal),
        field(messages.serial, serial, Severity::Normal),
    ]
}

fn field(label: &'static str, value: impl Into<String>, severity: Severity) -> SmartField {
    SmartField {
        label,
        value: value.into(),
        severity,
    }
}

fn coded_metric(name: &str, code: u8) -> String {
    format!("{name} ({code:02X})")
}

fn nvme_row(
    code: u8,
    name: &'static str,
    value: String,
    raw: String,
    severity: Severity,
) -> SmartRow {
    SmartRow {
        cells: vec![coded_metric(name, code), value, raw],
        severity,
    }
}

fn decimal_row(
    code: u8,
    name: &'static str,
    counter: &DecimalCounter,
    formatter: impl FnOnce(&str) -> String,
) -> SmartRow {
    nvme_row(
        code,
        name,
        formatter(&counter.0),
        counter.0.clone(),
        Severity::Normal,
    )
}

fn interpret_ata(attribute: &AtaSmartAttribute, messages: &Messages) -> String {
    match attribute.id {
        0x09 => format_hours(&attribute.raw_value.to_string(), messages),
        0x0C | 0x04 | 0x05 | 0xBB | 0xBC | 0xBF | 0xC0 | 0xC1 | 0xC4 | 0xC5 | 0xC6 | 0xC7 => {
            group_decimal(&attribute.raw_value.to_string())
        }
        0xBE | 0xC2 if attribute.raw_bytes[0] <= 125 => {
            format!("{} °C", attribute.raw_bytes[0])
        }
        _ => "—".into(),
    }
}

fn ata_temperature(data: &AtaSmartSnapshot) -> Option<i32> {
    data.attributes
        .iter()
        .find(|attribute| attribute.id == 0xC2 && attribute.raw_bytes[0] <= 125)
        .map(|attribute| i32::from(attribute.raw_bytes[0]))
}

fn format_ata_raw(bytes: [u8; 6]) -> String {
    bytes
        .iter()
        .rev()
        .map(|value| format!("{value:02X}"))
        .collect()
}

fn format_data_units(value: &str, messages: &Messages) -> String {
    let Ok(units) = value.parse::<f64>() else {
        return messages.not_available.into();
    };
    format_bytes(units * 512_000.0)
}

fn format_bytes(bytes: f64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes;
    let mut index = 0;
    while value >= 1_000.0 && index < UNITS.len() - 1 {
        value /= 1_000.0;
        index += 1;
    }
    if index == 0 {
        format!("{value:.0} {}", UNITS[index])
    } else {
        format!("{value:.1} {}", UNITS[index])
    }
}

fn format_decimal_count(value: &str, messages: &Messages) -> String {
    if value.bytes().all(|byte| byte.is_ascii_digit()) && !value.is_empty() {
        group_decimal(value)
    } else {
        messages.not_available.into()
    }
}

fn group_decimal(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + value.len() / 3);
    for (index, character) in value.chars().enumerate() {
        if index > 0 && (value.len() - index).is_multiple_of(3) {
            output.push(',');
        }
        output.push(character);
    }
    output
}

fn format_hours(value: &str, messages: &Messages) -> String {
    let Ok(hours) = value.parse::<u128>() else {
        return messages.not_available.into();
    };
    let days = hours / 24;
    if days == 0 {
        format!("{} {}", group_decimal(value), messages.hours)
    } else {
        format!(
            "{} {} / {} {}",
            group_decimal(&days.to_string()),
            messages.days,
            group_decimal(value),
            messages.hours
        )
    }
}

fn format_minutes(value: &str, messages: &Messages) -> String {
    let Ok(minutes) = value.parse::<u128>() else {
        return messages.not_available.into();
    };
    if minutes < 60 {
        format!("{} {}", group_decimal(value), messages.minutes)
    } else {
        format!(
            "{:.1} {} / {} {}",
            minutes as f64 / 60.0,
            messages.hours,
            group_decimal(value),
            messages.minutes
        )
    }
}

fn format_unavailable_reason(reason: &SmartUnavailableReason, messages: &Messages) -> String {
    match reason {
        SmartUnavailableReason::InterfaceNotExposed => messages.interface_not_exposed.into(),
        SmartUnavailableReason::DeviceNotSmartCapable => messages.device_not_smart_capable.into(),
        SmartUnavailableReason::UnsupportedProtocol { protocol } => {
            format!("{}: {protocol}", messages.unsupported_protocol)
        }
        SmartUnavailableReason::UnsupportedTransport { transport } => {
            format!("{}: {transport}", messages.unsupported_transport)
        }
    }
}

fn health_severity(health: &HealthState) -> Severity {
    match health {
        HealthState::Warning { .. } => Severity::Warning,
        HealthState::Critical { .. } => Severity::Critical,
        HealthState::Healthy { .. } | HealthState::Unknown { .. } => Severity::Normal,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::*;

    use super::*;

    #[test]
    fn ata_projection_uses_catalog_and_preserves_raw_bytes() {
        let smart = SmartSnapshot::Ata {
            data: AtaSmartSnapshot {
                identity: DeviceIdentityData::default(),
                version: 1,
                overall_passed: Some(true),
                attributes: vec![
                    AtaSmartAttribute {
                        id: 0x05,
                        flags: 0,
                        current: 5,
                        worst: 5,
                        threshold: Some(10),
                        raw_value: 3,
                        raw_bytes: [3, 2, 1, 0, 0, 0],
                    },
                    AtaSmartAttribute {
                        id: 0xAA,
                        flags: 0,
                        current: 1,
                        worst: 1,
                        threshold: None,
                        raw_value: 0,
                        raw_bytes: [0; 6],
                    },
                ],
            },
        };
        let snapshot = available_snapshot(smart);
        let view = project_smart(&snapshot, Locale::ZhCn);
        assert_eq!(view.kind, SmartViewKind::Ata);
        assert_eq!(view.rows[0].cells[0], "重新分配扇区数 (05)");
        assert_eq!(view.rows[0].cells[5], "000000010203");
        assert_eq!(view.rows[0].severity, Severity::Warning);
        assert_eq!(view.rows[1].cells[0], "未知属性 (AA)");
    }

    #[test]
    fn nvme_projection_formats_units_and_retains_exact_counter() {
        let smart = SmartSnapshot::Nvme {
            data: Box::new(NvmeSmartSnapshot {
                identity: DeviceIdentityData::default(),
                critical_warning: 0,
                temperature_celsius: Some(31),
                available_spare_percent: 100,
                available_spare_threshold_percent: 10,
                percentage_used: 1,
                data_units_read: DecimalCounter("2_000_000".replace('_', "")),
                data_units_written: DecimalCounter("0".into()),
                host_read_commands: DecimalCounter("1234567".into()),
                host_write_commands: DecimalCounter("0".into()),
                controller_busy_minutes: DecimalCounter("90".into()),
                power_cycles: DecimalCounter("10".into()),
                power_on_hours: DecimalCounter("48".into()),
                unsafe_shutdowns: DecimalCounter("0".into()),
                media_errors: DecimalCounter("0".into()),
                error_log_entries: DecimalCounter("0".into()),
            }),
        };
        let snapshot = available_snapshot(smart);
        let view = project_smart(&snapshot, Locale::En);
        assert_eq!(view.kind, SmartViewKind::Nvme);
        let read = view
            .rows
            .iter()
            .find(|row| row.cells[0] == "Data read (06)")
            .unwrap();
        assert_eq!(read.cells[1], "1.0 TB");
        assert_eq!(read.cells[2], "2000000");
        assert!(
            view.rows
                .iter()
                .any(|row| row.cells[0] == "Media errors (0E)")
        );
        assert!(render_smart_text(&snapshot, Locale::En).contains("1,234,567"));
    }

    #[test]
    fn unavailable_projection_keeps_primary_label_and_reason() {
        let snapshot =
            DeviceSnapshot::unavailable(device(), SmartUnavailableReason::InterfaceNotExposed);
        let text = render_smart_text(&snapshot, Locale::ZhCn);
        assert!(text.contains("SMART 不可用"));
        assert!(text.contains("设备接口未公开 SMART 数据"));
    }

    #[test]
    fn nvme_evidence_drives_row_severity() {
        let smart = SmartSnapshot::Nvme {
            data: Box::new(NvmeSmartSnapshot {
                critical_warning: 2,
                available_spare_percent: 5,
                available_spare_threshold_percent: 10,
                percentage_used: 100,
                ..nvme_defaults()
            }),
        };
        let view = project_smart(&available_snapshot(smart), Locale::En);
        assert_eq!(view.rows[0].severity, Severity::Critical);
        assert_eq!(view.rows[2].severity, Severity::Warning);
        assert_eq!(view.rows[4].severity, Severity::Warning);
    }

    fn available_snapshot(smart: SmartSnapshot) -> DeviceSnapshot {
        DeviceSnapshot {
            device: device(),
            health: evaluate_health(&smart),
            smart: SmartState::Available {
                snapshot: Box::new(smart),
                warnings: Vec::new(),
            },
            observed_at_unix_seconds: 0,
        }
    }

    fn device() -> DeviceRecord {
        DeviceRecord {
            id: DeviceId("disk:test".into()),
            generation: 1,
            device_node: PathBuf::from("/dev/disk9"),
            identity: DeviceIdentity {
                model: Some("Test Disk".into()),
                serial: Some("SERIAL".into()),
                firmware: Some("1.0".into()),
            },
            connection: ConnectionInfo {
                protocol: StorageProtocol::Nvme,
                bus: ConnectionBus::Pcie,
                removable: false,
            },
            capacity_bytes: 1_000_000_000,
            external: true,
        }
    }

    fn nvme_defaults() -> NvmeSmartSnapshot {
        NvmeSmartSnapshot {
            identity: DeviceIdentityData::default(),
            critical_warning: 0,
            temperature_celsius: None,
            available_spare_percent: 100,
            available_spare_threshold_percent: 10,
            percentage_used: 0,
            data_units_read: DecimalCounter("0".into()),
            data_units_written: DecimalCounter("0".into()),
            host_read_commands: DecimalCounter("0".into()),
            host_write_commands: DecimalCounter("0".into()),
            controller_busy_minutes: DecimalCounter("0".into()),
            power_cycles: DecimalCounter("0".into()),
            power_on_hours: DecimalCounter("0".into()),
            unsafe_shutdowns: DecimalCounter("0".into()),
            media_errors: DecimalCounter("0".into()),
            error_log_entries: DecimalCounter("0".into()),
        }
    }
}
