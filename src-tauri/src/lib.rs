mod cleaner;
mod db;
mod risk;
mod scanner;
mod watcher;

use cleaner::{CleanItem, CleanPreview, CleanResult, RestoreResult};
use scanner::{DirInfo, DriveInfo, DriveMeta};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";
pub const CLEAN_PROGRESS_EVENT: &str = "clean-progress";
pub const FS_EVENT_BATCH: &str = "fs-event-batch";
pub const DRIVE_CACHE_REFRESHED_EVENT: &str = "drive-cache-refreshed";
pub const AUTO_SCAN_EVENT: &str = "auto-scan";

/// Global watcher guard so we can stop it from another command.
static WATCHER: Mutex<Option<watcher::WatcherGuard>> = Mutex::new(None);
static SCAN_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

fn begin_scan_token() -> Arc<AtomicBool> {
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = SCAN_CANCEL.lock() {
        if let Some(previous) = guard.replace(token.clone()) {
            previous.store(true, Ordering::Relaxed);
        }
    }
    token
}

fn finish_scan_token(token: &Arc<AtomicBool>) {
    if let Ok(mut guard) = SCAN_CANCEL.lock() {
        if guard
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            *guard = None;
        }
    }
}

/// Scan a drive and emit progress events
#[tauri::command]
fn scan_drive(app: AppHandle, drive: String) -> Result<DriveInfo, String> {
    let token = begin_scan_token();
    let info = scanner::scan_drive_with_progress_and_cancel(
        &drive,
        |progress| {
            let _ = app.emit(SCAN_PROGRESS_EVENT, progress);
        },
        Some(&token),
    );
    finish_scan_token(&token);
    let info = info?;
    if let Err(e) = db::save_snapshot(&info) {
        eprintln!("Failed to save snapshot to history DB: {}", e);
    }
    Ok(info)
}

/// Return drive capacity and last cached top-level directories without a full walk.
#[tauri::command]
fn scan_drive_meta(drive: String) -> Result<DriveMeta, String> {
    let cached = db::get_latest_snapshot_for_drive(&drive).ok().flatten();
    let (cached_top_dirs, cache_age_ms) = cached
        .and_then(|cache| {
            serde_json::from_str::<Vec<DirInfo>>(&cache.snapshot.snapshot_json)
                .ok()
                .map(|dirs| (Some(dirs), Some(cache.cache_age_ms)))
        })
        .unwrap_or((None, None));

    scanner::scan_drive_meta(&drive, cached_top_dirs, cache_age_ms)
}

/// Scan top-level directories in the background and return only the dir list.
#[tauri::command]
fn scan_drive_dirs(app: AppHandle, drive: String) -> Result<Vec<DirInfo>, String> {
    let token = begin_scan_token();
    let info = scanner::scan_drive_with_progress_and_cancel(
        &drive,
        |progress| {
            let _ = app.emit(SCAN_PROGRESS_EVENT, progress);
        },
        Some(&token),
    );
    finish_scan_token(&token);
    let info = info?;
    if let Err(e) = db::save_snapshot(&info) {
        eprintln!("Failed to save snapshot to history DB: {}", e);
    }
    Ok(info.top_dirs)
}

