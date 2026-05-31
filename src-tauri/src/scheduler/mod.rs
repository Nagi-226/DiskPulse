use crate::cleaner::{self, CleanItem, CleanResult};
use crate::db::{self, AutoCleanupReportInput};
use crate::risk::RiskLevel;
use crate::{risk, scanner};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const AUTO_CLEANUP_COMPLETE_EVENT: &str = "auto-cleanup-complete";
pub const AUTO_CLEANUP_SCHEDULED_EVENT: &str = "auto-cleanup-scheduled";

const DAY_SECS: u64 = 24 * 60 * 60;
const WEEK_SECS: u64 = 7 * DAY_SECS;
const MONTH_SECS: u64 = 30 * DAY_SECS;

static SCHEDULER_CANCEL: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
static LAST_STATUS: Mutex<Option<AutoCleanupStatus>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCleanupConfig {
    pub enabled: bool,
    pub frequency: String,
    pub time: String,
    pub risk_levels: String,
    pub min_free_gb: f64,
    pub drive_letter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCleanupStatus {
    pub enabled: bool,
    pub running: bool,
    pub drive_letter: String,
    pub frequency: String,
    pub next_run_epoch_ms: Option<u64>,
    pub last_run_at: Option<String>,
    pub last_freed_bytes: u64,
    pub message: String,
}

impl From<db::AppSettings> for AutoCleanupConfig {
    fn from(settings: db::AppSettings) -> Self {
        Self {
            enabled: settings.auto_cleanup_enabled,
            frequency: settings.auto_cleanup_frequency,
            time: settings.auto_cleanup_time,
            risk_levels: settings.auto_cleanup_risk_levels,
            min_free_gb: settings.auto_cleanup_min_free_gb,
            drive_letter: settings.default_drive,
        }
    }
}

pub fn calculate_next_run(frequency: &str, time: &str, now_epoch_secs: u64) -> Result<u64, String> {
    let target_secs = parse_time_secs(time)?;
    let today_start = now_epoch_secs - (now_epoch_secs % DAY_SECS);
    let today_target = today_start + target_secs;

    let next = match frequency {
        "daily" => {
            if today_target > now_epoch_secs {
                today_target
            } else {
                today_target + DAY_SECS
            }
        }
        "weekly" => {
            if today_target > now_epoch_secs {
                today_target
            } else {
                today_target + WEEK_SECS
            }
        }
        "monthly" => {
            if today_target > now_epoch_secs {
                today_target
            } else {
                today_target + MONTH_SECS
            }
        }
        other => return Err(format!("Unsupported auto-cleanup frequency: {}", other)),
    };

    Ok(next)
}

pub fn filter_auto_cleanup_candidates(items: Vec<CleanItem>, risk_levels: &str) -> Vec<CleanItem> {
    let allow_low = risk_levels
        .split(',')
        .map(|level| level.trim().to_ascii_lowercase())
        .any(|level| level == "low");

    if !allow_low {
        return Vec::new();
    }

    // Safety invariant: automatic cleanup only runs LOW-risk safe candidates.
    items
        .into_iter()
        .filter(|item| item.safe_to_delete && item.risk_level == RiskLevel::Low)
        .collect()
}

pub fn get_auto_cleanup_status() -> Result<AutoCleanupStatus, String> {
    if let Ok(guard) = LAST_STATUS.lock() {
        if let Some(status) = guard.clone() {
            return Ok(status);
        }
    }

    let settings = db::get_settings().unwrap_or_default();
    let config = AutoCleanupConfig::from(settings);
    status_from_config(&config, false, "auto-cleanup idle")
}

pub fn get_auto_cleanup_history() -> Result<Vec<db::AutoCleanupReport>, String> {
    db::get_auto_cleanup_history()
}

pub fn apply_auto_cleanup_settings(
    app: &AppHandle,
    settings: &db::AppSettings,
) -> Result<(), String> {
    stop_auto_cleanup_scheduler()?;

    let config = AutoCleanupConfig::from(settings.clone());
    let message = if config.enabled {
        "auto-cleanup scheduled"
    } else {
        "auto-cleanup disabled"
    };
    let status = status_from_config(&config, false, message)?;
    set_status(status.clone());
    let _ = app.emit(AUTO_CLEANUP_SCHEDULED_EVENT, &status);

    if config.enabled {
        start_auto_cleanup_scheduler(app)?;
    }

    Ok(())
}

pub fn run_auto_cleanup_now(app: Option<AppHandle>) -> Result<CleanResult, String> {
    let settings = db::get_settings().unwrap_or_default();
    let config = AutoCleanupConfig::from(settings);
    set_status(status_from_config(&config, true, "auto-cleanup running")?);

    let result = run_auto_cleanup_for_config(&config);

    let (clean_result, report_status, final_status, message) = match result {
        Ok(clean_result) => {
            let message = format!(
                "auto-cleanup completed: {} succeeded, {} skipped, {} failed",
                clean_result.succeeded, clean_result.skipped, clean_result.failed
            );
            let mut status = status_from_config(&config, false, &message)?;
            status.last_freed_bytes = clean_result.freed_bytes;
            (clean_result, "completed".to_string(), status, message)
        }
        Err(err) => {
            let clean_result = empty_clean_result();
            let message = format!("auto-cleanup failed: {}", err);
            let status = status_from_config(&config, false, &message)?;
            persist_report(&config.drive_letter, &clean_result, "failed", &message)?;
            set_status(status);
            return Err(err);
        }
    };

    persist_report(
        &config.drive_letter,
        &clean_result,
        &report_status,
        &message,
    )?;
    set_status(final_status);

    if let Some(app_handle) = app {
        let _ = app_handle.emit(AUTO_CLEANUP_COMPLETE_EVENT, &clean_result);
    }

    Ok(clean_result)
}

pub fn start_auto_cleanup_scheduler(app: &AppHandle) -> Result<String, String> {
    let settings = db::get_settings().unwrap_or_default();
    let config = AutoCleanupConfig::from(settings);
    if !config.enabled {
        return Ok("auto-cleanup disabled in settings".into());
    }

    let mut guard = SCHEDULER_CANCEL
        .lock()
        .map_err(|e| format!("Auto-cleanup lock error: {}", e))?;
    if guard.is_some() {
        return Ok("auto-cleanup scheduler already running".into());
    }

    let cancel = Arc::new(AtomicBool::new(false));
    *guard = Some(cancel.clone());
    let thread_cancel = cancel.clone();
    let app_handle = app.clone();

    std::thread::Builder::new()
        .name("diskpulse-auto-cleanup".into())
        .spawn(move || {
            loop {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }

                let settings = db::get_settings().unwrap_or_default();
                let config = AutoCleanupConfig::from(settings);
                if !config.enabled {
                    break;
                }

                let Ok(status) = status_from_config(&config, false, "auto-cleanup scheduled") else {
                    break;
                };
                set_status(status.clone());
                let _ = app_handle.emit(AUTO_CLEANUP_SCHEDULED_EVENT, &status);

                let Some(next_run_ms) = status.next_run_epoch_ms else {
                    break;
                };
                let now_ms = now_epoch_secs().saturating_mul(1000);
                let wait_ms = next_run_ms.saturating_sub(now_ms);
                sleep_cancellable(wait_ms, &cancel);
                if cancel.load(Ordering::Relaxed) {
                    break;
                }

                let _ = run_auto_cleanup_now(Some(app_handle.clone()));
            }

            finish_scheduler_token(&thread_cancel);
        })
        .map_err(|e| format!("Failed to spawn auto-cleanup thread: {}", e))?;

    Ok("auto-cleanup scheduler started".into())
}

