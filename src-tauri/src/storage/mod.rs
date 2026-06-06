use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const STORAGE_ATTACHED_EVENT: &str = "storage-attached";
pub const STORAGE_DETACHED_EVENT: &str = "storage-detached";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageKind {
    Removable,
    FixedExternal,
    Network,
    Optical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalStorageInfo {
    pub id: String,
    pub name: String,
    pub mount_path: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub removable: bool,
    pub kind: StorageKind,
    pub platform: String,
    pub detection_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageEventType {
    Attached,
    Detached,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageEvent {
    pub event_type: StorageEventType,
    pub storage: Option<ExternalStorageInfo>,
    pub mount_path: Option<String>,
    pub raw_code: Option<u32>,
    pub source: String,
}

pub trait ExternalStorageProvider: Send + Sync {
    fn list_external_storage(&self) -> Result<Vec<ExternalStorageInfo>, String>;
    fn get_storage_info(&self, path: &str) -> Result<ExternalStorageInfo, String>;
    fn detection_source(&self) -> &'static str;
}

pub struct StorageMonitorGuard {
    cancel: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl StorageMonitorGuard {
    pub fn stop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for StorageMonitorGuard {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn list_external_storage() -> Result<Vec<ExternalStorageInfo>, String> {
    platform_provider().list_external_storage()
}

pub fn get_storage_info(path: &str) -> Result<ExternalStorageInfo, String> {
    platform_provider().get_storage_info(path)
}

pub fn start_monitor<F>(poll_interval_ms: u64, on_event: F) -> Result<StorageMonitorGuard, String>
where
    F: Fn(StorageEvent) + Send + Sync + 'static,
{
    let provider = platform_provider();
    let source = provider.detection_source().to_string();
    let cancel = Arc::new(AtomicBool::new(false));
    let thread_cancel = cancel.clone();
    let on_event: Arc<dyn Fn(StorageEvent) + Send + Sync + 'static> = Arc::new(on_event);
    let interval = Duration::from_millis(poll_interval_ms.max(500));
    let handle = thread::Builder::new()
        .name("diskpulse-storage-monitor".into())
        .spawn(move || {
            let mut previous = list_by_mount(provider.list_external_storage().unwrap_or_default());
            while !thread_cancel.load(Ordering::Relaxed) {
                thread::sleep(interval);
                let current = list_by_mount(provider.list_external_storage().unwrap_or_default());
                for event in diff_storage_sets(&previous, &current, &source) {
                    on_event(event);
                }
                previous = current;
            }
        })
        .map_err(|e| format!("spawn storage monitor failed: {e}"))?;

    Ok(StorageMonitorGuard {
        cancel,
        handle: Some(handle),
    })
}

fn list_by_mount(items: Vec<ExternalStorageInfo>) -> HashMap<String, ExternalStorageInfo> {
    items
        .into_iter()
        .map(|item| (normalize_mount_key(&item.mount_path), item))
        .collect()
}

fn diff_storage_sets(
    previous: &HashMap<String, ExternalStorageInfo>,
    current: &HashMap<String, ExternalStorageInfo>,
    source: &str,
) -> Vec<StorageEvent> {
    let previous_keys: HashSet<_> = previous.keys().cloned().collect();
    let current_keys: HashSet<_> = current.keys().cloned().collect();
    let mut events = Vec::new();

    for key in current_keys.difference(&previous_keys) {
        if let Some(storage) = current.get(key.as_str()) {
            events.push(StorageEvent {
                event_type: StorageEventType::Attached,
                storage: Some(storage.clone()),
                mount_path: Some(storage.mount_path.clone()),
                raw_code: None,
                source: source.into(),
            });
        }
    }
    for key in previous_keys.difference(&current_keys) {
        if let Some(storage) = previous.get(key.as_str()) {
            events.push(StorageEvent {
                event_type: StorageEventType::Detached,
                storage: None,
                mount_path: Some(storage.mount_path.clone()),
                raw_code: None,
                source: source.into(),
            });
        }
    }
    events
}

fn normalize_mount_key(path: &str) -> String {
    path.replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

#[allow(dead_code)]
fn storage_name_from_path(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn platform_provider() -> Box<dyn ExternalStorageProvider> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows_storage::WindowsStorageProvider)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux_storage::LinuxStorageProvider)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos_storage::MacOsStorageProvider)
    }
}

#[cfg(target_os = "windows")]
mod windows_storage {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };

    pub struct WindowsStorageProvider;
    const DRIVE_NO_ROOT_DIR: u32 = 1;
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;

    impl ExternalStorageProvider for WindowsStorageProvider {
        fn list_external_storage(&self) -> Result<Vec<ExternalStorageInfo>, String> {
            let mask = unsafe { GetLogicalDrives() };
            if mask == 0 {
                return Err("GetLogicalDrives failed".into());
            }
            let mut drives = Vec::new();
            for i in 0..26 {
                if mask & (1 << i) == 0 {
                    continue;
                }
                let letter = (b'A' + i) as char;
                let root = format!("{letter}:\\");
                let drive_type = drive_type(&root);
                if matches!(drive_type, DRIVE_REMOVABLE | DRIVE_CDROM) {
                    drives.push(build_info(&root, drive_type)?);
                }
            }
            Ok(drives)
        }

        fn get_storage_info(&self, path: &str) -> Result<ExternalStorageInfo, String> {
            let root = windows_root(path);
            let drive_type = drive_type(&root);
            if drive_type == DRIVE_NO_ROOT_DIR {
                return Err(format!("Storage root is not available: {root}"));
            }
            build_info(&root, drive_type)
        }

        fn detection_source(&self) -> &'static str {
            "wm_devicechange_poll_fallback"
        }
    }

    fn build_info(root: &str, drive_type: u32) -> Result<ExternalStorageInfo, String> {
        let (total_bytes, free_bytes) = drive_space(root)?;
        Ok(ExternalStorageInfo {
            id: root.trim_end_matches('\\').to_string(),
            name: root.trim_end_matches('\\').to_string(),
            mount_path: root.into(),
            filesystem: filesystem_type(root).unwrap_or_else(|_| "unknown".into()),
            total_bytes,
            free_bytes,
            removable: matches!(drive_type, DRIVE_REMOVABLE | DRIVE_CDROM),
            kind: match drive_type {
                DRIVE_REMOVABLE => StorageKind::Removable,
                DRIVE_CDROM => StorageKind::Optical,
                DRIVE_REMOTE => StorageKind::Network,
                DRIVE_FIXED => StorageKind::FixedExternal,
                _ => StorageKind::Unknown,
            },
            platform: "windows".into(),
            detection_source: "wm_devicechange".into(),
        })
    }

    fn windows_root(path: &str) -> String {
        let trimmed = path.trim();
        let bytes = trimmed.as_bytes();
        if bytes.len() >= 2 && bytes[1] == b':' {
            format!("{}:\\", (bytes[0] as char).to_ascii_uppercase())
        } else {
            trimmed.to_string()
        }
    }

    fn drive_type(root: &str) -> u32 {
        let wide = wide(root);
        unsafe { GetDriveTypeW(windows::core::PCWSTR(wide.as_ptr())) }
    }

    fn drive_space(root: &str) -> Result<(u64, u64), String> {
        let wide = wide(root);
        let mut free_bytes_available = 0u64;
        let mut total_bytes = 0u64;
        let mut total_free_bytes = 0u64;
        unsafe {
            GetDiskFreeSpaceExW(
                windows::core::PCWSTR(wide.as_ptr()),
                Some(&mut free_bytes_available as *mut u64 as *mut _),
                Some(&mut total_bytes as *mut u64 as *mut _),
                Some(&mut total_free_bytes as *mut u64 as *mut _),
            )
            .map_err(|e| format!("GetDiskFreeSpaceExW failed for {root}: {e}"))?;
        }
        Ok((total_bytes, free_bytes_available))
    }

    fn filesystem_type(root: &str) -> Result<String, String> {
        let wide = wide(root);
        let mut fs_name = [0u16; 64];
        unsafe {
            GetVolumeInformationW(
                windows::core::PCWSTR(wide.as_ptr()),
                None,
                None,
                None,
                None,
                Some(&mut fs_name),
            )
        }
        .map_err(|e| format!("GetVolumeInformationW failed for {root}: {e}"))?;
        let len = fs_name
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(fs_name.len());
        Ok(String::from_utf16_lossy(&fs_name[..len]))
    }

    fn wide(value: &str) -> Vec<u16> {
        std::ffi::OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

#[cfg(target_os = "linux")]
mod linux_storage {
    use super::*;

    pub struct LinuxStorageProvider;

    impl ExternalStorageProvider for LinuxStorageProvider {
        fn list_external_storage(&self) -> Result<Vec<ExternalStorageInfo>, String> {
            Ok(
                parse_mounts(&std::fs::read_to_string("/proc/mounts").unwrap_or_default())
                    .into_iter()
                    .filter(|mount| looks_external_mount(&mount.mount_path))
                    .collect(),
            )
        }

        fn get_storage_info(&self, path: &str) -> Result<ExternalStorageInfo, String> {
            parse_mounts(&std::fs::read_to_string("/proc/mounts").unwrap_or_default())
                .into_iter()
                .filter(|mount| path.starts_with(&mount.mount_path))
                .max_by_key(|mount| mount.mount_path.len())
                .ok_or_else(|| format!("No mounted storage found for {path}"))
        }

        fn detection_source(&self) -> &'static str {
            "linux_mount_poll_fallback"
        }
    }

    fn parse_mounts(text: &str) -> Vec<ExternalStorageInfo> {
        text.lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let device = parts.next()?;
                let mount = parts.next()?.replace("\\040", " ");
                let fs = parts.next().unwrap_or("unknown");
                let (total_bytes, free_bytes) = df_bytes(&mount).unwrap_or((0, 0));
                Some(ExternalStorageInfo {
                    id: device.into(),
                    name: storage_name_from_path(&mount),
                    mount_path: mount,
                    filesystem: fs.into(),
                    total_bytes,
                    free_bytes,
                    removable: true,
                    kind: StorageKind::Removable,
                    platform: "linux".into(),
                    detection_source: "mount_table".into(),
                })
            })
            .collect()
    }

    fn looks_external_mount(path: &str) -> bool {
        path.starts_with("/media/")
            || path.starts_with("/mnt/")
            || path.starts_with("/run/media/")
            || path.starts_with("/Volumes/")
    }

    fn df_bytes(path: &str) -> Result<(u64, u64), String> {
        let output = std::process::Command::new("df")
            .args(["-B1", path])
            .output()
            .map_err(|e| format!("df failed: {e}"))?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut columns = text.lines().nth(1).unwrap_or_default().split_whitespace();
        let _filesystem = columns.next();
        let total = columns
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let _used = columns.next();
        let free = columns
            .next()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        Ok((total, free))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_external_mounts_from_proc_mounts() {
            let mounts =
                "/dev/sda1 / ext4 rw 0 0\n/dev/sdb1 /media/alice/USB\\040DISK exfat rw 0 0\n";
            let parsed: Vec<_> = parse_mounts(mounts)
                .into_iter()
                .filter(|item| looks_external_mount(&item.mount_path))
                .collect();

            assert_eq!(parsed.len(), 1);
            assert_eq!(parsed[0].mount_path, "/media/alice/USB DISK");
            assert_eq!(parsed[0].filesystem, "exfat");
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_storage {
    use super::*;

    pub struct MacOsStorageProvider;

    impl ExternalStorageProvider for MacOsStorageProvider {
        fn list_external_storage(&self) -> Result<Vec<ExternalStorageInfo>, String> {
            let entries =
                std::fs::read_dir("/Volumes").map_err(|e| format!("read /Volumes: {e}"))?;
            Ok(entries
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path().to_string_lossy().to_string();
                    if path == "/Volumes/Macintosh HD" {
                        return None;
                    }
                    Some(build_info(&path))
                })
                .collect())
        }

        fn get_storage_info(&self, path: &str) -> Result<ExternalStorageInfo, String> {
            let mount = if path.starts_with("/Volumes/") {
                path.split('/').take(3).collect::<Vec<_>>().join("/")
            } else {
                path.into()
            };
            Ok(build_info(&mount))
        }

        fn detection_source(&self) -> &'static str {
            "macos_volumes_poll_fallback"
        }
    }

    fn build_info(path: &str) -> ExternalStorageInfo {
        let (total_bytes, free_bytes) = df_bytes(path).unwrap_or((0, 0));
        ExternalStorageInfo {
            id: path.into(),
            name: storage_name_from_path(path),
            mount_path: path.into(),
            filesystem: "apfs".into(),
            total_bytes,
            free_bytes,
            removable: true,
            kind: StorageKind::Removable,
            platform: "macos".into(),
            detection_source: "volumes_dir".into(),
        }
    }

    fn df_bytes(path: &str) -> Result<(u64, u64), String> {
        let output = std::process::Command::new("df")
            .args(["-k", path])
            .output()
            .map_err(|e| format!("df failed: {e}"))?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut columns = text.lines().nth(1).unwrap_or_default().split_whitespace();
        let _filesystem = columns.next();
        let total = columns
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024;
        let _used = columns.next();
        let free = columns
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0)
            * 1024;
        Ok((total, free))
    }
}