/// Cancel the active background scan.
#[tauri::command]
fn cancel_scan() -> Result<(), String> {
    let mut guard = SCAN_CANCEL
        .lock()
        .map_err(|e| format!("Scan lock error: {}", e))?;
    if let Some(token) = guard.take() {
        token.store(true, Ordering::Relaxed);
    }
    Ok(())
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

#[derive(Debug, Clone)]
struct DirtyTopDir {
    name: String,
    path: String,
}

fn top_level_from_path(path: &str) -> Option<(String, String, String)> {
    let normalized = path.replace('/', "\\");
    let mut chars = normalized.chars();
    let drive = chars.next()?.to_ascii_uppercase();
    if chars.next()? != ':' {
        return None;
    }
    let rest = normalized.get(2..)?.trim_start_matches('\\');
    let name = rest.split('\\').next()?.trim();
    let protected_name = name.to_ascii_lowercase();
    if name.is_empty()
        || matches!(
            protected_name.as_str(),
            "system volume information" | "$recycle.bin"
        )
    {
        return None;
    }
    Some((
        drive.to_string(),
        name.to_string(),
        format!("{}:\\{}", drive, name),
    ))
}

fn dirty_top_dirs_from_batch(
    batch: &watcher::FsChangeBatch,
) -> HashMap<String, HashMap<String, DirtyTopDir>> {
    let mut updates: HashMap<String, HashMap<String, DirtyTopDir>> = HashMap::new();

    for event in &batch.events {
        let Some((drive, name, top_path)) = top_level_from_path(&event.path) else {
            continue;
        };

        let by_path = updates.entry(drive).or_default();
        by_path.entry(top_path.clone()).or_insert(DirtyTopDir {
            name,
            path: top_path,
        });
    }

    updates
}

fn refresh_dirty_drive_cache(batch: &watcher::FsChangeBatch) -> Vec<DriveInfo> {
    let mut refreshed = Vec::new();

    for (drive, by_path) in dirty_top_dirs_from_batch(batch) {
        let Some(cached) = db::get_latest_snapshot_for_drive(&drive).ok().flatten() else {
            continue;
        };
        let Ok(mut dirs) = serde_json::from_str::<Vec<DirInfo>>(&cached.snapshot.snapshot_json)
        else {
            continue;
        };

        let mut changed = false;
        for dirty in by_path.values() {
            let existing_index = dirs
                .iter()
                .position(|dir| dir.path.eq_ignore_ascii_case(&dirty.path));

            if Path::new(&dirty.path).is_dir() {
                match scanner::scan_top_level_dir(&dirty.path, None) {
                    Ok(mut dir) => {
                        dir.name = dirty.name.clone();
                        dir.path = dirty.path.clone();
                        if let Some(index) = existing_index {
                            dirs[index] = dir;
                        } else {
                            dirs.push(dir);
                        }
                        changed = true;
                    }
                    Err(e) => eprintln!("Failed to refresh dirty dir {}: {}", dirty.path, e),
                }
            } else if let Some(index) = existing_index {
                dirs.remove(index);
                changed = true;
            }
        }

        if !changed {
            continue;
        }

        dirs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        let meta = scanner::scan_drive_meta(&drive, None, None).ok();
        let info = DriveInfo {
            drive_letter: drive,
            total_bytes: meta
                .as_ref()
                .map(|m| m.total_bytes)
                .unwrap_or(cached.snapshot.total_bytes),
            used_bytes: meta
                .as_ref()
                .map(|m| m.used_bytes)
                .unwrap_or(cached.snapshot.used_bytes),
            free_bytes: meta
                .map(|m| m.free_bytes)
                .unwrap_or(cached.snapshot.free_bytes),
            top_dirs: dirs,
        };

        if let Err(e) = db::save_snapshot(&info) {
            eprintln!("Failed to refresh dirty drive cache: {}", e);
        } else {
            refreshed.push(info);
        }
    }

    refreshed
}

fn handle_fs_change_batch(handle: AppHandle, batch: watcher::FsChangeBatch) {
    let refresh_batch = batch.clone();
    let refresh_handle = handle.clone();

    let _ = handle.emit(FS_EVENT_BATCH, batch);
    std::thread::spawn(move || {
        for info in refresh_dirty_drive_cache(&refresh_batch) {
            let _ = refresh_handle.emit(DRIVE_CACHE_REFRESHED_EVENT, info);
        }
    });
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
        handle_fs_change_batch(handle.clone(), batch);
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
        handle_fs_change_batch(handle.clone(), batch);
    });
    *guard = Some(watcher_guard);
    Ok(format!("watching: {}", dir_list))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_batch(paths: &[&str]) -> watcher::FsChangeBatch {
        watcher::FsChangeBatch {
            watched_dir: "C:\\Users\\me\\Downloads".into(),
            events: paths
                .iter()
                .map(|path| watcher::FsEvent {
                    kind: watcher::FsEventKind::Modified,
                    path: (*path).into(),
                    is_directory: false,
                    size_bytes: 1,
                    previous_size_bytes: Some(0),
                })
                .collect(),
            event_count: paths.len(),
            timestamp_ms: 1,
        }
    }

    #[test]
    fn top_level_from_path_extracts_drive_root_child() {
        let (drive, name, path) =
            top_level_from_path("c:/Users/me/Downloads/file.zip").expect("top dir");
        assert_eq!(drive, "C");
        assert_eq!(name, "Users");
        assert_eq!(path, "C:\\Users");
    }

    #[test]
    fn top_level_from_path_skips_protected_roots() {
        assert!(top_level_from_path("C:\\System Volume Information\\x").is_none());
        assert!(top_level_from_path("C:\\$Recycle.Bin\\x").is_none());
    }

    #[test]
    fn dirty_top_dirs_are_deduplicated_per_drive() {
        let batch = make_batch(&[
            "C:\\Users\\me\\Downloads\\a.zip",
            "C:\\Users\\me\\AppData\\Local\\b.tmp",
            "D:\\Games\\cache.bin",
        ]);

        let dirty = dirty_top_dirs_from_batch(&batch);
        assert_eq!(dirty.get("C").map(|d| d.len()), Some(1));
        assert!(dirty
            .get("C")
            .is_some_and(|dirs| dirs.contains_key("C:\\Users")));
        assert!(dirty
            .get("D")
            .is_some_and(|dirs| dirs.contains_key("D:\\Games")));
    }
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
                .on_menu_event(|app, event| match event.id().0.as_str() {
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
            scan_drive_meta,
            scan_drive_dirs,
            cancel_scan,
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
