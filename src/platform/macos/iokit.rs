use std::{ffi::c_char, ptr};

use core_foundation::{base::TCFType, string::CFString};
use core_foundation_sys::{
    base::{CFGetTypeID, CFRelease, CFTypeRef},
    dictionary::{CFDictionaryGetValue, CFDictionaryRef},
    number::{
        CFBooleanGetTypeID, CFBooleanGetValue, CFBooleanRef, CFNumberGetTypeID, CFNumberGetValue,
        CFNumberRef, kCFNumberSInt64Type,
    },
    string::{CFStringGetCString, CFStringGetTypeID, CFStringRef, kCFStringEncodingUTF8},
};

const IO_SERVICE_PLANE: &[u8] = b"IOService\0";
const REGISTRY_ITERATE_RECURSIVELY: u32 = 1;
const REGISTRY_ITERATE_PARENTS: u32 = 2;

pub(crate) type IoService = u32;
type IoIterator = u32;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOBSDNameMatching(
        main_port: u32,
        options: u32,
        bsd_name: *const c_char,
    ) -> *const core::ffi::c_void;
    fn IOServiceGetMatchingService(main_port: u32, matching: *const core::ffi::c_void)
    -> IoService;
    fn IOServiceMatching(name: *const c_char) -> *const core::ffi::c_void;
    fn IOServiceGetMatchingServices(
        main_port: u32,
        matching: *const core::ffi::c_void,
        existing: *mut IoIterator,
    ) -> i32;
    fn IOIteratorNext(iterator: IoIterator) -> IoService;
    fn IOObjectRelease(object: u32) -> i32;
    fn IOObjectConformsTo(object: u32, class_name: *const c_char) -> u8;
    fn IORegistryEntryGetRegistryEntryID(entry: IoService, entry_id: *mut u64) -> i32;
    fn IORegistryEntryGetName(entry: IoService, name: *mut c_char) -> i32;
    fn IORegistryEntryGetParentEntry(
        entry: IoService,
        plane: *const c_char,
        parent: *mut IoService,
    ) -> i32;
    fn IORegistryEntryCreateCFProperty(
        entry: IoService,
        key: CFStringRef,
        allocator: *const core::ffi::c_void,
        options: u32,
    ) -> CFTypeRef;
    fn IORegistryEntrySearchCFProperty(
        entry: IoService,
        plane: *const c_char,
        key: CFStringRef,
        allocator: *const core::ffi::c_void,
        options: u32,
    ) -> CFTypeRef;
}

pub(crate) struct ServiceIterator(IoIterator);

impl ServiceIterator {
    pub(crate) fn matching(class_name: &'static [u8]) -> Result<Self, String> {
        let matching = unsafe { IOServiceMatching(class_name.as_ptr().cast()) };
        if matching.is_null() {
            return Err("IOServiceMatching returned null".into());
        }

        let mut iterator = 0;
        let status = unsafe { IOServiceGetMatchingServices(0, matching, &mut iterator) };
        if status != 0 {
            return Err(format!(
                "IOServiceGetMatchingServices failed with IOReturn {status:#x}"
            ));
        }
        Ok(Self(iterator))
    }
}

impl Iterator for ServiceIterator {
    type Item = RegistryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let object = unsafe { IOIteratorNext(self.0) };
        (object != 0).then_some(RegistryEntry(object))
    }
}

impl Drop for ServiceIterator {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                IOObjectRelease(self.0);
            }
        }
    }
}

pub(crate) struct RegistryEntry(IoService);

impl RegistryEntry {
    pub(crate) fn from_bsd_name(name: &std::ffi::CStr) -> Option<Self> {
        let matching = unsafe { IOBSDNameMatching(0, 0, name.as_ptr()) };
        if matching.is_null() {
            return None;
        }
        let service = unsafe { IOServiceGetMatchingService(0, matching) };
        (service != 0).then_some(Self(service))
    }

    pub(crate) fn raw(&self) -> IoService {
        self.0
    }

    pub(crate) fn id(&self) -> Option<u64> {
        let mut value = 0;
        let status = unsafe { IORegistryEntryGetRegistryEntryID(self.0, &mut value) };
        (status == 0).then_some(value)
    }