pub mod windows_event_model {
    use super::*;

    pub const WM_DEVICECHANGE: u32 = 0x0219;
    pub const DBT_DEVICEARRIVAL: u32 = 0x8000;
    pub const DBT_DEVICEREMOVECOMPLETE: u32 = 0x8004;
    pub const DBT_DEVTYP_VOLUME: u32 = 0x0000_0002;

    pub fn event_from_devicechange(wparam: u32, unit_mask: u32) -> Option<StorageEvent> {
        let event_type = match wparam {
            DBT_DEVICEARRIVAL => StorageEventType::Attached,
            DBT_DEVICEREMOVECOMPLETE => StorageEventType::Detached,
            _ => return None,
        };
        let mount_path = first_drive_from_unit_mask(unit_mask);
        Some(StorageEvent {
            event_type,
            storage: None,
            mount_path,
            raw_code: Some(wparam),
            source: "wm_devicechange".into(),
        })
    }

    pub fn first_drive_from_unit_mask(unit_mask: u32) -> Option<String> {
        (0..26).find_map(|index| {
            if unit_mask & (1 << index) != 0 {
                Some(format!("{}:\\", (b'A' + index as u8) as char))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(path: &str) -> ExternalStorageInfo {
        ExternalStorageInfo {
            id: path.into(),
            name: storage_name_from_path(path),
            mount_path: path.into(),
            filesystem: "testfs".into(),
            total_bytes: 100,
            free_bytes: 50,
            removable: true,
            kind: StorageKind::Removable,
            platform: "test".into(),
            detection_source: "test".into(),
        }
    }

    #[test]
    fn diff_storage_sets_emits_attach_and_detach() {
        let previous = list_by_mount(vec![item("E:\\")]);
        let current = list_by_mount(vec![item("F:\\")]);
        let events = diff_storage_sets(&previous, &current, "test");

        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .any(|event| event.event_type == StorageEventType::Attached
                && event.mount_path.as_deref() == Some("F:\\")));
        assert!(events
            .iter()
            .any(|event| event.event_type == StorageEventType::Detached
                && event.mount_path.as_deref() == Some("E:\\")));
    }

    #[test]
    fn windows_devicechange_model_maps_unit_mask() {
        let event = windows_event_model::event_from_devicechange(
            windows_event_model::DBT_DEVICEARRIVAL,
            1 << 4,
        )
        .expect("arrival event");

        assert_eq!(event.event_type, StorageEventType::Attached);
        assert_eq!(event.mount_path.as_deref(), Some("E:\\"));
        assert_eq!(event.source, "wm_devicechange");
    }
}
