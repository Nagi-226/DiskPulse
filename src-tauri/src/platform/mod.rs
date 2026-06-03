pub mod common;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(all(target_os = "windows", feature = "mft-scanner"))]
pub mod windows_mft;

pub use common::{
    FileIdentity, PlatformProviders, PlatformSystemInfo, RestoreResult, TrashResult, WatcherGuard,
};

pub trait DiskInfoProvider: Send + Sync {
    fn total_bytes(&self, drive: &str) -> Result<u64, String>;
    fn free_bytes(&self, drive: &str) -> Result<u64, String>;
    fn list_drives(&self) -> Result<Vec<String>, String>;
    fn filesystem_type(&self, drive: &str) -> Result<String, String>;

    fn free_percent(&self, drive: &str) -> Result<f64, String> {
        let total = self.total_bytes(drive)?;
        if total == 0 {
            return Ok(0.0);
        }
        Ok((self.free_bytes(drive)? as f64 / total as f64) * 100.0)
    }
}

pub trait FsWatcher: Send + Sync {
    fn start(
        &self,
        config: crate::watcher::WatcherConfig,
        on_batch: Box<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
    ) -> Result<WatcherGuard, String>;
    fn stop(&self, guard: &mut WatcherGuard) -> Result<(), String>;
}

pub trait DirScanner: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> Result<crate::scanner::ScanOutput, String>;

    fn is_volume_streaming(&self) -> bool {
        false
    }

    fn execute_streaming(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> std::sync::mpsc::Receiver<crate::scanner::ScanBatch> {
        let stage = crate::scanner::MeasureStage;
        crate::scanner::ScanStage::execute_streaming(&stage, ctx)
    }
}

pub trait CleanupProvider: Send + Sync {
    fn move_to_trash(&self, path: &str) -> TrashResult;
    fn restore_from_trash(&self, original_path: &str) -> RestoreResult;
    fn is_available(&self) -> bool;
}

pub trait FileMetaAnalyzer: Send + Sync {
    fn hard_link_count(&self, path: &str) -> Result<u64, String>;
    fn is_sparse(&self, path: &str) -> Result<bool, String>;
    fn size_on_disk(&self, path: &str) -> Result<Option<u64>, String>;
    fn file_identity(&self, path: &str) -> Result<Option<FileIdentity>, String>;
}

pub trait SystemInfo: Send + Sync {
    fn os_name(&self) -> Result<String, String>;
    fn os_version(&self) -> Result<String, String>;
    fn cpu_count(&self) -> usize;
    fn total_ram_bytes(&self) -> Result<u64, String>;
    fn app_data_dir(&self) -> Result<String, String>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScanStrategy {
    Auto,
    Jwalk,
    #[cfg(all(target_os = "windows", feature = "mft-scanner"))]
    Mft {
        admin: bool,
    },
}

impl ScanStrategy {
    pub fn resolve(&self) -> Box<dyn DirScanner> {
        match self {
            Self::Auto => self.resolve_auto(),
            Self::Jwalk => platform_dir_scanner(),
            #[cfg(all(target_os = "windows", feature = "mft-scanner"))]
            Self::Mft { admin } => {
                if *admin {
                    Box::new(windows_mft::MftStage)
                } else {
                    platform_dir_scanner()
                }
            }
        }
    }

