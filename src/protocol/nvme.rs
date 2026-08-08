use crate::domain::{DecimalCounter, DeviceIdentityData, NvmeSmartSnapshot};

use super::{ParseError, require_len};

const IDENTIFY_CONTROLLER_SIZE: usize = 4096;
const SMART_LOG_SIZE: usize = 512;

pub fn parse_identify_controller(bytes: &[u8]) -> Result<DeviceIdentityData, ParseError> {
    require_len(bytes, IDENTIFY_CONTROLLER_SIZE, "NVMe Identify Controller")?;
    Ok(DeviceIdentityData {
        serial: trimmed_ascii(&bytes[4..24]),
        model: trimmed_ascii(&bytes[24..64]),
        firmware: trimmed_ascii(&bytes[64..72]),
    })
}

pub fn parse_smart_log(
    identity: DeviceIdentityData,
    bytes: &[u8],
) -> Result<NvmeSmartSnapshot, ParseError> {
    require_len(bytes, SMART_LOG_SIZE, "NVMe SMART / Health log")?;
    let kelvin = u16::from_le_bytes([bytes[1], bytes[2]]);

    Ok(NvmeSmartSnapshot {
        identity,
        critical_warning: bytes[0],
        temperature_celsius: (kelvin != 0).then_some(i32::from(kelvin) - 273),
        available_spare_percent: bytes[3],
        available_spare_threshold_percent: bytes[4],
        percentage_used: bytes[5],
        data_units_read: counter(bytes, 32),
        data_units_written: counter(bytes, 48),
        host_read_commands: counter(bytes, 64),
        host_write_commands: counter(bytes, 80),
        controller_busy_minutes: counter(bytes, 96),
        power_cycles: counter(bytes, 112),
        power_on_hours: counter(bytes, 128),
        unsafe_shutdowns: counter(bytes, 144),
        media_errors: counter(bytes, 160),
        error_log_entries: counter(bytes, 176),
    })
}

fn counter(bytes: &[u8], offset: usize) -> DecimalCounter {
    let value = u128::from_le_bytes(
        bytes[offset..offset + 16]
            .try_into()
            .expect("NVMe counter has a fixed width"),
    );
    value.into()
}

fn trimmed_ascii(bytes: &[u8]) -> Option<String> {
    let value = String::from_utf8_lossy(bytes)
        .trim_matches(|character: char| character == '\0' || character.is_ascii_whitespace())
        .to_string();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_identify_and_health_log() {
        let mut identify = [0_u8; IDENTIFY_CONTROLLER_SIZE];
        identify[4..24].copy_from_slice(b"SERIAL              ");
        identify[24..64].copy_from_slice(b"DiskScry NVMe                           ");
        identify[64..72].copy_from_slice(b"1.0     ");
        let identity = parse_identify_controller(&identify).unwrap();

        let mut log = [0_u8; SMART_LOG_SIZE];
        log[0] = 0;
        log[1..3].copy_from_slice(&303_u16.to_le_bytes());
        log[3] = 100;
        log[4] = 10;
        log[5] = 4;
        log[32..48].copy_from_slice(&1234_u128.to_le_bytes());

        let snapshot = parse_smart_log(identity, &log).unwrap();
        assert_eq!(snapshot.identity.serial.as_deref(), Some("SERIAL"));
        assert_eq!(snapshot.temperature_celsius, Some(30));
        assert_eq!(snapshot.data_units_read.0, "1234");
    }

    #[test]
    fn rejects_short_log() {
        assert!(matches!(
            parse_smart_log(DeviceIdentityData::default(), &[0; 64]),
            Err(ParseError::Truncated { .. })
        ));
    }
}