pub fn stop_auto_cleanup_scheduler() -> Result<String, String> {
    let mut guard = SCHEDULER_CANCEL
        .lock()
        .map_err(|e| format!("Auto-cleanup lock error: {}", e))?;

    if let Some(cancel) = guard.take() {
        cancel.store(true, Ordering::Relaxed);
        Ok("auto-cleanup scheduler stopped".into())
    } else {
        Ok("auto-cleanup scheduler not running".into())
    }
}

fn finish_scheduler_token(token: &Arc<AtomicBool>) {
    if let Ok(mut guard) = SCHEDULER_CANCEL.lock() {
        if guard
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, token))
        {
            *guard = None;
        }
    }
}

fn run_auto_cleanup_for_config(config: &AutoCleanupConfig) -> Result<CleanResult, String> {
    if should_skip_for_free_space(config)? {
        return Ok(empty_clean_result());
    }

    let scan = scanner::scan_drive_with_progress(&config.drive_letter, |_| {})?;
    let report = risk::classify_risks(&scan);
    let items: Vec<CleanItem> = report.items.iter().map(CleanItem::from).collect();
    let candidates = filter_auto_cleanup_candidates(items, &config.risk_levels);

    if candidates.is_empty() {
        return Ok(empty_clean_result());
    }

    Ok(cleaner::clean_items_with_progress(candidates, None, |_| {}))
}

