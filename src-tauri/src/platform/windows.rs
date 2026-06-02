use crate::platform::{
    CleanupProvider, DirScanner, DiskInfoProvider, FileIdentity, FileMetaAnalyzer, FsWatcher,
    RestoreResult, SystemInfo, TrashResult, WatcherGuard,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct WindowsDiskInfoProvider;
pub struct WindowsNativeWatcher;
pub struct WindowsPollWatcher;
pub struct JwalkStage;
pub struct WindowsCleanupProvider;
pub struct WindowsFileMetaAnalyzer;
pub struct WindowsSystemProvider;

impl DiskInfoProvider for WindowsDiskInfoProvider {
    fn total_bytes(&self, drive: &str) -> Result<u64, String> {
        Ok(drive_space(drive)?.0)
    }

    fn free_bytes(&self, drive: &str) -> Result<u64, String> {
        Ok(drive_space(drive)?.1)
    }

    fn list_drives(&self) -> Result<Vec<String>, String> {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetLogicalDrives;

        unsafe {
            let drives_mask = GetLogicalDrives();
            if drives_mask == 0 {
                return Err("Failed to get logical drives".into());
            }

            let mut drives = Vec::new();
            for i in 0..26 {
                if drives_mask & (1 << i) != 0 {
                    let letter = (b'A' + i) as char;
                    let path = format!("{}:\\", letter);
                    let wide: Vec<u16> = std::ffi::OsStr::new(&path)
                        .encode_wide()
                        .chain(std::iter::once(0))
                        .collect();
                    let result = windows::Win32::Storage::FileSystem::GetDriveTypeW(
                        windows::core::PCWSTR(wide.as_ptr()),
                    );
                    if result != 1 {
                        drives.push(letter.to_string());
                    }
                }
            }
            Ok(drives)
        }
    }

    fn filesystem_type(&self, drive: &str) -> Result<String, String> {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetVolumeInformationW;

        let root = normalize_drive_root(drive);
        let wide: Vec<u16> = std::ffi::OsStr::new(&root)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut fs_name = [0u16; 64];
        let result = unsafe {
            GetVolumeInformationW(
                windows::core::PCWSTR(wide.as_ptr()),
                None,
                None,
                None,
                None,
                Some(&mut fs_name),
            )
        };
        if result.is_err() {
            return Err(format!("Cannot read filesystem type for {}", drive));
        }
        let len = fs_name
            .iter()
            .position(|value| *value == 0)
            .unwrap_or(fs_name.len());
        Ok(String::from_utf16_lossy(&fs_name[..len]))
    }
}

impl FsWatcher for WindowsPollWatcher {
    fn start(
        &self,
        config: crate::watcher::WatcherConfig,
        on_batch: Box<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
    ) -> Result<WatcherGuard, String> {
        Ok(WatcherGuard::from_polling(crate::watcher::start_watching(
            config,
            move |batch| {
                on_batch(batch);
            },
        )))
    }

    fn stop(&self, guard: &mut WatcherGuard) -> Result<(), String> {
        guard.stop();
        Ok(())
    }
}

impl FsWatcher for WindowsNativeWatcher {
    fn start(
        &self,
        mut config: crate::watcher::WatcherConfig,
        on_batch: Box<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
    ) -> Result<WatcherGuard, String> {
        let on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static> =
            Arc::from(on_batch);
        let debounce_ms = config.debounce_ms.min(500);
        let mut native_handles = Vec::new();
        let mut stop_events = Vec::new();
        let mut fallback_dirs = Vec::new();

        for dir in config
            .directories
            .iter()
            .filter(|dir| Path::new(dir).exists())
        {
            match start_native_dir_watch(dir.clone(), debounce_ms, on_batch.clone()) {
                Ok((stop_event, handle)) => {
                    stop_events.push(stop_event.0 as usize);
                    native_handles.push(handle);
                }
                Err(_) => fallback_dirs.push(dir.clone()),
            }
        }

        let fallback_guard = if fallback_dirs.is_empty() {
            None
        } else {
            config.directories = fallback_dirs;
            Some(crate::watcher::start_watching(config, move |batch| {
                on_batch(batch);
            }))
        };

        if native_handles.is_empty() {
            return fallback_guard
                .map(WatcherGuard::from_polling)
                .map(Ok)
                .unwrap_or_else(|| Ok(WatcherGuard::empty()));
        }

        Ok(WatcherGuard::new(
            move || {
                for raw in stop_events {
                    unsafe {
                        let _ = windows::Win32::System::Threading::SetEvent(
                            windows::Win32::Foundation::HANDLE(raw as *mut _),
                        );
                    }
                }
                drop(fallback_guard);
            },
            native_handles,
        ))
    }

    fn stop(&self, guard: &mut WatcherGuard) -> Result<(), String> {
        guard.stop();
        Ok(())
    }
}

impl DirScanner for JwalkStage {
    fn name(&self) -> &'static str {
        "jwalk"
    }

    fn execute(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> Result<crate::scanner::ScanOutput, String> {
        let stage = crate::scanner::MeasureStage;
        crate::scanner::ScanStage::execute(&stage, ctx)
    }
}

impl CleanupProvider for WindowsCleanupProvider {
    fn move_to_trash(&self, path: &str) -> TrashResult {
        let success = crate::cleaner::recycle_bin_delete(path);
        TrashResult {
            success,
            original_path: path.to_string(),
            reason: if success {
                None
            } else {
                Some("Recycle Bin operation failed".into())
            },
        }
    }

    fn restore_from_trash(&self, original_path: &str) -> RestoreResult {
        let result = crate::cleaner::restore_items(vec![original_path.to_string()]);
        let success = result.restored > 0;
        RestoreResult {
            success,
            original_path: original_path.to_string(),
            restored_path: if success {
                Some(original_path.to_string())
            } else {
                None
            },
            reason: if success {
                None
            } else {
                Some("No matching Recycle Bin item found".into())
            },
        }
    }

    fn is_available(&self) -> bool {
        true
    }
}

impl FileMetaAnalyzer for WindowsFileMetaAnalyzer {
    fn hard_link_count(&self, path: &str) -> Result<u64, String> {
        file_basic_info(path).map(|info| info.0)
    }

    fn is_sparse(&self, path: &str) -> Result<bool, String> {
        file_basic_info(path).map(|info| info.3)
    }

    fn size_on_disk(&self, path: &str) -> Result<Option<u64>, String> {
        compressed_file_size(path).map(Some)
    }

    fn file_identity(&self, path: &str) -> Result<Option<FileIdentity>, String> {
        let (_links, volume_serial, file_index, _sparse) = file_basic_info(path)?;
        Ok(Some(FileIdentity {
            volume_serial,
            file_index,
        }))
    }
}

impl SystemInfo for WindowsSystemProvider {
    fn os_name(&self) -> Result<String, String> {
        Ok("Windows".into())
    }

    fn os_version(&self) -> Result<String, String> {
        Ok(std::env::var("OS").unwrap_or_else(|_| "Windows".into()))
    }

    fn cpu_count(&self) -> usize {
        std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1)
    }

    fn total_ram_bytes(&self) -> Result<u64, String> {
        Ok(0)
    }

    fn app_data_dir(&self) -> Result<String, String> {
        std::env::var("APPDATA")
            .or_else(|_| std::env::var("LOCALAPPDATA"))
            .map_err(|e| format!("Cannot locate app data dir: {}", e))
    }
}

