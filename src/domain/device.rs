use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{HealthState, SmartState};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(pub String);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageProtocol {
    Ata,
    Nvme,
    Scsi,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionBus {
    Internal,
    Usb,
    Thunderbolt,
    Pcie,
    Sata,
    Sas,
    Virtual,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub protocol: StorageProtocol,
    pub bus: ConnectionBus,
    pub removable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub id: DeviceId,
    pub generation: u64,
    pub device_node: PathBuf,
    pub identity: DeviceIdentity,
    pub connection: ConnectionInfo,
    pub capacity_bytes: u64,
    pub external: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    #[serde(flatten)]
    pub device: DeviceRecord,
    pub smart: SmartState,
    pub health: HealthState,
    pub observed_at_unix_seconds: u64,
}

impl DeviceSnapshot {
    pub fn unavailable(device: DeviceRecord, reason: super::SmartUnavailableReason) -> Self {
        Self {
            device,
            smart: SmartState::Unavailable { reason },
            health: HealthState::Unknown {
                reason: super::HealthUnknownReason::SmartUnavailable,
            },
            observed_at_unix_seconds: 0,
        }
    }
}
