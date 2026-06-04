mod aging;
mod alert;
mod anomaly;
mod cleaner;
mod cli;
mod db;
pub mod duplicates;
pub mod hub;
pub mod platform;
mod prediction;
mod recommendations;
mod report;
mod risk;
pub mod scanner;
mod scheduler;
mod service;
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FileMeta {
    path: String,
    hard_link_count: u64,
    is_sparse: bool,
    size_on_disk_bytes: Option<u64>,
    identity: Option<platform::FileIdentity>,
}

pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";
pub const SCAN_BATCH_EVENT: &str = "scan-batch";
pub const LARGE_FILE_PROGRESS_EVENT: &str = "large-file-progress";
pub const DUPLICATE_SCAN_PROGRESS_EVENT: &str = "duplicate-scan-progress";
pub const AGING_SCAN_PROGRESS_EVENT: &str = "aging-scan-progress";
pub const CLEAN_PROGRESS_EVENT: &str = "clean-progress";
pub const CLEANUP_COMPLETE_EVENT: &str = "cleanup-complete";
pub const FS_EVENT_BATCH: &str = "fs-event-batch";
pub const DRIVE_CACHE_REFRESHED_EVENT: &str = "drive-cache-refreshed";
pub const AUTO_SCAN_EVENT: &str = "auto-scan";
pub const DISK_SPACE_ALERT: &str = "disk-space-alert";
pub const ANOMALY_DETECTED_EVENT: &str = "anomaly-detected";
pub const DEVICE_CONNECTED_EVENT: &str = hub::DEVICE_CONNECTED_EVENT;
pub const DEVICE_DISCONNECTED_EVENT: &str = hub::DEVICE_DISCONNECTED_EVENT;
pub const REMOTE_ALERT_EVENT: &str = hub::REMOTE_ALERT_EVENT;

/// Global watcher guard so we can stop it from another command.
static WATCHER: Mutex<Option<platform::WatcherGuard>> = Mutex::new(None);
static SCAN_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
static LARGE_FILE_SCAN_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
static DUPLICATE_SCAN_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
static AGING_SCAN_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

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

fn begin_large_file_scan_token() -> Arc<AtomicBool> {
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = LARGE_FILE_SCAN_CANCEL.lock() {
        if let Some(previous) = guard.replace(token.clone()) {
            previous.store(true, Ordering::Relaxed);
        }
    }
    token
}

fn finish_large_file_scan_token(token: &Arc<AtomicBool>) {
    if let Ok(mut guard) = LARGE_FILE_SCAN_CANCEL.lock() {
        if guard
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            *guard = None;
        }
    }
}

fn begin_duplicate_scan_token() -> Arc<AtomicBool> {
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = DUPLICATE_SCAN_CANCEL.lock() {
        if let Some(previous) = guard.replace(token.clone()) {
            previous.store(true, Ordering::Relaxed);
        }
    }
    token
}

fn finish_duplicate_scan_token(token: &Arc<AtomicBool>) {
    if let Ok(mut guard) = DUPLICATE_SCAN_CANCEL.lock() {
        if guard
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            *guard = None;
        }
    }
}

fn begin_aging_scan_token() -> Arc<AtomicBool> {
    let token = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = AGING_SCAN_CANCEL.lock() {
        if let Some(previous) = guard.replace(token.clone()) {
            previous.store(true, Ordering::Relaxed);
        }
    }
    token
}

fn finish_aging_scan_token(token: &Arc<AtomicBool>) {
    if let Ok(mut guard) = AGING_SCAN_CANCEL.lock() {
        if guard
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            *guard = None;
        }
    }
}