fn normalize_drive_root(drive: &str) -> String {
    let trimmed = drive.trim().trim_end_matches(['\\', '/']);
    if trimmed.ends_with(':') {
        format!("{}\\", trimmed.to_uppercase())
    } else if trimmed.len() == 1 {
        format!("{}:\\", trimmed.to_uppercase())
    } else {
        trimmed.to_string()
    }
}

fn drive_space(drive: &str) -> Result<(u64, u64), String> {
    use std::os::windows::ffi::OsStrExt;
    let root = normalize_drive_root(drive);
    let wide: Vec<u16> = std::ffi::OsStr::new(&root)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_bytes_available: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free_bytes: u64 = 0;

    unsafe {
        windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            windows::core::PCWSTR(wide.as_ptr()),
            Some(&mut free_bytes_available as *mut u64 as *mut _),
            Some(&mut total_bytes as *mut u64 as *mut _),
            Some(&mut total_free_bytes as *mut u64 as *mut _),
        )
        .map_err(|e| format!("GetDiskFreeSpaceExW failed for {}: {}", root, e))?;
    }

    Ok((total_bytes, free_bytes_available))
}

fn file_basic_info(path: &str) -> Result<(u64, u64, u64, bool), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        FILE_ATTRIBUTE_SPARSE_FILE, FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_READ,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let handle = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            None,
        )
    }
    .map_err(|e| format!("Open file metadata error: {}", e))?;

    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    let result = unsafe { GetFileInformationByHandle(handle, &mut info) };
    unsafe {
        CloseHandle(handle).ok();
    }
    result.map_err(|e| format!("GetFileInformationByHandle error: {}", e))?;

    let file_index = ((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64;
    let is_sparse = (info.dwFileAttributes & FILE_ATTRIBUTE_SPARSE_FILE.0) != 0;
    Ok((
        info.nNumberOfLinks as u64,
        info.dwVolumeSerialNumber as u64,
        file_index,
        is_sparse,
    ))
}

fn compressed_file_size(path: &str) -> Result<u64, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetCompressedFileSizeW;
    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut high = 0u32;
    let low =
        unsafe { GetCompressedFileSizeW(windows::core::PCWSTR(wide.as_ptr()), Some(&mut high)) };
    if low == u32::MAX {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error().unwrap_or(0) != 0 {
            return Err(format!("GetCompressedFileSizeW error: {}", err));
        }
    }
    Ok(((high as u64) << 32) | low as u64)
}

