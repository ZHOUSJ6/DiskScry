use std::{
    ffi::{CString, c_void},
    ptr,
};

use core_foundation_sys::uuid::{
    CFUUIDBytes, CFUUIDGetConstantUUIDWithBytes, CFUUIDGetUUIDBytes, CFUUIDRef,
};

use crate::{
    domain::{
        DeviceIdentityData, DeviceRecord, SmartReadError, SmartReadStage, SmartSnapshot,
        SmartState, SmartUnavailableReason,
    },
    platform::SmartReader,
    protocol::{ata, nvme},
};

use super::iokit::{IoService, RegistryEntry};

const ATA_USER_CLIENT_UUID: [u8; 16] = [
    0x24, 0x51, 0x4B, 0x7A, 0x28, 0x04, 0x11, 0xD6, 0x8A, 0x02, 0x00, 0x30, 0x65, 0x70, 0x48, 0x66,
];
const ATA_INTERFACE_UUID: [u8; 16] = [
    0x08, 0xAB, 0xE2, 0x1C, 0x20, 0xD4, 0x11, 0xD6, 0x8D, 0xF6, 0x00, 0x03, 0x93, 0x5A, 0x76, 0xB2,
];
const NVME_USER_CLIENT_UUID: [u8; 16] = [
    0xAA, 0x0F, 0xA6, 0xF9, 0xC2, 0xD6, 0x45, 0x7F, 0xB1, 0x0B, 0x59, 0xA1, 0x32, 0x53, 0x29, 0x2F,
];
const NVME_INTERFACE_UUID: [u8; 16] = [
    0xCC, 0xD1, 0xDB, 0x19, 0xFD, 0x9A, 0x4D, 0xAF, 0xBF, 0x95, 0x12, 0x45, 0x4B, 0x23, 0x0A, 0xB6,
];
const PLUGIN_INTERFACE_UUID: [u8; 16] = [
    0xC2, 0x44, 0xE8, 0x58, 0x10, 0x9C, 0x11, 0xD4, 0x91, 0xD4, 0x00, 0x50, 0xE4, 0xC6, 0x42, 0x6F,
];

type QueryInterface = unsafe extern "C" fn(*mut c_void, CFUUIDBytes, *mut *mut c_void) -> i32;
type AddRef = unsafe extern "C" fn(*mut c_void) -> u32;
type Release = unsafe extern "C" fn(*mut c_void) -> u32;
type UnusedMethod = unsafe extern "C" fn();

#[repr(C)]
struct PluginVTable {
    reserved: *mut c_void,
    query_interface: Option<QueryInterface>,
    add_ref: Option<AddRef>,
    release: Option<Release>,
    version: u16,
    revision: u16,
    probe: Option<UnusedMethod>,
    start: Option<UnusedMethod>,
    stop: Option<UnusedMethod>,
}

type PluginHandle = *mut *mut PluginVTable;

#[repr(C)]
struct AtaVTable {
    reserved: *mut c_void,
    query_interface: Option<QueryInterface>,
    add_ref: Option<AddRef>,
    release: Option<Release>,
    version: u16,
    revision: u16,
    enable_operations: Option<UnusedMethod>,
    enable_autosave: Option<UnusedMethod>,
    return_status: Option<unsafe extern "C" fn(*mut c_void, *mut u8) -> i32>,
    execute_offline: Option<UnusedMethod>,
    read_data: Option<unsafe extern "C" fn(*mut c_void, *mut u8) -> i32>,
    validate_data: Option<UnusedMethod>,
    read_thresholds: Option<unsafe extern "C" fn(*mut c_void, *mut u8) -> i32>,
    read_log_directory: Option<UnusedMethod>,
    read_log_at: Option<UnusedMethod>,
    write_log_at: Option<UnusedMethod>,
    get_identify: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32, *mut u32) -> i32>,
}

type AtaHandle = *mut *mut AtaVTable;

