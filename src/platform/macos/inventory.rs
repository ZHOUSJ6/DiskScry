use std::path::PathBuf;

use crate::{
    domain::{
        ConnectionBus, ConnectionInfo, DeviceId, DeviceIdentity, DeviceRecord, StorageProtocol,
    },
    platform::{DeviceInventory, PlatformError},
};

use super::iokit::{RegistryEntry, ServiceIterator};

const IO_MEDIA: &[u8] = b"IOMedia\0";
const APPLE_APFS_MEDIA: &[u8] = b"AppleAPFSMedia\0";
const HDIX_DRIVE: &[u8] = b"IOHDIXHDDriveInKernel\0";
const HDIX_CONTROLLER: &[u8] = b"IOHDIXController\0";

#[derive(Clone, Copy, Debug, Default)]
pub struct MacOsInventory;

impl DeviceInventory for MacOsInventory {
    fn list(&self) -> Result<Vec<DeviceRecord>, PlatformError> {
        let services = ServiceIterator::matching(IO_MEDIA).map_err(PlatformError::Inventory)?;
        let mut devices = services
            .filter(|entry| property_bool(entry, "Whole").unwrap_or(false))
            .filter(is_physical_media)
            .filter_map(device_from_entry)
            .collect::<Vec<_>>();
        devices.sort_by(|left, right| left.device_node.cmp(&right.device_node));
        Ok(devices)
    }
}

fn is_physical_media(entry: &RegistryEntry) -> bool {
    !entry.conforms_to(APPLE_APFS_MEDIA)
        && !entry.has_ancestor_conforming_to(HDIX_DRIVE)
        && !entry.has_ancestor_conforming_to(HDIX_CONTROLLER)
}

fn device_from_entry(entry: RegistryEntry) -> Option<DeviceRecord> {
    let bsd_name = entry.direct_property("BSD Name")?.as_string()?;
    let capacity_bytes = entry
        .direct_property("Size")
        .and_then(|value| value.as_i64())
        .and_then(|value| u64::try_from(value).ok())?;
    let removable = property_bool(&entry, "Removable").unwrap_or(false);
    let ejectable = property_bool(&entry, "Ejectable").unwrap_or(false);

    let protocol_characteristics = entry.parent_property("Protocol Characteristics");
    let interconnect = protocol_characteristics
        .as_ref()
        .and_then(|value| value.dictionary_string("Physical Interconnect"))
        .or_else(|| property_string(&entry, "Physical Interconnect"));
    let location = protocol_characteristics
        .as_ref()
        .and_then(|value| value.dictionary_string("Physical Interconnect Location"))
        .or_else(|| property_string(&entry, "Physical Interconnect Location"));

    let bus = classify_bus(interconnect.as_deref(), location.as_deref());
    let protocol = classify_protocol(
        &bus,
        interconnect.as_deref(),
        entry.parent_property("NVMe SMART Capable").is_some(),
        entry.parent_property("SMART Capable").is_some(),
    );
    let external = removable
        || ejectable
        || location
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case("external"))
        || matches!(bus, ConnectionBus::Usb | ConnectionBus::Thunderbolt);

    let model = property_string(&entry, "Product Name")
        .or_else(|| property_string(&entry, "Model"))
        .or_else(|| entry.name())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let serial = property_string(&entry, "Serial Number")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let firmware = property_string(&entry, "Firmware Revision")
        .or_else(|| property_string(&entry, "Revision"))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let registry_id = entry.id()?;
    Some(DeviceRecord {
        id: DeviceId(format!("iokit:{registry_id:016x}")),
        generation: registry_id,
        device_node: PathBuf::from(format!("/dev/{bsd_name}")),
        identity: DeviceIdentity {
            model,
            serial,
            firmware,
        },
        connection: ConnectionInfo {
            protocol,
            bus,
            removable,
        },
        capacity_bytes,
        external,
    })
}

fn property_bool(entry: &RegistryEntry, name: &str) -> Option<bool> {
    entry
        .direct_property(name)
        .or_else(|| entry.parent_property(name))
        .and_then(|value| value.as_bool())
}

fn property_string(entry: &RegistryEntry, name: &str) -> Option<String> {
    entry
        .direct_property(name)
        .or_else(|| entry.parent_property(name))
        .and_then(|value| value.as_string())
}

fn classify_bus(interconnect: Option<&str>, location: Option<&str>) -> ConnectionBus {
    let value = interconnect.unwrap_or_default().to_ascii_lowercase();
    if value.contains("usb") {
        ConnectionBus::Usb
    } else if value.contains("thunderbolt") {
        ConnectionBus::Thunderbolt
    } else if value.contains("pci") {
        ConnectionBus::Pcie
    } else if value.contains("sata") || value.contains("ata") {
        ConnectionBus::Sata
    } else if value.contains("sas") {
        ConnectionBus::Sas
    } else if location.is_some_and(|value| value.eq_ignore_ascii_case("internal")) {
        ConnectionBus::Internal
    } else {
        ConnectionBus::Unknown
    }
}

fn classify_protocol(
    bus: &ConnectionBus,
    interconnect: Option<&str>,
    nvme_smart_capable: bool,
    ata_smart_capable: bool,
) -> StorageProtocol {
    let value = interconnect.unwrap_or_default().to_ascii_lowercase();
    if nvme_smart_capable || value.contains("nvme") || matches!(bus, ConnectionBus::Pcie) {
        StorageProtocol::Nvme
    } else if ata_smart_capable || value.contains("sata") || value.contains("ata") {
        StorageProtocol::Ata
    } else if value.contains("scsi") || matches!(bus, ConnectionBus::Usb | ConnectionBus::Sas) {
        StorageProtocol::Scsi
    } else {
        StorageProtocol::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_external_buses() {
        assert_eq!(classify_bus(Some("USB"), None), ConnectionBus::Usb);
        assert_eq!(
            classify_bus(Some("PCI-Express"), Some("Internal")),
            ConnectionBus::Pcie
        );
    }

    #[test]
    fn maps_pcie_to_nvme() {
        assert_eq!(
            classify_protocol(&ConnectionBus::Pcie, Some("PCI-Express"), false, false),
            StorageProtocol::Nvme
        );
    }

    #[test]
    fn maps_smart_capability_to_protocol() {
        assert_eq!(
            classify_protocol(&ConnectionBus::Internal, Some("Apple Fabric"), true, false),
            StorageProtocol::Nvme
        );
    }
}