    fn resolve_auto(&self) -> Box<dyn DirScanner> {
        #[cfg(all(target_os = "windows", feature = "mft-scanner"))]
        {
            if windows_mft::check_admin_privilege() {
                return Box::new(windows_mft::MftStage);
            }
        }
        platform_dir_scanner()
    }
}

pub fn providers() -> PlatformProviders {
    PlatformProviders {
        disk_info: platform_disk_info(),
        fs_watcher: platform_fs_watcher(),
        dir_scanner: configured_scan_strategy().resolve(),
        cleanup: platform_cleanup(),
        file_meta: platform_file_meta(),
        system_info: platform_system_info(),
    }
}

fn configured_scan_strategy() -> ScanStrategy {
    match crate::db::get_settings().map(|settings| settings.scan_mode) {
        Ok(mode) if mode == "speed" => ScanStrategy::Auto,
        _ => ScanStrategy::Jwalk,
    }
}

#[cfg(target_os = "windows")]
fn platform_disk_info() -> Box<dyn DiskInfoProvider> {
    Box::new(windows::WindowsDiskInfoProvider)
}
#[cfg(target_os = "windows")]
fn platform_fs_watcher() -> Box<dyn FsWatcher> {
    Box::new(windows::WindowsNativeWatcher)
}
#[cfg(target_os = "windows")]
fn platform_dir_scanner() -> Box<dyn DirScanner> {
    Box::new(windows::JwalkStage)
}
#[cfg(target_os = "windows")]
fn platform_cleanup() -> Box<dyn CleanupProvider> {
    Box::new(windows::WindowsCleanupProvider)
}
#[cfg(target_os = "windows")]
fn platform_file_meta() -> Box<dyn FileMetaAnalyzer> {
    Box::new(windows::WindowsFileMetaAnalyzer)
}
#[cfg(target_os = "windows")]
fn platform_system_info() -> Box<dyn SystemInfo> {
    Box::new(windows::WindowsSystemProvider)
}

#[cfg(target_os = "linux")]
fn platform_disk_info() -> Box<dyn DiskInfoProvider> {
    Box::new(linux::LinuxDiskInfoProvider)
}
#[cfg(target_os = "linux")]
fn platform_fs_watcher() -> Box<dyn FsWatcher> {
    Box::new(linux::LinuxFsWatcher)
}
#[cfg(target_os = "linux")]
fn platform_dir_scanner() -> Box<dyn DirScanner> {
    Box::new(linux::LinuxWalkStage)
}
#[cfg(target_os = "linux")]
fn platform_cleanup() -> Box<dyn CleanupProvider> {
    Box::new(linux::LinuxCleanupProvider)
}
#[cfg(target_os = "linux")]
fn platform_file_meta() -> Box<dyn FileMetaAnalyzer> {
    Box::new(linux::LinuxFileMetaAnalyzer)
}
#[cfg(target_os = "linux")]
fn platform_system_info() -> Box<dyn SystemInfo> {
    Box::new(linux::LinuxSystemInfo)
}

#[cfg(target_os = "macos")]
fn platform_disk_info() -> Box<dyn DiskInfoProvider> {
    Box::new(macos::MacOsDiskInfoProvider)
}
#[cfg(target_os = "macos")]
fn platform_fs_watcher() -> Box<dyn FsWatcher> {
    Box::new(macos::MacOsFsWatcher)
}
#[cfg(target_os = "macos")]
fn platform_dir_scanner() -> Box<dyn DirScanner> {
    Box::new(macos::MacOsWalkStage)
}
#[cfg(target_os = "macos")]
fn platform_cleanup() -> Box<dyn CleanupProvider> {
    Box::new(macos::MacOsCleanupProvider)
}
#[cfg(target_os = "macos")]
fn platform_file_meta() -> Box<dyn FileMetaAnalyzer> {
    Box::new(macos::MacOsFileMetaAnalyzer)
}
#[cfg(target_os = "macos")]
fn platform_system_info() -> Box<dyn SystemInfo> {
    Box::new(macos::MacOsSystemInfo)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeDiskInfo;
    impl DiskInfoProvider for FakeDiskInfo {
        fn total_bytes(&self, _drive: &str) -> Result<u64, String> {
            Ok(100)
        }
        fn free_bytes(&self, _drive: &str) -> Result<u64, String> {
            Ok(42)
        }
        fn list_drives(&self) -> Result<Vec<String>, String> {
            Ok(vec!["T".into()])
        }
        fn filesystem_type(&self, _drive: &str) -> Result<String, String> {
            Ok("fakefs".into())
        }
    }

    #[test]
    fn disk_info_provider_trait_is_swappable() {
        let provider = FakeDiskInfo;
        assert_eq!(provider.free_percent("C").unwrap(), 42.0);
    }

    #[test]
    fn platform_providers_construct_with_trait_objects() {
        let providers = providers();
        assert!(providers.cleanup.is_available());
        assert!(!providers.dir_scanner.name().is_empty());
    }

    #[cfg(all(target_os = "windows", feature = "mft-scanner"))]
    #[test]
    fn mft_strategy_requires_admin_flag() {
        assert_eq!(ScanStrategy::Mft { admin: true }.resolve().name(), "mft");
        assert_eq!(ScanStrategy::Mft { admin: false }.resolve().name(), "jwalk");
    }
}
