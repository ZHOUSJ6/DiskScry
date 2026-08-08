use std::collections::HashMap;

use crate::domain::{AtaSmartAttribute, AtaSmartSnapshot, DeviceIdentityData};

use super::{ParseError, require_len};

const ATA_SECTOR_SIZE: usize = 512;
const ATTRIBUTE_START: usize = 2;
const ATTRIBUTE_SIZE: usize = 12;
const ATTRIBUTE_COUNT: usize = 30;

pub fn parse_identify(bytes: &[u8]) -> Result<DeviceIdentityData, ParseError> {
    require_len(bytes, ATA_SECTOR_SIZE, "ATA IDENTIFY DEVICE")?;
    Ok(DeviceIdentityData {
        serial: ata_string(bytes, 10, 10),
        firmware: ata_string(bytes, 23, 4),
        model: ata_string(bytes, 27, 20),
    })
}

pub fn parse_smart(
    identify: DeviceIdentityData,
    data: &[u8],
    thresholds: Option<&[u8]>,
    overall_passed: Option<bool>,
) -> Result<AtaSmartSnapshot, ParseError> {
    require_len(data, ATA_SECTOR_SIZE, "ATA SMART data")?;
    validate_checksum(data, "ATA SMART data")?;

    let threshold_map = thresholds
        .map(parse_thresholds)
        .transpose()?
        .unwrap_or_default();
    let mut attributes = Vec::new();

    for index in 0..ATTRIBUTE_COUNT {
        let offset = ATTRIBUTE_START + index * ATTRIBUTE_SIZE;
        let record = &data[offset..offset + ATTRIBUTE_SIZE];
        let id = record[0];
        if id == 0 {
            continue;
        }

        let raw_bytes: [u8; 6] = record[5..11]
            .try_into()
            .expect("ATA attribute raw value has a fixed width");
        let raw_value = raw_bytes
            .iter()
            .enumerate()
            .fold(0_u64, |value, (shift, byte)| {
                value | ((*byte as u64) << (shift * 8))
            });

        attributes.push(AtaSmartAttribute {
            id,
            flags: u16::from_le_bytes([record[1], record[2]]),
            current: record[3],
            worst: record[4],
            threshold: threshold_map.get(&id).copied(),
            raw_value,
            raw_bytes,
        });
    }

    Ok(AtaSmartSnapshot {
        identity: identify,
        version: u16::from_le_bytes([data[0], data[1]]),
        overall_passed,
        attributes,
    })
}

fn parse_thresholds(bytes: &[u8]) -> Result<HashMap<u8, u8>, ParseError> {
    require_len(bytes, ATA_SECTOR_SIZE, "ATA SMART thresholds")?;
    validate_checksum(bytes, "ATA SMART thresholds")?;

    let mut values = HashMap::new();
    for index in 0..ATTRIBUTE_COUNT {
        let offset = ATTRIBUTE_START + index * ATTRIBUTE_SIZE;
        let id = bytes[offset];
        if id != 0 {
            values.insert(id, bytes[offset + 1]);
        }
    }
    Ok(values)
}

fn validate_checksum(bytes: &[u8], structure: &'static str) -> Result<(), ParseError> {
    if bytes[..ATA_SECTOR_SIZE]
        .iter()
        .fold(0_u8, |sum, byte| sum.wrapping_add(*byte))
        != 0
    {
        return Err(ParseError::InvalidChecksum { structure });
    }
    Ok(())
}

fn ata_string(bytes: &[u8], start_word: usize, word_count: usize) -> Option<String> {
    let mut value = Vec::with_capacity(word_count * 2);
    for word in start_word..start_word + word_count {
        value.push(bytes[word * 2 + 1]);
        value.push(bytes[word * 2]);
    }
    trimmed_ascii(&value)
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

    fn finalize_checksum(page: &mut [u8; ATA_SECTOR_SIZE]) {
        let sum = page[..ATA_SECTOR_SIZE - 1]
            .iter()
            .fold(0_u8, |sum, byte| sum.wrapping_add(*byte));
        page[ATA_SECTOR_SIZE - 1] = 0_u8.wrapping_sub(sum);
    }

    fn write_ata_string(page: &mut [u8; ATA_SECTOR_SIZE], start_word: usize, text: &str) {
        let mut bytes = text.as_bytes().to_vec();
        if !bytes.len().is_multiple_of(2) {
            bytes.push(b' ');
        }
        for (index, pair) in bytes.chunks_exact(2).enumerate() {
            page[(start_word + index) * 2] = pair[1];
            page[(start_word + index) * 2 + 1] = pair[0];
        }
    }

    #[test]
    fn parses_identify_word_swapped_strings() {
        let mut page = [b' '; ATA_SECTOR_SIZE];
        write_ata_string(&mut page, 10, "SERIAL123");
        write_ata_string(&mut page, 23, "1.0");
        write_ata_string(&mut page, 27, "DiskScry Test Disk");

        let identity = parse_identify(&page).unwrap();
        assert_eq!(identity.serial.as_deref(), Some("SERIAL123"));
        assert_eq!(identity.firmware.as_deref(), Some("1.0"));
        assert_eq!(identity.model.as_deref(), Some("DiskScry Test Disk"));
    }

    #[test]
    fn parses_attributes_and_thresholds() {
        let mut data = [0_u8; ATA_SECTOR_SIZE];
        data[0..2].copy_from_slice(&1_u16.to_le_bytes());
        data[2] = 5;
        data[3..5].copy_from_slice(&1_u16.to_le_bytes());
        data[5] = 10;
        data[6] = 9;
        data[7..13].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        finalize_checksum(&mut data);

        let mut thresholds = [0_u8; ATA_SECTOR_SIZE];
        thresholds[2] = 5;
        thresholds[3] = 11;
        finalize_checksum(&mut thresholds);

        let snapshot = parse_smart(
            DeviceIdentityData::default(),
            &data,
            Some(&thresholds),
            Some(true),
        )
        .unwrap();
        let attribute = &snapshot.attributes[0];
        assert_eq!(attribute.id, 5);
        assert_eq!(attribute.threshold, Some(11));
        assert!(attribute.threshold_crossed());
        assert_eq!(attribute.raw_value, 0x0605_0403_0201);
    }

    #[test]
    fn rejects_invalid_checksum() {
        let mut data = [0_u8; ATA_SECTOR_SIZE];
        data[3] = 1;
        assert!(matches!(
            parse_smart(DeviceIdentityData::default(), &data, None, None),
            Err(ParseError::InvalidChecksum { .. })
        ));
    }
}