fn emit_latest_anomaly(app: &AppHandle, drive: &str) {
    let Ok(history) = db::get_snapshot_history(drive, 365) else {
        return;
    };
    let Some(latest_snapshot_at) = history.iter().map(|snapshot| &snapshot.created_at).max() else {
        return;
    };
    let events = anomaly::AnomalyDetector::default().detect(&history);
    if let Some(event) = events
        .into_iter()
        .filter(|event| event.created_at == *latest_snapshot_at)
        .max_by(|left, right| left.created_at.cmp(&right.created_at))
    {
        let _ = app.emit(ANOMALY_DETECTED_EVENT, event);
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
    } else {
        emit_latest_anomaly(&app, &info.drive_letter);
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
    let batch_app = app.clone();
    let info = scanner::scan_drive_with_progress_batches_and_cancel(
        &drive,
        |progress| {
            let _ = app.emit(SCAN_PROGRESS_EVENT, progress);
        },
        |batch| {
            let _ = batch_app.emit(SCAN_BATCH_EVENT, batch);
        },
        Some(&token),
    );
    finish_scan_token(&token);
    let info = info?;
    if let Err(e) = db::save_snapshot(&info) {
        eprintln!("Failed to save snapshot to history DB: {}", e);
    } else {
        emit_latest_anomaly(&app, &info.drive_letter);
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

/// Scan a drive for the largest individual files and emit progress events.
#[tauri::command]
fn find_large_files(
    app: AppHandle,
    drive: String,
    min_size: u64,
    limit: usize,
) -> Result<Vec<scanner::FileEntry>, String> {
    let token = begin_large_file_scan_token();
    let result = scanner::find_large_files_with_progress_and_cancel(
        &drive,
        min_size,
        limit,
        |progress| {
            let _ = app.emit(LARGE_FILE_PROGRESS_EVENT, progress);
        },
        Some(&token),
    );
    finish_large_file_scan_token(&token);
    result
}

/// Cancel the active large file scan.
#[tauri::command]
fn cancel_large_file_scan() -> Result<(), String> {
    let mut guard = LARGE_FILE_SCAN_CANCEL
        .lock()
        .map_err(|e| format!("Large file scan lock error: {}", e))?;
    if let Some(token) = guard.take() {
        token.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// Scan a drive for identical-content duplicate files.
#[tauri::command]
fn scan_duplicates(
    app: AppHandle,
    drive: String,
    min_size: u64,
) -> Result<Vec<duplicates::DuplicateGroup>, String> {
    let effective_min_size = if min_size == 0 {
        db::get_settings()
            .map(|settings| settings.duplicate_min_size_bytes)
            .unwrap_or_else(|_| db::AppSettings::default().duplicate_min_size_bytes)
    } else {
        min_size
    };
    let token = begin_duplicate_scan_token();
    let result = duplicates::scan_duplicates_with_progress_and_cancel(
        &drive,
        effective_min_size,
        |progress| {
            let _ = app.emit(DUPLICATE_SCAN_PROGRESS_EVENT, progress);
        },
        Some(&token),
    );
    finish_duplicate_scan_token(&token);
    result
}

/// Cancel the active duplicate scan.
#[tauri::command]
fn cancel_duplicate_scan() -> Result<(), String> {
    let mut guard = DUPLICATE_SCAN_CANCEL
        .lock()
        .map_err(|e| format!("Duplicate scan lock error: {}", e))?;
    if let Some(token) = guard.take() {
        token.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// Analyze file age distribution, zombie files, and recent growth hotspots.
#[tauri::command]
fn analyze_file_aging(app: AppHandle, drive: String) -> Result<aging::AgingReport, String> {
    let token = begin_aging_scan_token();
    let result = aging::analyze_file_aging_with_progress_and_cancel(
        &drive,
        |progress| {
            let _ = app.emit(AGING_SCAN_PROGRESS_EVENT, progress);
        },
        Some(&token),
    );
    finish_aging_scan_token(&token);
    result
}

/// Cancel the active aging analysis scan.
#[tauri::command]
fn cancel_aging_scan() -> Result<(), String> {
    let mut guard = AGING_SCAN_CANCEL
        .lock()
        .map_err(|e| format!("Aging scan lock error: {}", e))?;
    if let Some(token) = guard.take() {
        token.store(true, Ordering::Relaxed);
    }
    Ok(())
}

/// List available drives on the system
#[tauri::command]
fn list_drives() -> Result<Vec<String>, String> {
    platform::providers().disk_info.list_drives()
}

/// Scan a specific directory for drill-down navigation
#[tauri::command]
fn scan_directory(path: String) -> Result<Vec<scanner::DirInfo>, String> {
    scanner::scan_directory(&path)
}

/// Classify scan results into risk levels
#[tauri::command]
fn classify_risks(scan: DriveInfo) -> Result<risk::RiskReport, String> {
    let mut rules = risk::built_in_rules();
    if let Ok(custom_rules) = db::get_custom_rules() {
        rules.extend(custom_rules);
    }
    Ok(risk::classify_risks_with_rules(&scan, &rules))
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
    let _ = db::save_notification(&db::NotificationInput {
        notification_type: "cleanup-complete".into(),
        title: "Cleanup complete".into(),
        message: format!(
            "{} cleaned, {} skipped, {} failed",
            result.succeeded, result.skipped, result.failed
        ),
    });
    let _ = app.emit(CLEANUP_COMPLETE_EVENT, &result);
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

    let watcher_guard = platform::providers().fs_watcher.start(
        config,
        Box::new(move |batch| {
            handle_fs_change_batch(handle.clone(), batch);
        }),
    )?;

    *guard = Some(watcher_guard);

    Ok(format!("watching: {}", dir_list))
}

/// Stop the file system watcher.
#[tauri::command]
fn stop_fs_watcher() -> Result<String, String> {
    let mut guard = WATCHER.lock().map_err(|e| format!("Lock error: {}", e))?;
    if let Some(mut w) = guard.take() {
        w.stop();
        Ok("watcher stopped".into())
    } else {
        Ok("no watcher running".into())
    }
}

/// Start the disk space alert monitor.
#[tauri::command]
fn start_alert_monitor(app: AppHandle) -> Result<String, String> {
    alert::start_alert_monitor(&app)
}

/// Stop the disk space alert monitor.
#[tauri::command]
fn stop_alert_monitor() -> Result<String, String> {
    alert::stop_alert_monitor()
}

/// Get application settings.
#[tauri::command]
fn get_settings() -> Result<db::AppSettings, String> {
    db::get_settings()
}

/// Save application settings.
#[tauri::command]
fn save_settings(app: AppHandle, settings: db::AppSettings) -> Result<(), String> {
    db::save_settings(&settings)?;
    scheduler::apply_auto_cleanup_settings(&app, &settings)
}

#[tauri::command]
fn install_service() -> Result<service::ServiceStatus, String> {
    service::install()
}

#[tauri::command]
fn uninstall_service() -> Result<service::ServiceStatus, String> {
    service::uninstall()
}

#[tauri::command]
fn start_service() -> Result<service::ServiceStatus, String> {
    service::start()
}

#[tauri::command]
fn stop_service() -> Result<service::ServiceStatus, String> {
    service::stop()
}

#[tauri::command]
fn get_service_status() -> Result<service::ServiceStatus, String> {
    service::status()
}

#[tauri::command]
fn start_hub(port: u16) -> Result<(), String> {
    hub::start(port)
}

#[tauri::command]
fn stop_hub() -> Result<(), String> {
    hub::stop()
}

#[tauri::command]
fn get_connected_devices() -> Vec<hub::DeviceInfo> {
    hub::connected_devices()
}

#[tauri::command]
fn get_hub_discovery_info() -> Option<hub::DiscoveryInfo> {
    hub::discovery_info()
}

#[tauri::command]
fn discover_devices(timeout_ms: Option<u64>) -> Result<Vec<hub::DeviceInfo>, String> {
    hub::discover_devices(timeout_ms.unwrap_or(1500))
}

#[tauri::command]
fn create_pairing_token(
    device_name: String,
    ttl_seconds: Option<u64>,
) -> Result<hub::PairingToken, String> {
    hub::create_pairing_token(device_name, ttl_seconds.unwrap_or(300))
}

#[tauri::command]
fn pair_device(app: AppHandle, token: String) -> Result<hub::DeviceInfo, String> {
    let device = hub::pair_device(&token)?;
    let _ = app.emit(DEVICE_CONNECTED_EVENT, &device);
    Ok(device)
}

#[tauri::command]
fn unpair_device(app: AppHandle, device_id: String) -> Result<(), String> {
    hub::unpair_device(&device_id)?;
    let _ = app.emit(
        DEVICE_DISCONNECTED_EVENT,
        serde_json::json!({ "device_id": device_id }),
    );
    Ok(())
}

/// Get all risk rules with user overrides applied.
#[tauri::command]
fn get_rules() -> Result<Vec<risk::RiskRule>, String> {
    let overrides = db::get_rule_overrides()?;
    let mut rules = risk::get_rules_with_overrides(&overrides);
    rules.extend(db::get_custom_rules()?);
    Ok(rules)
}

/// Save a single rule override.
#[tauri::command]
fn save_rule_override(rule_id: String, safe_to_delete: bool) -> Result<(), String> {
    db::save_rule_override(&rule_id, safe_to_delete)
}

/// Create or update a custom risk rule.
#[tauri::command]
fn create_custom_rule(
    name: String,
    pattern: String,
    risk_level: String,
) -> Result<risk::RiskRule, String> {
    db::create_custom_rule(&name, &pattern, &risk_level)
}

/// Test a custom risk rule pattern against a sample path.
#[tauri::command]
fn test_rule_pattern(pattern: String, test_path: String) -> Result<bool, String> {
    Ok(risk::test_rule_pattern(&pattern, &test_path))
}

/// Delete a custom risk rule.
#[tauri::command]
fn delete_custom_rule(rule_id: String) -> Result<(), String> {
    db::delete_custom_rule(&rule_id)
}

/// Return persisted notification history.
#[tauri::command]
fn get_notifications() -> Result<Vec<db::NotificationRecord>, String> {
    db::get_notifications()
}

/// Mark all notifications as read.
#[tauri::command]
fn mark_notifications_read() -> Result<(), String> {
    db::mark_notifications_read()
}

/// Mark one notification as read.
#[tauri::command]
fn mark_notification_read(id: i64) -> Result<(), String> {
    db::mark_notification_read(id)
}

/// Clear all persisted notifications.
#[tauri::command]
fn clear_notifications() -> Result<(), String> {
    db::clear_notifications()
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

/// Predict future disk usage from saved snapshot history.
#[tauri::command]
fn predict_disk_usage(drive: String, days: u32) -> Result<prediction::Prediction, String> {
    prediction::predict_disk_usage(&drive, days)
}

/// Detect statistical anomalies from saved snapshot history.
#[tauri::command]
fn detect_anomalies(drive: String) -> Result<anomaly::AnomalyReport, String> {
    anomaly::detect_anomalies(&drive)
}

/// Run the auto-cleanup pipeline immediately using stored settings.
#[tauri::command]
fn run_auto_cleanup_now(app: AppHandle) -> Result<CleanResult, String> {
    scheduler::run_auto_cleanup_now(Some(app))
}

/// Get current auto-cleanup scheduler/status summary.
#[tauri::command]
fn get_auto_cleanup_status() -> Result<scheduler::AutoCleanupStatus, String> {
    scheduler::get_auto_cleanup_status()
}

/// Get previous auto-cleanup reports.
#[tauri::command]
fn get_auto_cleanup_history() -> Result<Vec<db::AutoCleanupReport>, String> {
    scheduler::get_auto_cleanup_history()
}

/// Get ranked cleanup recommendations for a drive.
#[tauri::command]
fn get_recommendations(drive: String) -> Result<Vec<recommendations::Recommendation>, String> {
    recommendations::get_recommendations(&drive)
}

/// Get a lightweight disk health score for a drive.
#[tauri::command]
fn get_disk_health(drive: String) -> Result<recommendations::DiskHealth, String> {
    recommendations::get_disk_health(&drive)
}

/// Export a scan report to a temporary CSV/JSON file.
#[tauri::command]
fn export_scan_report(drive: String, format: String) -> Result<String, String> {
    report::export_scan_report(&drive, &format)
}

/// Export cleanup history to a temporary CSV/JSON file.
#[tauri::command]
fn export_cleanup_history(format: String) -> Result<String, String> {
    report::export_cleanup_history(&format)
}

/// Export duplicate scan results to a temporary CSV/JSON file.
#[tauri::command]
fn export_duplicates(drive: String, format: String) -> Result<String, String> {
    report::export_duplicates(&drive, &format)
}

/// Get the app version
#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Return platform system metadata.
#[tauri::command]
fn get_system_info() -> Result<platform::PlatformSystemInfo, String> {
    let providers = platform::providers();
    Ok(platform::PlatformSystemInfo {
        os_name: providers.system_info.os_name()?,
        os_version: providers.system_info.os_version()?,
        cpu_count: providers.system_info.cpu_count(),
        total_ram_bytes: providers.system_info.total_ram_bytes()?,
        app_data_dir: providers.system_info.app_data_dir()?,
    })
}

/// Return hard-link and sparse-file metadata for a path.
#[tauri::command]
fn get_file_meta(path: String) -> Result<FileMeta, String> {
    let meta = platform::providers().file_meta;
    Ok(FileMeta {
        hard_link_count: meta.hard_link_count(&path)?,
        is_sparse: meta.is_sparse(&path)?,
        size_on_disk_bytes: meta.size_on_disk(&path)?,
        identity: meta.file_identity(&path)?,
        path,
    })
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
    let watcher_guard = platform::providers().fs_watcher.start(
        config,
        Box::new(move |batch| {
            handle_fs_change_batch(handle.clone(), batch);
        }),
    )?;
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
    let args = std::env::args().collect::<Vec<_>>();
    match cli::parse_cli_args(&args) {
        Ok(Some(options)) => {
            std::process::exit(cli::execute_cli_command(&options));
        }
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(3);
        }
        Ok(None) => {}
    }

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
                if settings.alert_enabled {
                    let app_handle3 = app.handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(5000));
                        let _ = alert::start_alert_monitor(&app_handle3);
                    });
                }
                if settings.auto_cleanup_enabled {
                    let app_handle4 = app.handle().clone();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(7000));
                        let _ = scheduler::start_auto_cleanup_scheduler(&app_handle4);
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
            find_large_files,
            cancel_large_file_scan,
            scan_duplicates,
            cancel_duplicate_scan,
            analyze_file_aging,
            cancel_aging_scan,
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
            predict_disk_usage,
            detect_anomalies,
            run_auto_cleanup_now,
            get_auto_cleanup_status,
            get_auto_cleanup_history,
            get_recommendations,
            get_disk_health,
            export_scan_report,
            export_cleanup_history,
            export_duplicates,
            start_alert_monitor,
            stop_alert_monitor,
            get_settings,
            save_settings,
            install_service,
            uninstall_service,
            start_service,
            stop_service,
            get_service_status,
            start_hub,
            stop_hub,
            get_connected_devices,
            get_hub_discovery_info,
            discover_devices,
            create_pairing_token,
            pair_device,
            unpair_device,
            get_rules,
            save_rule_override,
            create_custom_rule,
            test_rule_pattern,
            delete_custom_rule,
            get_notifications,
            mark_notifications_read,
            mark_notification_read,
            clear_notifications,
            get_system_info,
            get_file_meta,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
