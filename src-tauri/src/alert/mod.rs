use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

pub const DISK_SPACE_ALERT: &str = "disk-space-alert";

static ALERT_MONITOR: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertThresholdType {
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "absolute_gb")]
    AbsoluteGB,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskSpaceAlertPayload {
    pub alert_type: String,
    pub drive_letter: String,
    pub message: String,
    pub free_bytes: u64,
    pub total_bytes: u64,
    pub usage_percent: f64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub threshold_type: AlertThresholdType,
    pub threshold_value: f64,
    pub sudden_growth_enabled: bool,
    pub sudden_growth_percent: f64,
    pub sudden_growth_minutes: u64,
    pub check_interval_secs: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_type: AlertThresholdType::Percentage,
            threshold_value: 10.0,
            sudden_growth_enabled: false,
            sudden_growth_percent: 5.0,
            sudden_growth_minutes: 15,
            check_interval_secs: 60,
        }
    }
}

fn drive_free_space(drive: &str) -> Option<(u64, u64, u64)> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

    let path = format!("{}\\", drive);
    let wide: Vec<u16> = std::ffi::OsStr::new(&path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_bytes = 0u64;
    let mut total_bytes = 0u64;
    let mut total_free_bytes = 0u64;

    unsafe {
        let ok = GetDiskFreeSpaceExW(
            windows::core::PCWSTR(wide.as_ptr()),
            Some(&mut free_bytes as *mut u64),
            Some(&mut total_bytes as *mut u64),
            Some(&mut total_free_bytes as *mut u64),
        );
        if ok.is_ok() {
            Some((total_bytes, total_bytes - free_bytes, free_bytes))
        } else {
            None
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn check_threshold(config: &AlertConfig, _drive: &str, total: u64, used: u64, free: u64) -> bool {
    match config.threshold_type {
        AlertThresholdType::Percentage => {
            let usage_pct = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            usage_pct >= (100.0 - config.threshold_value)
        }
        AlertThresholdType::AbsoluteGB => {
            let free_gb = free as f64 / (1024.0 * 1024.0 * 1024.0);
            free_gb <= config.threshold_value
        }
    }
}

fn check_sudden_growth(
    config: &AlertConfig,
    total: u64,
    prev_used: u64,
    current_used: u64,
    elapsed_minutes: f64,
) -> bool {
    if elapsed_minutes <= 0.0 || prev_used == 0 || total == 0 {
        return false;
    }
    let prev_pct = prev_used as f64 / total as f64 * 100.0;
    let current_pct = current_used as f64 / total as f64 * 100.0;
    let growth_pct = current_pct - prev_pct;
    growth_pct >= config.sudden_growth_percent
        && elapsed_minutes <= config.sudden_growth_minutes as f64
}

pub fn start_alert_monitor(app: &AppHandle) -> Result<String, String> {
    let settings = crate::db::get_settings().unwrap_or_default();
    let config = AlertConfig {
        enabled: settings.alert_enabled,
        threshold_type: if settings.alert_threshold_type == "absolute_gb" {
            AlertThresholdType::AbsoluteGB
        } else {
            AlertThresholdType::Percentage
        },
        threshold_value: settings.alert_threshold_value,
        sudden_growth_enabled: settings.alert_growth_enabled,
        sudden_growth_percent: settings.alert_growth_percent,
        sudden_growth_minutes: settings.alert_growth_minutes,
        check_interval_secs: 60,
    };

    if !config.enabled {
        return Ok("alerts disabled in settings".into());
    }

    let mut guard = ALERT_MONITOR
        .lock()
        .map_err(|e| format!("Alert lock error: {}", e))?;
    if guard.is_some() {
        return Ok("alert monitor already running".into());
    }

    let cancel = Arc::new(AtomicBool::new(false));
    *guard = Some(cancel.clone());

    let drive = settings.default_drive;
    let app_handle = app.clone();

    std::thread::Builder::new()
        .name("diskpulse-alert".into())
        .spawn(move || {
            let mut prev_used_bytes: Option<u64> = None;
            let mut prev_check_time: Option<u64> = None;
            let mut last_notification_time: u64 = 0;
            let min_notification_interval_ms: u64 = 10 * 60 * 1000; // 10 min cooldown

            loop {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(config.check_interval_secs));

                if cancel.load(Ordering::Relaxed) {
                    break;
                }

                let Some((total, used, free)) = drive_free_space(&drive) else {
                    continue;
                };

                let now = now_ms();

                // Low space check
                if check_threshold(&config, &drive, total, used, free) {
                    let usage_pct = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
                    let free_gb = free as f64 / (1024.0 * 1024.0 * 1024.0);
                    let msg = match config.threshold_type {
                        AlertThresholdType::Percentage => format!(
                            "Drive {}: is {:.0}% full ({} free)",
                            drive,
                            usage_pct,
                            format_bytes(free),
                        ),
                        AlertThresholdType::AbsoluteGB => format!(
                            "Drive {}: has only {:.2} GB free",
                            drive, free_gb,
                        ),
                    };

                    if now.saturating_sub(last_notification_time) >= min_notification_interval_ms {
                        send_notification(&app_handle, &drive, &msg, free, total, usage_pct, now);
                        last_notification_time = now;
                    }
                }

                // Sudden growth check
                if config.sudden_growth_enabled {
                    if let (Some(prev_used), Some(prev_time)) = (prev_used_bytes, prev_check_time) {
                        let elapsed = (now.saturating_sub(prev_time)) as f64 / 60000.0;
                        if check_sudden_growth(&config, total, prev_used, used, elapsed) {
                            let usage_pct = if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 };
                            let growth_gb = (used.saturating_sub(prev_used)) as f64 / (1024.0 * 1024.0 * 1024.0);
                            let msg = format!(
                                "Drive {}: sudden growth detected — +{:.1} GB in {:.0} min ({:.0}% full)",
                                drive, growth_gb, elapsed, usage_pct,
                            );

                            if now.saturating_sub(last_notification_time) >= min_notification_interval_ms {
                                send_notification(&app_handle, &drive, &msg, free, total, usage_pct, now);
                                last_notification_time = now;
                            }
                        }
                    }
                }

                prev_used_bytes = Some(used);
                prev_check_time = Some(now);
            }
        })
        .map_err(|e| format!("Failed to spawn alert thread: {}", e))?;

    Ok("alert monitor started".into())
}

pub fn stop_alert_monitor() -> Result<String, String> {
    let mut guard = ALERT_MONITOR
        .lock()
        .map_err(|e| format!("Alert lock error: {}", e))?;
    if let Some(token) = guard.take() {
        token.store(true, Ordering::Relaxed);
        Ok("alert monitor stopped".into())
    } else {
        Ok("no alert monitor running".into())
    }
}

fn send_notification(
    app: &AppHandle,
    drive: &str,
    message: &str,
    free: u64,
    total: u64,
    usage_pct: f64,
    timestamp: u64,
) {
    use tauri_plugin_notification::NotificationExt;

    let _ = app
        .notification()
        .builder()
        .title("DiskPulse — Low Disk Space")
        .body(message)
        .show();

    let payload = DiskSpaceAlertPayload {
        alert_type: "low_space".into(),
        drive_letter: drive.to_string(),
        message: message.to_string(),
        free_bytes: free,
        total_bytes: total,
        usage_percent: usage_pct,
        timestamp_ms: timestamp,
    };
    let _ = app.emit(DISK_SPACE_ALERT, payload);
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_percentage_triggers_when_above_limit() {
        let config = AlertConfig {
            threshold_type: AlertThresholdType::Percentage,
            threshold_value: 10.0, // alert when < 10% free → usage >= 90%
            ..AlertConfig::default()
        };
        // 95% used → should trigger
        assert!(check_threshold(&config, "C", 1000, 950, 50));
        // 85% used → should not trigger
        assert!(!check_threshold(&config, "C", 1000, 850, 150));
    }

    #[test]
    fn threshold_absolute_gb_triggers_when_below_limit() {
        let config = AlertConfig {
            threshold_type: AlertThresholdType::AbsoluteGB,
            threshold_value: 20.0, // alert when <= 20 GB free
            ..AlertConfig::default()
        };
        // 15 GB free → trigger
        assert!(check_threshold(
            &config,
            "C",
            500 * 1024 * 1024 * 1024,
            480 * 1024 * 1024 * 1024,
            15 * 1024 * 1024 * 1024,
        ));
        // 25 GB free → no trigger
        assert!(!check_threshold(
            &config,
            "C",
            500 * 1024 * 1024 * 1024,
            480 * 1024 * 1024 * 1024,
            25 * 1024 * 1024 * 1024,
        ));
    }

    #[test]
    fn sudden_growth_detects_rapid_increase() {
        let config = AlertConfig {
            sudden_growth_enabled: true,
            sudden_growth_percent: 5.0,
            sudden_growth_minutes: 15,
            ..AlertConfig::default()
        };
        let total = 500 * 1024 * 1024 * 1024u64;
        let prev_used = 300 * 1024 * 1024 * 1024u64; // 60%
        let current_used = 330 * 1024 * 1024 * 1024u64; // 66% → 6% growth in 10 min → trigger
        assert!(check_sudden_growth(
            &config,
            total,
            prev_used,
            current_used,
            10.0
        ));
    }

    #[test]
    fn sudden_growth_ignores_slow_changes() {
        let config = AlertConfig {
            sudden_growth_enabled: true,
            sudden_growth_percent: 5.0,
            sudden_growth_minutes: 15,
            ..AlertConfig::default()
        };
        let total = 500 * 1024 * 1024 * 1024u64;
        let prev_used = 300 * 1024 * 1024 * 1024u64;
        let current_used = 330 * 1024 * 1024 * 1024u64; // 6% growth but over 60 min → no trigger
        assert!(!check_sudden_growth(
            &config,
            total,
            prev_used,
            current_used,
            60.0
        ));
    }
}
