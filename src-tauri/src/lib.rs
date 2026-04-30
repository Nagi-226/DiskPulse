mod cleaner;
mod risk;
mod scanner;
mod watcher;

use cleaner::{CleanItem, CleanPreview, CleanResult};
use scanner::DriveInfo;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";
pub const CLEAN_PROGRESS_EVENT: &str = "clean-progress";
pub const FS_EVENT_BATCH: &str = "fs-event-batch";

/// Global watcher guard so we can stop it from another command.
static WATCHER: Mutex<Option<watcher::WatcherGuard>> = Mutex::new(None);

/// Scan a drive and emit progress events
#[tauri::command]
fn scan_drive(app: AppHandle, drive: String) -> Result<DriveInfo, String> {
    scanner::scan_drive_with_progress(&drive, |progress| {
        let _ = app.emit(SCAN_PROGRESS_EVENT, progress);
    })
}

/// List available drives on the system
#[tauri::command]
fn list_drives() -> Result<Vec<String>, String> {
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

/// Scan a specific directory for drill-down navigation
#[tauri::command]
fn scan_directory(path: String) -> Result<Vec<scanner::DirInfo>, String> {
    scanner::scan_directory(&path)
}

/// Classify scan results into risk levels
#[tauri::command]
fn classify_risks(scan: DriveInfo) -> Result<risk::RiskReport, String> {
    Ok(risk::classify_risks(&scan))
}

/// Preview cleanup candidates with whitelist validation and safety checks.
#[tauri::command]
fn preview_cleanup(items: Vec<CleanItem>) -> Result<CleanPreview, String> {
    Ok(cleaner::preview_cleanup(items))
}

/// Execute cleanup with progress events emitted to the frontend.
#[tauri::command]
fn clean_items(app: AppHandle, items: Vec<CleanItem>) -> Result<CleanResult, String> {
    let handle = app.clone();
    let result = cleaner::clean_items_with_progress(items, None, move |progress| {
        let _ = handle.emit(CLEAN_PROGRESS_EVENT, progress);
    });
    Ok(result)
}

/// Start the file system watcher on default directories.
/// Emits `fs-event-batch` events to the frontend with aggregated changes.
#[tauri::command]
fn start_fs_watcher(app: AppHandle) -> Result<String, String> {
    let mut guard = WATCHER.lock().map_err(|e| format!("Lock error: {}", e))?;
    if guard.is_some() {
        return Ok("watcher already running".into());
    }

    let config = watcher::WatcherConfig::default();
    let dir_list = config.directories.join(", ");
    let handle = app.clone();

    let watcher_guard = watcher::start_watching(config, move |batch| {
        let _ = handle.emit(FS_EVENT_BATCH, batch);
    });

    *guard = Some(watcher_guard);

    Ok(format!("watching: {}", dir_list))
}

/// Stop the file system watcher.
#[tauri::command]
fn stop_fs_watcher() -> Result<String, String> {
    let mut guard = WATCHER.lock().map_err(|e| format!("Lock error: {}", e))?;
    if let Some(w) = guard.take() {
        w.stop();
        Ok("watcher stopped".into())
    } else {
        Ok("no watcher running".into())
    }
}

/// Get the app version
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            scan_drive,
            list_drives,
            scan_directory,
            classify_risks,
            preview_cleanup,
            clean_items,
            start_fs_watcher,
            stop_fs_watcher,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