fn start_native_dir_watch(
    dir: String,
    debounce_ms: u64,
    on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
) -> Result<(windows::Win32::Foundation::HANDLE, thread::JoinHandle<()>), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{BOOL, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::Win32::System::Threading::CreateEventW;

    let wide: Vec<u16> = std::ffi::OsStr::new(&dir)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let dir_handle = unsafe {
        CreateFileW(
            windows::core::PCWSTR(wide.as_ptr()),
            FILE_LIST_DIRECTORY.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED,
            None,
        )
    }
    .map_err(|e| format!("ReadDirectoryChangesW open failed for {dir}: {e}"))?;

    let stop_event = unsafe { CreateEventW(None, BOOL(1), BOOL(0), windows::core::PCWSTR::null()) }
        .map_err(|e| {
            unsafe {
                windows::Win32::Foundation::CloseHandle(dir_handle).ok();
            }
            format!("CreateEventW stop event failed for {dir}: {e}")
        })?;

    let (ready_tx, ready_rx) = mpsc::channel();
    let dir_handle_raw = dir_handle.0 as usize;
    let stop_event_raw = stop_event.0 as usize;
    let handle = thread::Builder::new()
        .name(format!(
            "diskpulse-rdcw-{}",
            dir.replace(['\\', ':', '/'], "_")
                .chars()
                .take(30)
                .collect::<String>()
        ))
        .spawn(move || {
            run_native_dir_watch(
                dir,
                HANDLE(dir_handle_raw as *mut _),
                HANDLE(stop_event_raw as *mut _),
                debounce_ms,
                on_batch,
                Some(ready_tx),
            );
        })
        .map_err(|e| {
            unsafe {
                windows::Win32::Foundation::CloseHandle(dir_handle).ok();
                windows::Win32::Foundation::CloseHandle(stop_event).ok();
            }
            format!("spawn native watcher failed: {e}")
        })?;

    let _ = ready_rx.recv_timeout(Duration::from_millis(500));
    Ok((stop_event, handle))
}

