use serde::{Deserialize, Serialize};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FileIdentity {
    pub volume_serial: u64,
    pub file_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrashResult {
    pub success: bool,
    pub original_path: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RestoreResult {
    pub success: bool,
    pub original_path: String,
    pub restored_path: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlatformSystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub cpu_count: usize,
    pub total_ram_bytes: u64,
    pub app_data_dir: String,
}

pub struct WatcherGuard {
    stop_fn: Option<Box<dyn FnOnce() + Send + 'static>>,
    handles: Vec<thread::JoinHandle<()>>,
}

impl WatcherGuard {
    pub fn new<F>(stop_fn: F, handles: Vec<thread::JoinHandle<()>>) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            stop_fn: Some(Box::new(stop_fn)),
            handles,
        }
    }

    pub fn from_polling(inner: crate::watcher::WatcherGuard) -> Self {
        Self::new(move || drop(inner), Vec::new())
    }

    pub fn empty() -> Self {
        Self::new(|| {}, Vec::new())
    }

    pub fn stop(&mut self) {
        if let Some(stop_fn) = self.stop_fn.take() {
            stop_fn();
        }
        for handle in self.handles.drain(..) {
            let _ = handle.join();
        }
    }
}

impl Drop for WatcherGuard {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct PlatformProviders {
    pub disk_info: Box<dyn crate::platform::DiskInfoProvider>,
    pub fs_watcher: Box<dyn crate::platform::FsWatcher>,
    pub dir_scanner: Box<dyn crate::platform::DirScanner>,
    pub cleanup: Box<dyn crate::platform::CleanupProvider>,
    pub file_meta: Box<dyn crate::platform::FileMetaAnalyzer>,
    pub system_info: Box<dyn crate::platform::SystemInfo>,
}
