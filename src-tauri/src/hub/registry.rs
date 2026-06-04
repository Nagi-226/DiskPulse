use super::pairing::now_epoch_ms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static DEVICE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    pub name: String,
    pub address: String,
    pub paired: bool,
    pub connected: bool,
    pub last_seen_epoch_ms: u64,
}

impl DeviceInfo {
    pub fn new_paired(name: String, address: String) -> Self {
        let id = DEVICE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            device_id: format!("device-{id}"),
            name,
            address,
            paired: true,
            connected: true,
            last_seen_epoch_ms: now_epoch_ms(),
        }
    }
}

#[derive(Debug, Default)]
pub struct DeviceRegistry {
    devices: HashMap<String, DeviceInfo>,
}

impl DeviceRegistry {
    pub fn upsert(&mut self, device: DeviceInfo) {
        self.devices.insert(device.device_id.clone(), device);
    }

    pub fn list_devices(&self) -> Vec<DeviceInfo> {
        let mut devices: Vec<DeviceInfo> = self.devices.values().cloned().collect();
        devices.sort_by(|left, right| left.name.cmp(&right.name));
        devices
    }

    pub fn mark_connected(&mut self, device_id: &str, connected: bool) -> Option<DeviceInfo> {
        let device = self.devices.get_mut(device_id)?;
        device.connected = connected;
        device.last_seen_epoch_ms = now_epoch_ms();
        Some(device.clone())
    }

    pub fn unpair(&mut self, device_id: &str) -> bool {
        self.devices.remove(device_id).is_some()
    }
}