fn run_native_dir_watch(
    watched_dir: String,
    dir_handle: windows::Win32::Foundation::HANDLE,
    stop_event: windows::Win32::Foundation::HANDLE,
    debounce_ms: u64,
    on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
    mut ready: Option<mpsc::Sender<()>>,
) {
    use windows::Win32::Foundation::{CloseHandle, BOOL, WAIT_OBJECT_0};
    use windows::Win32::Storage::FileSystem::{
        ReadDirectoryChangesW, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE,
        FILE_NOTIFY_CHANGE_SIZE,
    };
    use windows::Win32::System::Threading::{
        CreateEventW, ResetEvent, WaitForMultipleObjects, INFINITE,
    };
    use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

    let io_event =
        match unsafe { CreateEventW(None, BOOL(1), BOOL(0), windows::core::PCWSTR::null()) } {
            Ok(event) => event,
            Err(_) => {
                unsafe {
                    CloseHandle(dir_handle).ok();
                    CloseHandle(stop_event).ok();
                }
                return;
            }
        };

    let mut buffer = vec![0u8; 64 * 1024];
    let filter =
        FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_SIZE | FILE_NOTIFY_CHANGE_LAST_WRITE;
    let flush_after = Duration::from_millis(debounce_ms);
    let mut pending = Vec::new();
    let mut first_pending_at: Option<Instant> = None;

    loop {
        unsafe {
            let _ = ResetEvent(io_event);
        }
        let mut overlapped = OVERLAPPED {
            hEvent: io_event,
            ..Default::default()
        };
        let mut immediate_bytes = 0u32;
        let read_result = unsafe {
            ReadDirectoryChangesW(
                dir_handle,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                BOOL(1),
                filter,
                Some(&mut immediate_bytes),
                Some(&mut overlapped),
                None,
            )
        };

        if let Err(error) = read_result {
            if !is_win32_error(&error, windows::Win32::Foundation::ERROR_IO_PENDING.0) {
                break;
            }
        }
        if let Some(sender) = ready.take() {
            let _ = sender.send(());
        }

        let wait = unsafe { WaitForMultipleObjects(&[stop_event, io_event], false, INFINITE) };
        if wait == WAIT_OBJECT_0 {
            unsafe {
                let _ = CancelIoEx(dir_handle, Some(&overlapped));
            }
            break;
        }
        if wait != windows::Win32::Foundation::WAIT_EVENT(WAIT_OBJECT_0.0 + 1) {
            break;
        }

        let mut bytes = 0u32;
        let result = unsafe { GetOverlappedResult(dir_handle, &overlapped, &mut bytes, false) };
        if result.is_err() || bytes == 0 {
            continue;
        }

        let events = parse_native_notify_buffer(&watched_dir, &buffer[..bytes as usize]);
        if !events.is_empty() {
            if first_pending_at.is_none() {
                first_pending_at = Some(Instant::now());
            }
            pending.extend(events);
        }

        let should_flush = !pending.is_empty()
            && (flush_after <= Duration::from_millis(50)
                || first_pending_at.is_some_and(|started| started.elapsed() >= flush_after));
        if should_flush {
            flush_native_events(&watched_dir, &mut pending, &on_batch);
            first_pending_at = None;
        }
    }

    flush_native_events(&watched_dir, &mut pending, &on_batch);
    unsafe {
        CloseHandle(io_event).ok();
        CloseHandle(dir_handle).ok();
        CloseHandle(stop_event).ok();
    }
}

fn parse_native_notify_buffer(watched_dir: &str, buffer: &[u8]) -> Vec<crate::watcher::FsEvent> {
    use windows::Win32::Storage::FileSystem::{
        FILE_ACTION_ADDED, FILE_ACTION_REMOVED, FILE_ACTION_RENAMED_NEW_NAME,
        FILE_ACTION_RENAMED_OLD_NAME, FILE_NOTIFY_INFORMATION,
    };

    let mut events = Vec::new();
    let mut offset = 0usize;
    while offset + std::mem::size_of::<FILE_NOTIFY_INFORMATION>() <= buffer.len() {
        let info = unsafe {
            &*(buffer
                .as_ptr()
                .add(offset)
                .cast::<FILE_NOTIFY_INFORMATION>())
        };
        let name_bytes = info.FileNameLength as usize;
        let name_len = name_bytes / std::mem::size_of::<u16>();
        let name_ptr = info.FileName.as_ptr();
        let name =
            String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(name_ptr, name_len) });
        let full_path = Path::new(watched_dir)
            .join(name)
            .to_string_lossy()
            .to_string();
        let metadata = std::fs::metadata(&full_path).ok();
        let kind = if info.Action == FILE_ACTION_REMOVED
            || info.Action == FILE_ACTION_RENAMED_OLD_NAME
        {
            crate::watcher::FsEventKind::Removed
        } else if info.Action == FILE_ACTION_ADDED || info.Action == FILE_ACTION_RENAMED_NEW_NAME {
            crate::watcher::FsEventKind::Added
        } else {
            crate::watcher::FsEventKind::Modified
        };
        events.push(crate::watcher::FsEvent {
            kind,
            path: full_path,
            is_directory: metadata.as_ref().is_some_and(|m| m.is_dir()),
            size_bytes: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
            previous_size_bytes: None,
        });

        if info.NextEntryOffset == 0 {
            break;
        }
        offset += info.NextEntryOffset as usize;
    }
    events
}

