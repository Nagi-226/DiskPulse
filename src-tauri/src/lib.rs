mod risk;
mod scanner;

use scanner::DriveInfo;
use tauri::{AppHandle, Emitter};

pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";

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
                // Check if drive exists
                let path = format!("{}:\\", letter);
                let wide: Vec<u16> = std::ffi::OsStr::new(&path)
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();
                let result = windows::Win32::Storage::FileSystem::GetDriveTypeW(
                    windows::core::PCWSTR(wide.as_ptr()),
                );
                // Include fixed drives, removable drives, and network drives
                if result != 1 {
                    // DRIVE_NO_ROOT_DIR = 1, skip non-existent
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
        .invoke_handler(tauri::generate_handler![scan_drive, list_drives, scan_directory, classify_risks, app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
