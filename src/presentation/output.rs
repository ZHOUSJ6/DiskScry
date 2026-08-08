use crate::{
    domain::{DeviceSnapshot, HealthState},
    presentation::{locale::Locale, smart_view},
};

pub fn smart_label(snapshot: &DeviceSnapshot, locale: Locale) -> &'static str {
    locale.smart_label(&snapshot.smart)
}

pub fn health_label(health: &HealthState, locale: Locale) -> &'static str {
    locale.health_label(health)
}

pub fn render_list(devices: &[DeviceSnapshot], locale: Locale) -> String {
    let messages = locale.messages();
    let mut output = format!(
        "{}\t{}\t{}\t{}\t{}\n",
        messages.device, messages.model, messages.capacity, messages.health, messages.smart
    );
    for snapshot in devices {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            snapshot.device.device_node.display(),
            snapshot
                .device
                .identity
                .model
                .as_deref()
                .unwrap_or(messages.unknown),
            format_capacity(snapshot.device.capacity_bytes),
            health_label(&snapshot.health, locale),
            smart_label(snapshot, locale),
        ));
    }
    output
}

pub fn render_detail(snapshot: &DeviceSnapshot, locale: Locale) -> String {
    let mut output = render_overview(snapshot, locale);
    output.push('\n');
    output.push_str(&smart_view::render_smart_text(snapshot, locale));
    output
}

pub fn render_overview(snapshot: &DeviceSnapshot, locale: Locale) -> String {
    let messages = locale.messages();
    format!(
        "{}: {}\n{}: {}\n{}: {}\n{}: {}\n{}: {:?}\n{}: {:?}\n{}: {}\n{}: {}\n",
        messages.device,
        snapshot.device.device_node.display(),
        messages.id,
        snapshot.device.id.0,
        messages.model,
        snapshot
            .device
            .identity
            .model
            .as_deref()
            .unwrap_or(messages.unknown),
        messages.capacity,
        format_capacity(snapshot.device.capacity_bytes),
        messages.protocol,
        snapshot.device.connection.protocol,
        messages.connection,
        snapshot.device.connection.bus,
        messages.health,
        health_label(&snapshot.health, locale),
        messages.smart,
        smart_label(snapshot, locale),
    )
}

pub fn temperature(snapshot: &DeviceSnapshot) -> Option<i32> {
    match &snapshot.smart {
        crate::domain::SmartState::Available { snapshot, .. } => match snapshot.as_ref() {
            crate::domain::SmartSnapshot::Nvme { data } => data.temperature_celsius,
            crate::domain::SmartSnapshot::Ata { .. } => None,
        },
        _ => None,
    }
}

fn format_capacity(bytes: u64) -> String {
    const GIGABYTE: f64 = 1_000_000_000.0;
    format!("{:.1} GB", bytes as f64 / GIGABYTE)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::*;

    use super::*;

    #[test]
    fn unavailable_uses_exact_primary_label() {
        let device = DeviceRecord {
            id: DeviceId("disk:test".into()),
            generation: 0,
            device_node: PathBuf::from("/dev/disk9"),
            identity: DeviceIdentity::default(),
            connection: ConnectionInfo {
                protocol: StorageProtocol::Unknown,
                bus: ConnectionBus::Usb,
                removable: true,
            },
            capacity_bytes: 1,
            external: true,
        };
        let snapshot =
            DeviceSnapshot::unavailable(device, SmartUnavailableReason::InterfaceNotExposed);
        assert_eq!(smart_label(&snapshot, Locale::En), "SMART unavailable");
        assert_eq!(smart_label(&snapshot, Locale::ZhCn), "SMART 不可用");
        let detail = render_detail(&snapshot, Locale::ZhCn);
        assert!(detail.contains("SMART 详细信息"));
        assert!(detail.contains("设备接口未公开 SMART 数据"));
        assert!(
            render_list(&[snapshot], Locale::ZhCn)
                .starts_with("设备\t型号\t容量\t健康状态\tSMART\n")
        );
    }

    #[test]
    fn human_detail_includes_readable_smart_without_changing_json() {
        let smart = SmartSnapshot::Nvme {
            data: Box::new(NvmeSmartSnapshot {
                identity: DeviceIdentityData::default(),
                critical_warning: 0,
                temperature_celsius: Some(30),
                available_spare_percent: 100,
                available_spare_threshold_percent: 10,
                percentage_used: 1,
                data_units_read: DecimalCounter("2000000".into()),
                data_units_written: DecimalCounter("0".into()),
                host_read_commands: DecimalCounter("0".into()),
                host_write_commands: DecimalCounter("0".into()),
                controller_busy_minutes: DecimalCounter("0".into()),
                power_cycles: DecimalCounter("0".into()),
                power_on_hours: DecimalCounter("0".into()),
                unsafe_shutdowns: DecimalCounter("0".into()),
                media_errors: DecimalCounter("0".into()),
                error_log_entries: DecimalCounter("0".into()),
            }),
        };
        let snapshot = DeviceSnapshot {
            device: DeviceRecord {
                id: DeviceId("disk:test".into()),
                generation: 0,
                device_node: PathBuf::from("/dev/disk9"),
                identity: DeviceIdentity::default(),
                connection: ConnectionInfo {
                    protocol: StorageProtocol::Nvme,
                    bus: ConnectionBus::Pcie,
                    removable: false,
                },
                capacity_bytes: 1,
                external: true,
            },
            health: evaluate_health(&smart),
            smart: SmartState::Available {
                snapshot: Box::new(smart),
                warnings: Vec::new(),
            },
            observed_at_unix_seconds: 0,
        };

        let detail = render_detail(&snapshot, Locale::En);
        assert!(detail.contains("Data read (06)\t1.0 TB\t2000000"));
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(!json.contains("Data read"));
        assert!(!json.contains("读取数据量"));
    }
}