#[repr(C)]
struct NvmeVTable {
    reserved: *mut c_void,
    query_interface: Option<QueryInterface>,
    add_ref: Option<AddRef>,
    release: Option<Release>,
    version: u16,
    revision: u16,
    read_data: Option<unsafe extern "C" fn(*mut c_void, *mut u8) -> i32>,
    get_identify: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32) -> i32>,
    reserved0: u64,
    reserved1: u64,
    get_log_page: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, u32, u32) -> i32>,
}

type NvmeHandle = *mut *mut NvmeVTable;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOCreatePlugInInterfaceForService(
        service: IoService,
        plugin_type: CFUUIDRef,
        interface_type: CFUUIDRef,
        interface: *mut PluginHandle,
        score: *mut i32,
    ) -> i32;
    fn IODestroyPlugInInterface(interface: PluginHandle) -> i32;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MacOsSmartReader;

impl SmartReader for MacOsSmartReader {
    fn read(&self, device: &DeviceRecord) -> SmartState {
        match find_smart_service(device) {
            Ok(Some((SmartKind::Ata, service))) => read_ata(service),
            Ok(Some((SmartKind::Nvme, service))) => read_nvme(service),
            Ok(None) => SmartState::Unavailable {
                reason: SmartUnavailableReason::InterfaceNotExposed,
            },
            Err(error) => SmartState::Failed { error },
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum SmartKind {
    Ata,
    Nvme,
}

fn find_smart_service(
    device: &DeviceRecord,
) -> Result<Option<(SmartKind, RegistryEntry)>, SmartReadError> {
    let bsd_name = device
        .device_node
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            read_error(
                SmartReadStage::InterfaceAcquisition,
                "resolve BSD device name",
                "device node has no valid UTF-8 file name",
                None,
            )
        })?;
    let bsd_name = CString::new(bsd_name).map_err(|_| {
        read_error(
            SmartReadStage::InterfaceAcquisition,
            "resolve BSD device name",
            "device node contains an interior null byte",
            None,
        )
    })?;
    let mut current = RegistryEntry::from_bsd_name(&bsd_name).ok_or_else(|| {
        read_error(
            SmartReadStage::InterfaceAcquisition,
            "locate IOKit media service",
            "IOKit could not resolve the device node",
            None,
        )
    })?;

    loop {
        if current.has_direct_property("NVMe SMART Capable") {
            return Ok(Some((SmartKind::Nvme, current)));
        }
        if current.has_direct_property("SMART Capable") {
            return Ok(Some((SmartKind::Ata, current)));
        }
        let Some(parent) = current.parent() else {
            return Ok(None);
        };
        current = parent;
    }
}

fn read_ata(service: RegistryEntry) -> SmartState {
    let connection = match AtaConnection::open(service.raw()) {
        Ok(connection) => connection,
        Err(error) => return SmartState::Failed { error },
    };
    let table = match connection.table() {
        Ok(table) => table,
        Err(error) => return SmartState::Failed { error },
    };
    let interface = connection.interface.cast();
    let mut warnings = Vec::new();

    let identity = match table.get_identify {
        Some(get_identify) => {
            let mut bytes = [0_u8; 512];
            let mut output_size = 0_u32;
            let status = unsafe {
                get_identify(
                    interface,
                    bytes.as_mut_ptr().cast(),
                    bytes.len() as u32,
                    &mut output_size,
                )
            };
            if status == 0 && output_size as usize == bytes.len() {
                match ata::parse_identify(&bytes) {
                    Ok(identity) => identity,
                    Err(error) => {
                        warnings.push(parse_warning(SmartReadStage::Identify, error.to_string()));
                        DeviceIdentityData::default()
                    }
                }
            } else {
                warnings.push(status_error(
                    SmartReadStage::Identify,
                    "read ATA identify data",
                    status,
                ));
                DeviceIdentityData::default()
            }
        }
        None => {
            warnings.push(read_error(
                SmartReadStage::Identify,
                "read ATA identify data",
                "IOATASMARTInterface does not expose GetATAIdentifyData",
                None,
            ));
            DeviceIdentityData::default()
        }
    };

    let mut data = [0_u8; 512];
    let Some(read_data) = table.read_data else {
        return SmartState::Failed {
            error: read_error(
                SmartReadStage::SmartData,
                "read ATA SMART data",
                "IOATASMARTInterface does not expose SMARTReadData",
                None,
            ),
        };
    };
    let status = unsafe { read_data(interface, data.as_mut_ptr()) };
    if status != 0 {
        return SmartState::Failed {
            error: status_error(SmartReadStage::SmartData, "read ATA SMART data", status),
        };
    }

    let thresholds = table.read_thresholds.and_then(|read_thresholds| {
        let mut values = [0_u8; 512];
        let status = unsafe { read_thresholds(interface, values.as_mut_ptr()) };
        if status == 0 {
            Some(values)
        } else {
            warnings.push(status_error(
                SmartReadStage::Thresholds,
                "read ATA SMART thresholds",
                status,
            ));
            None
        }
    });

    let overall_passed = table.return_status.and_then(|return_status| {
        let mut exceeded = 0_u8;
        let status = unsafe { return_status(interface, &mut exceeded) };
        if status == 0 {
            Some(exceeded == 0)
        } else {
            warnings.push(status_error(
                SmartReadStage::SmartData,
                "read ATA SMART overall status",
                status,
            ));
            None
        }
    });

    match ata::parse_smart(
        identity,
        &data,
        thresholds.as_ref().map(|value| value.as_slice()),
        overall_passed,
    ) {
        Ok(data) => SmartState::Available {
            snapshot: Box::new(SmartSnapshot::Ata { data }),
            warnings,
        },
        Err(error) => SmartState::Failed {
            error: parse_warning(SmartReadStage::Parse, error.to_string()),
        },
    }
}

fn read_nvme(service: RegistryEntry) -> SmartState {
    let connection = match NvmeConnection::open(service.raw()) {
        Ok(connection) => connection,
        Err(error) => return SmartState::Failed { error },
    };
    let table = match connection.table() {
        Ok(table) => table,
        Err(error) => return SmartState::Failed { error },
    };
    let interface = connection.interface.cast();
    let mut warnings = Vec::new();

    let identity = match table.get_identify {
        Some(get_identify) => {
            let mut bytes = [0_u8; 4096];
            let status = unsafe { get_identify(interface, bytes.as_mut_ptr().cast(), 0) };
            if status == 0 {
                match nvme::parse_identify_controller(&bytes) {
                    Ok(identity) => identity,
                    Err(error) => {
                        warnings.push(parse_warning(SmartReadStage::Identify, error.to_string()));
                        DeviceIdentityData::default()
                    }
                }
            } else {
                warnings.push(status_error(
                    SmartReadStage::Identify,
                    "read NVMe identify controller",
                    status,
                ));
                DeviceIdentityData::default()
            }
        }
        None => {
            warnings.push(read_error(
                SmartReadStage::Identify,
                "read NVMe identify controller",
                "IONVMeSMARTInterface does not expose GetIdentifyData",
                None,
            ));
            DeviceIdentityData::default()
        }
    };

    let Some(read_data) = table.read_data else {
        return SmartState::Failed {
            error: read_error(
                SmartReadStage::SmartData,
                "read NVMe SMART data",
                "IONVMeSMARTInterface does not expose SMARTReadData",
                None,
            ),
        };
    };
    let mut bytes = [0_u8; 512];
    let status = unsafe { read_data(interface, bytes.as_mut_ptr()) };
    if status != 0 {
        return SmartState::Failed {
            error: status_error(SmartReadStage::SmartData, "read NVMe SMART data", status),
        };
    }

    match nvme::parse_smart_log(identity, &bytes) {
        Ok(data) => SmartState::Available {
            snapshot: Box::new(SmartSnapshot::Nvme {
                data: Box::new(data),
            }),
            warnings,
        },
        Err(error) => SmartState::Failed {
            error: parse_warning(SmartReadStage::Parse, error.to_string()),
        },
    }
}

struct AtaConnection {
    interface: AtaHandle,
    plugin: PluginHandle,
}

impl AtaConnection {
    fn open(service: IoService) -> Result<Self, SmartReadError> {
        let plugin = create_plugin(service, ATA_USER_CLIENT_UUID)?;
        match query_interface::<AtaVTable>(plugin, ATA_INTERFACE_UUID) {
            Ok(interface) => Ok(Self { interface, plugin }),
            Err(error) => {
                unsafe {
                    IODestroyPlugInInterface(plugin);
                }
                Err(error)
            }
        }
    }

