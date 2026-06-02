use crate::platform::{
    CleanupProvider, DirScanner, DiskInfoProvider, FileIdentity, FileMetaAnalyzer, FsWatcher,
    RestoreResult, SystemInfo, TrashResult, WatcherGuard,
};
use std::os::unix::fs::MetadataExt;

pub struct MacOsDiskInfoProvider;
pub struct MacOsFsWatcher;
pub struct MacOsWalkStage;
pub struct MacOsCleanupProvider;
pub struct MacOsFileMetaAnalyzer;
pub struct MacOsSystemInfo;

impl DiskInfoProvider for MacOsDiskInfoProvider {
    fn total_bytes(&self, drive: &str) -> Result<u64, String> {
        df_bytes(drive, 1)
    }
    fn free_bytes(&self, drive: &str) -> Result<u64, String> {
        df_bytes(drive, 3)
    }
    fn list_drives(&self) -> Result<Vec<String>, String> {
        Ok(vec!["/".into(), "/Volumes".into()])
    }
    fn filesystem_type(&self, _drive: &str) -> Result<String, String> {
        Ok("apfs".into())
    }
}

impl FsWatcher for MacOsFsWatcher {
    fn start(
        &self,
        config: crate::watcher::WatcherConfig,
        on_batch: Box<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
    ) -> Result<WatcherGuard, String> {
        Ok(WatcherGuard::from_polling(crate::watcher::start_watching(
            config,
            move |batch| on_batch(batch),
        )))
    }
    fn stop(&self, guard: &mut WatcherGuard) -> Result<(), String> {
        guard.stop();
        Ok(())
    }
}

impl DirScanner for MacOsWalkStage {
    fn name(&self) -> &'static str {
        "macos-walk"
    }
    fn execute(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> Result<crate::scanner::ScanOutput, String> {
        let stage = crate::scanner::MeasureStage;
        crate::scanner::ScanStage::execute(&stage, ctx)
    }
}

impl CleanupProvider for MacOsCleanupProvider {
    fn move_to_trash(&self, path: &str) -> TrashResult {
        let script = format!(
            "tell application \"Finder\" to delete POSIX file \"{}\"",
            path.replace('"', "\\\"")
        );
        let success = std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        TrashResult {
            success,
            original_path: path.into(),
            reason: if success {
                None
            } else {
                Some("osascript trash failed".into())
            },
        }
    }
    fn restore_from_trash(&self, original_path: &str) -> RestoreResult {
        RestoreResult {
            success: false,
            original_path: original_path.into(),
            restored_path: None,
            reason: Some("macOS trash restore is not implemented".into()),
        }
    }
    fn is_available(&self) -> bool {
        true
    }
}

impl FileMetaAnalyzer for MacOsFileMetaAnalyzer {
    fn hard_link_count(&self, path: &str) -> Result<u64, String> {
        unix_nlink(path)
    }
    fn is_sparse(&self, path: &str) -> Result<bool, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("metadata: {e}"))?;
        Ok(self
            .size_on_disk(path)?
            .is_some_and(|disk| disk < meta.len()))
    }
    fn size_on_disk(&self, path: &str) -> Result<Option<u64>, String> {
        Ok(Some(
            std::fs::metadata(path)
                .map_err(|e| format!("metadata: {e}"))?
                .blocks()
                * 512,
        ))
    }
    fn file_identity(&self, path: &str) -> Result<Option<FileIdentity>, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("metadata: {e}"))?;
        Ok(Some(FileIdentity {
            volume_serial: meta.dev(),
            file_index: meta.ino(),
        }))
    }
}

impl SystemInfo for MacOsSystemInfo {
    fn os_name(&self) -> Result<String, String> {
        Ok("macOS".into())
    }
    fn os_version(&self) -> Result<String, String> {
        let output = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_err(|e| format!("sw_vers failed: {e}"))?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }
    fn cpu_count(&self) -> usize {
        std::thread::available_parallelism()
            .map(|c| c.get())
            .unwrap_or(1)
    }
    fn total_ram_bytes(&self) -> Result<u64, String> {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .map_err(|e| format!("sysctl hw.memsize failed: {e}"))?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(0))
    }
    fn app_data_dir(&self) -> Result<String, String> {
        std::env::var("HOME")
            .map(|home| format!("{home}/Library/Application Support"))
            .map_err(|e| format!("app data dir: {e}"))
    }
}

fn df_bytes(path: &str, column: usize) -> Result<u64, String> {
    let output = std::process::Command::new("df")
        .args(["-k", path])
        .output()
        .map_err(|e| format!("df failed: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(column))
        .and_then(|value| value.parse::<u64>().ok())
        .map(|kb| kb * 1024)
        .ok_or_else(|| "cannot parse df output".into())
}

#[cfg(unix)]
fn unix_nlink(path: &str) -> Result<u64, String> {
    Ok(std::fs::metadata(path)
        .map_err(|e| format!("metadata: {e}"))?
        .nlink())
}
