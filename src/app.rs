use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    JSON_SCHEMA_VERSION,
    domain::{DeviceSnapshot, HealthState, HealthUnknownReason, SmartState, evaluate_health},
    platform::{DeviceInventory, PlatformError, SmartReader},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEnvelope {
    pub schema_version: u32,
    pub devices: Vec<DeviceSnapshot>,
}

impl SnapshotEnvelope {
    pub fn new(devices: Vec<DeviceSnapshot>) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION,
            devices,
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Platform(#[from] PlatformError),
    #[error("device selector '{0}' did not match an emitted device id or device node")]
    DeviceNotFound(String),
}

pub fn collect_snapshots<I: DeviceInventory, R: SmartReader>(
    inventory: &I,
    reader: &R,
) -> Result<Vec<DeviceSnapshot>, AppError> {
    let observed_at_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let devices = inventory
        .list()?
        .into_iter()
        .map(|device| {
            let smart = reader.read(&device);
            let health = match &smart {
                SmartState::Available { snapshot, .. } => evaluate_health(snapshot),
                SmartState::Unavailable { .. } => HealthState::Unknown {
                    reason: HealthUnknownReason::SmartUnavailable,
                },
                SmartState::Failed { .. } => HealthState::Unknown {
                    reason: HealthUnknownReason::SmartReadFailed,
                },
            };
            DeviceSnapshot {
                device,
                smart,
                health,
                observed_at_unix_seconds,
            }
        })
        .collect();
    Ok(devices)
}

pub fn select_device<'a>(
    devices: &'a [DeviceSnapshot],
    selector: &str,
) -> Result<&'a DeviceSnapshot, AppError> {
    devices
        .iter()
        .find(|snapshot| {
            snapshot.device.id.0 == selector
                || snapshot.device.device_node.to_string_lossy() == selector
                || snapshot
                    .device
                    .device_node
                    .file_name()
                    .is_some_and(|name| name == selector)
        })
        .ok_or_else(|| AppError::DeviceNotFound(selector.into()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::*;

    use super::*;

    #[derive(Clone)]
    struct Inventory(DeviceRecord);

    impl DeviceInventory for Inventory {
        fn list(&self) -> Result<Vec<DeviceRecord>, PlatformError> {
            Ok(vec![self.0.clone()])
        }
    }

    struct UnavailableReader;

    impl SmartReader for UnavailableReader {
        fn read(&self, _device: &DeviceRecord) -> SmartState {
            SmartState::Unavailable {
                reason: SmartUnavailableReason::InterfaceNotExposed,
            }
        }
    }

    #[test]
    fn json_distinguishes_unavailable_from_failed() {
        let device = DeviceRecord {
            id: DeviceId("disk:test".into()),
            generation: 1,
            device_node: PathBuf::from("/dev/disk9"),
            identity: DeviceIdentity::default(),
            connection: ConnectionInfo {
                protocol: StorageProtocol::Unknown,
                bus: ConnectionBus::Usb,
                removable: true,
            },
            capacity_bytes: 1000,
            external: true,
        };
        let unavailable = DeviceSnapshot::unavailable(
            device.clone(),
            SmartUnavailableReason::InterfaceNotExposed,
        );
        let failed = DeviceSnapshot {
            device,
            smart: SmartState::Failed {
                error: SmartReadError {
                    stage: SmartReadStage::SmartData,
                    operation: "read SMART data".into(),
                    message: "permission denied".into(),
                    native_code: Some(13),
                    permission_denied: true,
                },
            },
            health: HealthState::Unknown {
                reason: HealthUnknownReason::SmartReadFailed,
            },
            observed_at_unix_seconds: 0,
        };

        let json = serde_json::to_value(SnapshotEnvelope::new(vec![unavailable, failed])).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["devices"][0]["smart"]["state"], "unavailable");
        assert_eq!(json["devices"][1]["smart"]["state"], "failed");
        let serialized = serde_json::to_string(&json).unwrap();
        assert!(!serialized.contains("SMART unavailable"));
        assert!(!serialized.contains("SMART 不可用"));
    }

    #[test]
    fn external_disk_survives_unavailable_smart_enrichment() {
        let device = DeviceRecord {
            id: DeviceId("disk:external".into()),
            generation: 1,
            device_node: PathBuf::from("/dev/disk9"),
            identity: DeviceIdentity::default(),
            connection: ConnectionInfo {
                protocol: StorageProtocol::Scsi,
                bus: ConnectionBus::Usb,
                removable: true,
            },
            capacity_bytes: 1_000,
            external: true,
        };
        let snapshots = collect_snapshots(&Inventory(device), &UnavailableReader).unwrap();
        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].device.external);
        assert_eq!(snapshots[0].smart.display_label(), "SMART unavailable");
        assert!(matches!(snapshots[0].health, HealthState::Unknown { .. }));
    }
}