    fn table(&self) -> Result<&AtaVTable, SmartReadError> {
        interface_table(self.interface, "IOATASMARTInterface")
    }
}

impl Drop for AtaConnection {
    fn drop(&mut self) {
        release_interface(self.interface);
        unsafe {
            IODestroyPlugInInterface(self.plugin);
        }
    }
}

struct NvmeConnection {
    interface: NvmeHandle,
    plugin: PluginHandle,
}

impl NvmeConnection {
    fn open(service: IoService) -> Result<Self, SmartReadError> {
        let plugin = create_plugin(service, NVME_USER_CLIENT_UUID)?;
        match query_interface::<NvmeVTable>(plugin, NVME_INTERFACE_UUID) {
            Ok(interface) => Ok(Self { interface, plugin }),
            Err(error) => {
                unsafe {
                    IODestroyPlugInInterface(plugin);
                }
                Err(error)
            }
        }
    }

    fn table(&self) -> Result<&NvmeVTable, SmartReadError> {
        interface_table(self.interface, "IONVMeSMARTInterface")
    }
}

impl Drop for NvmeConnection {
    fn drop(&mut self) {
        release_interface(self.interface);
        unsafe {
            IODestroyPlugInInterface(self.plugin);
        }
    }
}

fn create_plugin(
    service: IoService,
    user_client_uuid: [u8; 16],
) -> Result<PluginHandle, SmartReadError> {
    let mut plugin = ptr::null_mut();
    let mut score = 0_i32;
    let status = unsafe {
        IOCreatePlugInInterfaceForService(
            service,
            constant_uuid(user_client_uuid),
            constant_uuid(PLUGIN_INTERFACE_UUID),
            &mut plugin,
            &mut score,
        )
    };
    if status != 0 || plugin.is_null() {
        return Err(status_error(
            SmartReadStage::InterfaceAcquisition,
            "create IOKit SMART plugin",
            status,
        ));
    }
    Ok(plugin)
}

fn query_interface<T>(
    plugin: PluginHandle,
    interface_uuid: [u8; 16],
) -> Result<*mut *mut T, SmartReadError> {
    if plugin.is_null() || unsafe { (*plugin).is_null() } {
        return Err(interface_pointer_error("IOCFPlugInInterface"));
    }
    let table = unsafe { &**plugin };
    let query = table
        .query_interface
        .ok_or_else(|| interface_pointer_error("IOCFPlugInInterface::QueryInterface"))?;
    let mut interface = ptr::null_mut();
    let status = unsafe {
        query(
            plugin.cast(),
            CFUUIDGetUUIDBytes(constant_uuid(interface_uuid)),
            &mut interface,
        )
    };
    if status < 0 || interface.is_null() {
        return Err(status_error(
            SmartReadStage::InterfaceAcquisition,
            "query IOKit SMART interface",
            status,
        ));
    }
    Ok(interface.cast())
}

fn interface_table<T>(
    interface: *mut *mut T,
    name: &'static str,
) -> Result<&'static T, SmartReadError> {
    if interface.is_null() || unsafe { (*interface).is_null() } {
        return Err(interface_pointer_error(name));
    }
    Ok(unsafe { &**interface })
}