    pub(crate) fn name(&self) -> Option<String> {
        let mut buffer = [0_i8; 128];
        let status = unsafe { IORegistryEntryGetName(self.0, buffer.as_mut_ptr()) };
        if status != 0 {
            return None;
        }
        let length = buffer.iter().position(|byte| *byte == 0)?;
        let bytes = buffer[..length]
            .iter()
            .map(|byte| *byte as u8)
            .collect::<Vec<_>>();
        String::from_utf8(bytes).ok()
    }

    pub(crate) fn parent(&self) -> Option<Self> {
        let mut parent = 0;
        let status = unsafe {
            IORegistryEntryGetParentEntry(self.0, IO_SERVICE_PLANE.as_ptr().cast(), &mut parent)
        };
        (status == 0 && parent != 0).then_some(Self(parent))
    }

    pub(crate) fn has_direct_property(&self, name: &str) -> bool {
        self.direct_property(name).is_some()
    }

    pub(crate) fn conforms_to(&self, class_name: &'static [u8]) -> bool {
        unsafe { IOObjectConformsTo(self.0, class_name.as_ptr().cast()) != 0 }
    }

    pub(crate) fn has_ancestor_conforming_to(&self, class_name: &'static [u8]) -> bool {
        let mut current = self.parent();
        while let Some(entry) = current {
            if entry.conforms_to(class_name) {
                return true;
            }
            current = entry.parent();
        }
        false
    }

    pub(crate) fn direct_property(&self, name: &str) -> Option<Property> {
        let key = CFString::new(name);
        let value = unsafe {
            IORegistryEntryCreateCFProperty(self.0, key.as_concrete_TypeRef(), ptr::null(), 0)
        };
        Property::from_owned(value)
    }

    pub(crate) fn parent_property(&self, name: &str) -> Option<Property> {
        let key = CFString::new(name);
        let value = unsafe {
            IORegistryEntrySearchCFProperty(
                self.0,
                IO_SERVICE_PLANE.as_ptr().cast(),
                key.as_concrete_TypeRef(),
                ptr::null(),
                REGISTRY_ITERATE_RECURSIVELY | REGISTRY_ITERATE_PARENTS,
            )
        };
        Property::from_owned(value)
    }
}

impl Drop for RegistryEntry {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                IOObjectRelease(self.0);
            }
        }
    }
}

pub(crate) struct Property(CFTypeRef);

impl Property {
    fn from_owned(value: CFTypeRef) -> Option<Self> {
        (!value.is_null()).then_some(Self(value))
    }

    pub(crate) fn as_bool(&self) -> Option<bool> {
        let is_boolean = unsafe { CFGetTypeID(self.0) == CFBooleanGetTypeID() };
        is_boolean.then(|| unsafe { CFBooleanGetValue(self.0 as CFBooleanRef) })
    }

    pub(crate) fn as_i64(&self) -> Option<i64> {
        let is_number = unsafe { CFGetTypeID(self.0) == CFNumberGetTypeID() };
        if !is_number {
            return None;
        }
        let mut value = 0_i64;
        let success = unsafe {
            CFNumberGetValue(
                self.0 as CFNumberRef,
                kCFNumberSInt64Type,
                (&mut value as *mut i64).cast(),
            )
        };
        success.then_some(value)
    }

    pub(crate) fn as_string(&self) -> Option<String> {
        string_from_cf(self.0)
    }

    pub(crate) fn dictionary_string(&self, name: &str) -> Option<String> {
        let is_dictionary = unsafe {
            CFGetTypeID(self.0) == core_foundation_sys::dictionary::CFDictionaryGetTypeID()
        };
        if !is_dictionary {
            return None;
        }

        let key = CFString::new(name);
        let value = unsafe {
            CFDictionaryGetValue(self.0 as CFDictionaryRef, key.as_concrete_TypeRef().cast())
        };
        string_from_cf(value.cast())
    }
}

impl Drop for Property {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
}

fn string_from_cf(value: CFTypeRef) -> Option<String> {
    if value.is_null() || unsafe { CFGetTypeID(value) != CFStringGetTypeID() } {
        return None;
    }

    let mut buffer = [0_i8; 1024];
    let success = unsafe {
        CFStringGetCString(
            value as CFStringRef,
            buffer.as_mut_ptr(),
            buffer.len() as isize,
            kCFStringEncodingUTF8,
        )
    };
    if success == 0 {
        return None;
    }

    let length = buffer.iter().position(|byte| *byte == 0)?;
    let bytes = buffer[..length]
        .iter()
        .map(|byte| *byte as u8)
        .collect::<Vec<_>>();
    String::from_utf8(bytes).ok()
}