fn flush_native_events(
    watched_dir: &str,
    pending: &mut Vec<crate::watcher::FsEvent>,
    on_batch: &Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
) {
    if pending.is_empty() {
        return;
    }
    let count = pending.len();
    on_batch(crate::watcher::FsChangeBatch {
        watched_dir: watched_dir.to_string(),
        events: std::mem::take(pending),
        event_count: count,
        timestamp_ms: now_ms(),
    });
}

fn is_win32_error(error: &windows::core::Error, code: u32) -> bool {
    (error.code().0 as u32 & 0xffff) == code
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[allow(dead_code)]
fn _pathbuf(path: &Path) -> PathBuf {
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::FsWatcher;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn native_watcher_guard_drop_stops_thread() {
        let dir = unique_temp_dir("native-stop");
        std::fs::create_dir_all(&dir).unwrap();
        let (tx, _rx) = mpsc::channel();
        let watcher = WindowsNativeWatcher;
        let guard = watcher
            .start(
                crate::watcher::WatcherConfig {
                    directories: vec![dir.to_string_lossy().into_owned()],
                    poll_interval_ms: 2000,
                    debounce_ms: 25,
                },
                Box::new(move |batch| {
                    let _ = tx.send(batch);
                }),
            )
            .unwrap();

        drop(guard);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn native_watcher_reports_file_create_quickly() {
        let dir = unique_temp_dir("native-event");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("created.txt");
        let (tx, rx) = mpsc::channel();
        let watcher = WindowsNativeWatcher;
        let mut guard = watcher
            .start(
                crate::watcher::WatcherConfig {
                    directories: vec![dir.to_string_lossy().into_owned()],
                    poll_interval_ms: 2000,
                    debounce_ms: 25,
                },
                Box::new(move |batch| {
                    let _ = tx.send(batch);
                }),
            )
            .unwrap();

        std::fs::write(&file_path, b"hello").unwrap();
        let batch = rx.recv_timeout(Duration::from_secs(1)).unwrap();
        watcher.stop(&mut guard).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(batch
            .events
            .iter()
            .any(|event| event.path.ends_with("created.txt")));
    }

    #[test]
    fn sparse_file_metadata_reports_allocated_size() {
        let dir = unique_temp_dir("sparse");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("sparse.bin");
        std::fs::write(&file_path, b"x").unwrap();
        make_sparse(&file_path).unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&file_path)
            .unwrap();
        file.set_len(16 * 1024 * 1024).unwrap();

        let analyzer = WindowsFileMetaAnalyzer;
        let path = file_path.to_string_lossy();
        let is_sparse = analyzer.is_sparse(&path).unwrap();
        let size_on_disk = analyzer.size_on_disk(&path).unwrap().unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(is_sparse);
        assert!(size_on_disk < 16 * 1024 * 1024);
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "diskpulse-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn make_sparse(path: &Path) -> Result<(), String> {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::Storage::FileSystem::{
            CreateFileW, FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_WRITE, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        };
        use windows::Win32::System::Ioctl::FSCTL_SET_SPARSE;
        use windows::Win32::System::IO::DeviceIoControl;

        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe {
            CreateFileW(
                windows::core::PCWSTR(wide.as_ptr()),
                FILE_GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                None,
            )
        }
        .map_err(|e| format!("open sparse test file: {e}"))?;

        let mut returned = 0u32;
        let result = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_SET_SPARSE,
                None,
                0,
                None,
                0,
                Some(&mut returned),
                None,
            )
        };
        unsafe {
            CloseHandle(handle).ok();
        }
        result.map_err(|e| format!("FSCTL_SET_SPARSE failed: {e}"))
    }
}