fn release_interface<T>(interface: *mut *mut T) {
    if interface.is_null() || unsafe { (*interface).is_null() } {
        return;
    }
    let prefix = unsafe { &*((*interface).cast::<UnknownPrefix>()) };
    if let Some(release) = prefix.release {
        unsafe {
            release(interface.cast());
        }
    }
}

#[repr(C)]
struct UnknownPrefix {
    reserved: *mut c_void,
    query_interface: Option<QueryInterface>,
    add_ref: Option<AddRef>,
    release: Option<Release>,
}

fn constant_uuid(bytes: [u8; 16]) -> CFUUIDRef {
    unsafe {
        CFUUIDGetConstantUUIDWithBytes(
            ptr::null(),
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            bytes[8],
            bytes[9],
            bytes[10],
            bytes[11],
            bytes[12],
            bytes[13],
            bytes[14],
            bytes[15],
        )
    }
}

fn interface_pointer_error(name: &'static str) -> SmartReadError {
    read_error(
        SmartReadStage::InterfaceAcquisition,
        "acquire IOKit SMART interface",
        format!("{name} returned a null interface pointer"),
        None,
    )
}

fn status_error(stage: SmartReadStage, operation: &'static str, status: i32) -> SmartReadError {
    read_error(
        stage,
        operation,
        format!("{operation} failed with native status {status:#x}"),
        Some(status),
    )
}

