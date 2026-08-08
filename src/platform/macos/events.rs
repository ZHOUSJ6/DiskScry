use std::{
    ffi::c_void,
    ptr,
    sync::mpsc::{self, Receiver, RecvError, RecvTimeoutError, Sender, TryRecvError},
    thread::{self, JoinHandle},
    time::Duration,
};

use core_foundation_sys::{
    base::{CFRelease, CFTypeRef},
    dictionary::CFDictionaryRef,
    runloop::{
        CFRunLoopGetCurrent, CFRunLoopRef, CFRunLoopRun, CFRunLoopStop, CFRunLoopWakeUp,
        kCFRunLoopDefaultMode,
    },
    string::CFStringRef,
};

use crate::platform::{DeviceEvent, DeviceEventSource, DeviceEventSubscription, PlatformError};

use super::MacOsInventory;

type DASessionRef = *const c_void;
type DADiskRef = *const c_void;
type DiskCallback = unsafe extern "C" fn(DADiskRef, *mut c_void);

#[link(name = "DiskArbitration", kind = "framework")]
unsafe extern "C" {
    fn DASessionCreate(allocator: *const c_void) -> DASessionRef;
    fn DASessionScheduleWithRunLoop(
        session: DASessionRef,
        run_loop: CFRunLoopRef,
        run_loop_mode: CFStringRef,
    );
    fn DASessionUnscheduleFromRunLoop(
        session: DASessionRef,
        run_loop: CFRunLoopRef,
        run_loop_mode: CFStringRef,
    );
    fn DARegisterDiskAppearedCallback(
        session: DASessionRef,
        matching: CFDictionaryRef,
        callback: DiskCallback,
        context: *mut c_void,
    );
    fn DARegisterDiskDisappearedCallback(
        session: DASessionRef,
        matching: CFDictionaryRef,
        callback: DiskCallback,
        context: *mut c_void,
    );
}

pub struct MacOsDiskSubscription {
    receiver: Receiver<DeviceEvent>,
    run_loop: usize,
    thread: Option<JoinHandle<()>>,
}

impl DeviceEventSubscription for MacOsDiskSubscription {
    fn try_recv(&self) -> Result<DeviceEvent, TryRecvError> {
        self.receiver.try_recv()
    }

    fn recv(&self) -> Result<DeviceEvent, RecvError> {
        self.receiver.recv()
    }

    fn recv_timeout(&self, timeout: Duration) -> Result<DeviceEvent, RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

impl Drop for MacOsDiskSubscription {
    fn drop(&mut self) {
        let run_loop = self.run_loop as CFRunLoopRef;
        unsafe {
            CFRunLoopStop(run_loop);
            CFRunLoopWakeUp(run_loop);
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl DeviceEventSource for MacOsInventory {
    type Subscription = MacOsDiskSubscription;

    fn subscribe(&self) -> Result<Self::Subscription, PlatformError> {
        let (event_sender, event_receiver) = mpsc::channel();
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let thread = thread::spawn(move || run_session(event_sender, ready_sender));
        let run_loop = ready_receiver
            .recv()
            .map_err(|error| PlatformError::Inventory(error.to_string()))??;
        Ok(MacOsDiskSubscription {
            receiver: event_receiver,
            run_loop,
            thread: Some(thread),
        })
    }
}

fn run_session(sender: Sender<DeviceEvent>, ready: mpsc::SyncSender<Result<usize, PlatformError>>) {
    let session = unsafe { DASessionCreate(ptr::null()) };
    if session.is_null() {
        let _ = ready.send(Err(PlatformError::Inventory(
            "DASessionCreate returned null".into(),
        )));
        return;
    }

    let context = Box::new(sender);
    let context_pointer = Box::into_raw(context).cast::<c_void>();
    let run_loop = unsafe { CFRunLoopGetCurrent() };
    unsafe {
        DARegisterDiskAppearedCallback(session, ptr::null(), disk_changed, context_pointer);
        DARegisterDiskDisappearedCallback(session, ptr::null(), disk_changed, context_pointer);
        DASessionScheduleWithRunLoop(session, run_loop, kCFRunLoopDefaultMode);
    }
    if ready.send(Ok(run_loop as usize)).is_ok() {
        unsafe {
            CFRunLoopRun();
        }
    }
    unsafe {
        DASessionUnscheduleFromRunLoop(session, run_loop, kCFRunLoopDefaultMode);
        CFRelease(session as CFTypeRef);
        drop(Box::from_raw(context_pointer.cast::<Sender<DeviceEvent>>()));
    }
}

unsafe extern "C" fn disk_changed(_disk: DADiskRef, context: *mut c_void) {
    if let Some(sender) = unsafe { context.cast::<Sender<DeviceEvent>>().as_ref() } {
        let _ = sender.send(DeviceEvent::InventoryChanged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires unsandboxed Disk Arbitration access"]
    fn subscription_starts_and_stops() {
        let subscription = MacOsInventory.subscribe().unwrap();
        drop(subscription);
    }

    #[test]
    #[ignore = "requires unsandboxed Disk Arbitration access"]
    fn subscription_receives_initial_inventory_events() {
        let subscription = MacOsInventory.subscribe().unwrap();
        assert_eq!(
            subscription.recv_timeout(Duration::from_secs(2)).unwrap(),
            DeviceEvent::InventoryChanged
        );
    }
}
