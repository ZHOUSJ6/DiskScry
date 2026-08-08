use serde::{Deserialize, Serialize};

use super::{AtaSmartSnapshot, NvmeSmartSnapshot, SmartSnapshot};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum HealthState {
    Healthy { evidence: Vec<HealthEvidence> },
    Warning { evidence: Vec<HealthEvidence> },
    Critical { evidence: Vec<HealthEvidence> },
    Unknown { reason: HealthUnknownReason },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HealthEvidence {
    AtaOverall {
        passed: bool,
    },
    AtaThresholdCrossed {
        attribute_id: u8,
        current: u8,
        threshold: u8,
    },
    NvmeCriticalWarning {
        bits: u8,
    },
    NvmeSpare {
        available_percent: u8,
        threshold_percent: u8,
    },
    NvmePercentageUsed {
        percent: u8,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthUnknownReason {
    SmartUnavailable,
    SmartReadFailed,
    InsufficientEvidence,
}

pub fn evaluate_health(snapshot: &SmartSnapshot) -> HealthState {
    match snapshot {
        SmartSnapshot::Ata { data } => evaluate_ata(data),
        SmartSnapshot::Nvme { data } => evaluate_nvme(data),
    }
}

fn evaluate_ata(snapshot: &AtaSmartSnapshot) -> HealthState {
    let crossings = snapshot
        .attributes
        .iter()
        .filter_map(|attribute| {
            attribute
                .threshold
                .filter(|_| attribute.threshold_crossed())
                .map(|threshold| HealthEvidence::AtaThresholdCrossed {
                    attribute_id: attribute.id,
                    current: attribute.current,
                    threshold,
                })
        })
        .collect::<Vec<_>>();

    if snapshot.overall_passed == Some(false) {
        let mut evidence = vec![HealthEvidence::AtaOverall { passed: false }];
        evidence.extend(crossings);
        return HealthState::Critical { evidence };
    }

    if !crossings.is_empty() {
        return HealthState::Warning {
            evidence: crossings,
        };
    }

    match snapshot.overall_passed {
        Some(true) => HealthState::Healthy {
            evidence: vec![HealthEvidence::AtaOverall { passed: true }],
        },
        None => HealthState::Unknown {
            reason: HealthUnknownReason::InsufficientEvidence,
        },
        Some(false) => unreachable!("handled above"),
    }
}

fn evaluate_nvme(snapshot: &NvmeSmartSnapshot) -> HealthState {
    if snapshot.critical_warning != 0 {
        return HealthState::Critical {
            evidence: vec![HealthEvidence::NvmeCriticalWarning {
                bits: snapshot.critical_warning,
            }],
        };
    }

    let mut warnings = Vec::new();
    if snapshot.available_spare_percent < snapshot.available_spare_threshold_percent {
        warnings.push(HealthEvidence::NvmeSpare {
            available_percent: snapshot.available_spare_percent,
            threshold_percent: snapshot.available_spare_threshold_percent,
        });
    }
    if snapshot.percentage_used >= 100 {
        warnings.push(HealthEvidence::NvmePercentageUsed {
            percent: snapshot.percentage_used,
        });
    }

    if warnings.is_empty() {
        HealthState::Healthy {
            evidence: vec![HealthEvidence::NvmeCriticalWarning { bits: 0 }],
        }
    } else {
        HealthState::Warning { evidence: warnings }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AtaSmartAttribute, DecimalCounter, DeviceIdentityData};

    fn nvme() -> NvmeSmartSnapshot {
        NvmeSmartSnapshot {
            identity: DeviceIdentityData::default(),
            critical_warning: 0,
            temperature_celsius: Some(30),
            available_spare_percent: 100,
            available_spare_threshold_percent: 10,
            percentage_used: 1,
            data_units_read: DecimalCounter::default(),
            data_units_written: DecimalCounter::default(),
            host_read_commands: DecimalCounter::default(),
            host_write_commands: DecimalCounter::default(),
            controller_busy_minutes: DecimalCounter::default(),
            power_cycles: DecimalCounter::default(),
            power_on_hours: DecimalCounter::default(),
            unsafe_shutdowns: DecimalCounter::default(),
            media_errors: DecimalCounter::default(),
            error_log_entries: DecimalCounter::default(),
        }
    }

    #[test]
    fn nvme_critical_warning_is_critical() {
        let mut value = nvme();
        value.critical_warning = 0b10;
        assert!(matches!(
            evaluate_nvme(&value),
            HealthState::Critical { .. }
        ));
    }

    #[test]
    fn ata_without_reported_status_is_unknown() {
        let value = AtaSmartSnapshot {
            identity: DeviceIdentityData::default(),
            version: 1,
            overall_passed: None,
            attributes: vec![AtaSmartAttribute {
                id: 5,
                flags: 0,
                current: 100,
                worst: 100,
                threshold: None,
                raw_value: 0,
                raw_bytes: [0; 6],
            }],
        };
        assert_eq!(
            evaluate_ata(&value),
            HealthState::Unknown {
                reason: HealthUnknownReason::InsufficientEvidence
            }
        );
    }
}