fn should_skip_for_free_space(config: &AutoCleanupConfig) -> Result<bool, String> {
    let meta = scanner::scan_drive_meta(&config.drive_letter, None, None)?;
    let min_free_bytes = (config.min_free_gb * 1024.0 * 1024.0 * 1024.0) as u64;
    Ok(meta.free_bytes >= min_free_bytes)
}

fn persist_report(
    drive_letter: &str,
    clean_result: &CleanResult,
    status: &str,
    message: &str,
) -> Result<(), String> {
    let items_json = serde_json::to_string(&clean_result.items)
        .map_err(|e| format!("Auto-cleanup report serialize error: {}", e))?;
    db::save_auto_cleanup_report(&AutoCleanupReportInput {
        drive_letter: drive_letter.to_string(),
        freed_bytes: clean_result.freed_bytes,
        succeeded: clean_result.succeeded,
        skipped: clean_result.skipped,
        failed: clean_result.failed,
        status: status.to_string(),
        message: message.to_string(),
        items_json,
    })
}

fn status_from_config(
    config: &AutoCleanupConfig,
    running: bool,
    message: &str,
) -> Result<AutoCleanupStatus, String> {
    let next_run_epoch_ms = if config.enabled {
        Some(calculate_next_run(&config.frequency, &config.time, now_epoch_secs())? * 1000)
    } else {
        None
    };

    let last = db::get_auto_cleanup_history()
        .ok()
        .and_then(|mut reports| reports.drain(..).next());

    Ok(AutoCleanupStatus {
        enabled: config.enabled,
        running,
        drive_letter: config.drive_letter.clone(),
        frequency: config.frequency.clone(),
        next_run_epoch_ms,
        last_run_at: last.as_ref().map(|report| report.created_at.clone()),
        last_freed_bytes: last.map(|report| report.freed_bytes).unwrap_or(0),
        message: message.to_string(),
    })
}

fn set_status(status: AutoCleanupStatus) {
    if let Ok(mut guard) = LAST_STATUS.lock() {
        *guard = Some(status);
    }
}

fn empty_clean_result() -> CleanResult {
    CleanResult {
        total_attempted: 0,
        succeeded: 0,
        skipped: 0,
        failed: 0,
        freed_bytes: 0,
        items: Vec::new(),
    }
}

fn parse_time_secs(time: &str) -> Result<u64, String> {
    let (hour, minute) = time
        .split_once(':')
        .ok_or_else(|| "Auto-cleanup time must use HH:MM".to_string())?;
    let hour: u64 = hour
        .parse()
        .map_err(|_| "Auto-cleanup hour must be numeric".to_string())?;
    let minute: u64 = minute
        .parse()
        .map_err(|_| "Auto-cleanup minute must be numeric".to_string())?;
    if hour > 23 || minute > 59 {
        return Err("Auto-cleanup time is out of range".into());
    }
    Ok(hour * 60 * 60 + minute * 60)
}

fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sleep_cancellable(wait_ms: u64, cancel: &AtomicBool) {
    let mut remaining = wait_ms;
    while remaining > 0 && !cancel.load(Ordering::Relaxed) {
        let step = remaining.min(30_000);
        std::thread::sleep(std::time::Duration::from_millis(step));
        remaining -= step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaner::CleanItem;
    use crate::risk::RiskLevel;

    #[test]
    fn calculate_next_run_daily_uses_today_when_time_is_future() {
        let now = 10 * 60 * 60;
        let next = calculate_next_run("daily", "12:30", now).expect("next run");
        assert_eq!(next, 12 * 60 * 60 + 30 * 60);
    }

    #[test]
    fn calculate_next_run_daily_rolls_to_tomorrow_when_time_passed() {
        let now = 23 * 60 * 60;
        let next = calculate_next_run("daily", "03:00", now).expect("next run");
        assert_eq!(next, 27 * 60 * 60);
    }

    #[test]
    fn calculate_next_run_rejects_invalid_time() {
        assert!(calculate_next_run("daily", "25:99", 0).is_err());
    }

    #[test]
    fn auto_cleanup_candidates_only_include_low_risk_safe_items() {
        let items = vec![
            CleanItem {
                name: "cache".into(),
                path: "C:\\Temp\\cache".into(),
                size_bytes: 10,
                risk_level: RiskLevel::Low,
                safe_to_delete: true,
            },
            CleanItem {
                name: "downloads".into(),
                path: "C:\\Users\\me\\Downloads".into(),
                size_bytes: 20,
                risk_level: RiskLevel::Medium,
                safe_to_delete: true,
            },
        ];

        let candidates = filter_auto_cleanup_candidates(items, "low");

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "cache");
    }
}
