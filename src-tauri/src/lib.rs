mod cleaner;
mod db;
mod risk;
mod scanner;
mod watcher;

use cleaner::{CleanItem, CleanPreview, CleanResult, RestoreResult};
use scanner::DriveInfo;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";
pub const CLEAN_PROGRESS_EVENT: &str = "clean-progress";
pub const FS_EVENT_BATCH: &str = "fs-event-batch";
pub const AUTO_SCAN_EVENT: &str = "auto-scan";

/// Global watcher guard so we can stop it from another command.
static WATCHER: Mutex<Option<watcher::WatcherGuard>> = Mutex::new(None);

/// Scan a drive and emit progress events
#[tauri::command]
fn scan_drive(app: AppHandle, drive: String) -> Result<DriveInfo, String> {
    let info = scanner::scan_drive_with_progress(&drive, |progress| {
        let _ = app.emit(SCAN_PROGRESS_EVENT, progress);
    })?;
    if let Err(e) = db::save_snapshot(&info) {
        eprintln!("Failed to save snapshot to history DB: {}", e);
    }
    Ok(info)
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
    if let Err(e) = db::save_cleanup_log(&result) {
        eprintln!("Failed to save cleanup log to history DB: {}", e);
    }
    Ok(result)
}

/// Attempt to restore previously cleaned items from the Recycle Bin.
#[tauri::command]
fn undo_cleanup(original_paths: Vec<String>) -> Result<RestoreResult, String> {
    Ok(cleaner::restore_items(original_paths))
}

/// Start the file system watcher on default directories.
#[tauri::command]
fn start_fs_watcher(app: AppHandle) -> Result<String, String> {
    let mut guard = WATCHER.lock().map_err(|e| format!("Lock error: {}", e))?;
    if guard.is_some() {
        return Ok("watcher already running".into());
    }

    let settings = db::get_settings().unwrap_or_default();
    let config = watcher::WatcherConfig {
        directories: watcher::default_watch_dirs(),
        poll_interval_ms: settings.watcher_poll_interval_ms,
        debounce_ms: settings.watcher_debounce_ms,
    };
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

/// Get application settings.
#[tauri::command]
fn get_settings() -> Result<db::AppSettings, String> {
    db::get_settings()
}

/// Save application settings.
#[tauri::command]
fn save_settings(settings: db::AppSettings) -> Result<(), String> {
    db::save_settings(&settings)
}

/// Get all risk rules with user overrides applied.
#[tauri::command]
fn get_rules() -> Result<Vec<risk::RiskRule>, String> {
    let overrides = db::get_rule_overrides()?;
    Ok(risk::get_rules_with_overrides(&overrides))
}

/// Save a single rule override.
#[tauri::command]
fn save_rule_override(rule_id: String, safe_to_delete: bool) -> Result<(), String> {
    db::save_rule_override(&rule_id, safe_to_delete)
}

/// Get snapshot history for a drive within the specified number of past days.
#[tauri::command]
fn get_snapshot_history(drive: String, days: u32) -> Result<Vec<db::Snapshot>, String> {
    db::get_snapshot_history(&drive, days)
}

/// Get all cleanup operation history.
#[tauri::command]
fn get_cleanup_history() -> Result<Vec<db::CleanupLog>, String> {
    db::get_cleanup_history()
}

/// Get the app version
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Internal: start watcher without command signature (used by auto-startup).
fn start_fs_watcher_internal(app: &AppHandle) -> Result<String, String> {
    let mut guard = WATCHER.lock().map_err(|e| format!("Lock error: {}", e))?;
    if guard.is_some() {
        return Ok("watcher already running".into());
    }
    let settings = db::get_settings().unwrap_or_default();
    let config = watcher::WatcherConfig {
        directories: watcher::default_watch_dirs(),
        poll_interval_ms: settings.watcher_poll_interval_ms,
        debounce_ms: settings.watcher_debounce_ms,
    };
    let dir_list = config.directories.join(", ");
    let handle = app.clone();
    let watcher_guard = watcher::start_watching(config, move |batch| {
        let _ = handle.emit(FS_EVENT_BATCH, batch);
    });
    *guard = Some(watcher_guard);
    Ok(format!("watching: {}", dir_list))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Initialize the history database
            if let Ok(app_data) = app.path().app_data_dir() {
                if let Some(path) = app_data.to_str() {
                    let db_dir = format!("{}\\DiskPulse", path);
                    let db_path = format!("{}\\diskpulse.db", db_dir);
                    if let Err(e) = std::fs::create_dir_all(&db_dir) {
                        eprintln!("Warning: Cannot create DB directory: {}", e);
                    }
                    db::set_db_path(db_path);
                    if let Err(e) = db::ensure_tables() {
                        eprintln!("Warning: DB init failed: {}", e);
                    }
                }
            }

            // Build tray menu
            let quick_scan = MenuItemBuilder::with_id("quick_scan", "Quick Scan").build(app)?;
            let open = MenuItemBuilder::with_id("open", "Open DiskPulse").build(app)?;
            let pause = MenuItemBuilder::with_id("pause_monitor", "Pause Monitoring").build(app)?;
            let exit = MenuItemBuilder::with_id("exit", "Exit").build(app)?;

            let menu = MenuBuilder::new(app)
                .item(&quick_scan)
                .item(&open)
                .item(&pause)
                .separator()
                .item(&exit)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("DiskPulse — Disk Monitor & Cleaner")
                .on_menu_event(|app, event| {
                    match event.id().0.as_str() {
                        "quick_scan" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.emit("tray-quick-scan", ());
                            }
                        }
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "pause_monitor" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("tray-toggle-monitor", ());
                            }
                        }
                        "exit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Auto-startup features based on stored settings
            if let Ok(settings) = db::get_settings() {
                let app_handle = app.handle().clone();
                if settings.auto_scan_on_startup {
                    let drive = settings.default_drive.clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(1500));
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.emit(AUTO_SCAN_EVENT, &drive);
                        }
                    });
                }
                if settings.auto_monitor_on_startup {
                    let app_handle2 = app.handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(3000));
                        let _ = start_fs_watcher_internal(&app_handle2);
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_drive,
            list_drives,
            scan_directory,
            classify_risks,
            preview_cleanup,
            clean_items,
            undo_cleanup,
            start_fs_watcher,
            stop_fs_watcher,
            get_snapshot_history,
            get_cleanup_history,
            get_settings,
            save_settings,
            get_rules,
            save_rule_override,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