fn parse_warning(stage: SmartReadStage, message: String) -> SmartReadError {
    read_error(stage, "parse SMART response", message, None)
}

fn read_error(
    stage: SmartReadStage,
    operation: impl Into<String>,
    message: impl Into<String>,
    native_code: Option<i32>,
) -> SmartReadError {
    SmartReadError {
        stage,
        operation: operation.into(),
        message: message.into(),
        native_code: native_code.map(i64::from),
        permission_denied: native_code.is_some_and(is_permission_status),
    }
}

fn is_permission_status(status: i32) -> bool {
    let raw = status as u32;
    let code = raw & 0x3fff;
    code == 0x2c1 || code == 0x2e2 || raw == 0x8000_0009
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_iokit_permission_statuses() {
        assert!(is_permission_status(0xe000_02c1_u32 as i32));
        assert!(is_permission_status(0xe000_02e2_u32 as i32));
        assert!(is_permission_status(0x8000_0009_u32 as i32));
        assert!(!is_permission_status(0));
    }

    #[test]
    fn smart_interface_prefixes_match_64_bit_apple_abi() {
        assert_eq!(std::mem::size_of::<PluginVTable>(), 64);
        assert_eq!(std::mem::offset_of!(PluginVTable, query_interface), 8);
        assert_eq!(std::mem::offset_of!(PluginVTable, version), 32);
        assert_eq!(std::mem::offset_of!(PluginVTable, probe), 40);

        assert_eq!(std::mem::size_of::<AtaVTable>(), 128);
        assert_eq!(std::mem::offset_of!(AtaVTable, return_status), 56);
        assert_eq!(std::mem::offset_of!(AtaVTable, read_data), 72);
        assert_eq!(std::mem::offset_of!(AtaVTable, read_thresholds), 88);
        assert_eq!(std::mem::offset_of!(AtaVTable, get_identify), 120);

        assert_eq!(std::mem::size_of::<NvmeVTable>(), 80);
        assert_eq!(std::mem::offset_of!(NvmeVTable, read_data), 40);
        assert_eq!(std::mem::offset_of!(NvmeVTable, get_identify), 48);
        assert_eq!(std::mem::offset_of!(NvmeVTable, get_log_page), 72);
    }
}
