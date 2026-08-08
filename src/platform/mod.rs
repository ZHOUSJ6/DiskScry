use std::{
    sync::mpsc::{RecvError, RecvTimeoutError, TryRecvError},
    time::Duration,
};

use thiserror::Error;

use crate::domain::{DeviceRecord, SmartState};

#[cfg(target_os = "macos")]
pub mod macos;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("device inventory failed: {0}")]
    Inventory(String),
}

pub trait DeviceInventory {
    fn list(&self) -> Result<Vec<DeviceRecord>, PlatformError>;
}

pub trait SmartReader {
    fn read(&self, device: &DeviceRecord) -> SmartState;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceEvent {
    InventoryChanged,
}

pub trait DeviceEventSubscription {
    fn try_recv(&self) -> Result<DeviceEvent, TryRecvError>;
    fn recv(&self) -> Result<DeviceEvent, RecvError>;
    fn recv_timeout(&self, timeout: Duration) -> Result<DeviceEvent, RecvTimeoutError>;
}

pub trait DeviceEventSource {
    type Subscription: DeviceEventSubscription;

    fn subscribe(&self) -> Result<Self::Subscription, PlatformError>;
}
