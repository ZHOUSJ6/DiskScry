use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SmartState {
    Available {
        snapshot: Box<SmartSnapshot>,
        warnings: Vec<SmartReadError>,
    },
    Unavailable {
        reason: SmartUnavailableReason,
    },
    Failed {
        error: SmartReadError,
    },
}

impl SmartState {
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Available { .. } => "SMART available",
            Self::Unavailable { .. } | Self::Failed { .. } => "SMART unavailable",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "protocol", rename_all = "snake_case")]
pub enum SmartSnapshot {
    Ata { data: AtaSmartSnapshot },
    Nvme { data: Box<NvmeSmartSnapshot> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum SmartUnavailableReason {
    InterfaceNotExposed,
    DeviceNotSmartCapable,
    UnsupportedProtocol { protocol: String },
    UnsupportedTransport { transport: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmartReadStage {
    InterfaceAcquisition,
    Identify,
    SmartData,
    Thresholds,
    Parse,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmartReadError {
    pub stage: SmartReadStage,
    pub operation: String,
    pub message: String,
    pub native_code: Option<i64>,
    pub permission_denied: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtaSmartSnapshot {
    pub identity: DeviceIdentityData,
    pub version: u16,
    pub overall_passed: Option<bool>,
    pub attributes: Vec<AtaSmartAttribute>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentityData {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub firmware: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AtaSmartAttribute {
    pub id: u8,
    pub flags: u16,
    pub current: u8,
    pub worst: u8,
    pub threshold: Option<u8>,
    pub raw_value: u64,
    pub raw_bytes: [u8; 6],
}

impl AtaSmartAttribute {
    pub fn threshold_crossed(&self) -> bool {
        self.threshold
            .is_some_and(|threshold| threshold > 0 && self.current <= threshold)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NvmeSmartSnapshot {
    pub identity: DeviceIdentityData,
    pub critical_warning: u8,
    pub temperature_celsius: Option<i32>,
    pub available_spare_percent: u8,
    pub available_spare_threshold_percent: u8,
    pub percentage_used: u8,
    pub data_units_read: DecimalCounter,
    pub data_units_written: DecimalCounter,
    pub host_read_commands: DecimalCounter,
    pub host_write_commands: DecimalCounter,
    pub controller_busy_minutes: DecimalCounter,
    pub power_cycles: DecimalCounter,
    pub power_on_hours: DecimalCounter,
    pub unsafe_shutdowns: DecimalCounter,
    pub media_errors: DecimalCounter,
    pub error_log_entries: DecimalCounter,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DecimalCounter(pub String);

impl From<u128> for DecimalCounter {
    fn from(value: u128) -> Self {
        Self(value.to_string())
    }
}
