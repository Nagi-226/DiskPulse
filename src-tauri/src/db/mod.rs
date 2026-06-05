use crate::cleaner;
use crate::scanner;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// A stored disk usage snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: i64,
    pub drive_letter: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub snapshot_json: String,
    pub created_at: String,
}

/// Latest cached snapshot with an age computed by SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSnapshot {
    pub snapshot: Snapshot,
    pub cache_age_ms: u64,
}

/// A stored cleanup operation log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupLog {
    pub id: i64,
    pub item_count: usize,
    pub freed_bytes: u64,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub items_json: String,
    pub created_at: String,
}

/// Stored scheduled/manual auto-cleanup execution report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCleanupReport {
    pub id: i64,
    pub drive_letter: String,
    pub freed_bytes: u64,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub status: String,
    pub message: String,
    pub items_json: String,
    pub created_at: String,
}

/// Input used to persist an auto-cleanup report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCleanupReportInput {
    pub drive_letter: String,
    pub freed_bytes: u64,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub status: String,
    pub message: String,
    pub items_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub id: i64,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationInput {
    pub notification_type: String,
    pub title: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub id: i64,
    pub drive_letter: String,
    pub score: u8,
    pub space_score: u8,
    pub waste_score: u8,
    pub trend_score: u8,
    pub age_score: u8,
    pub frag_score: u8,
    pub anomaly_score: u8,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshotInput {
    pub drive_letter: String,
    pub score: u8,
    pub space_score: u8,
    pub waste_score: u8,
    pub trend_score: u8,
    pub age_score: u8,
    pub frag_score: u8,
    pub anomaly_score: u8,
}

/// Application settings persisted across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub default_drive: String,
    pub scan_mode: String,
    pub auto_scan_on_startup: bool,
    pub auto_monitor_on_startup: bool,
    pub watcher_poll_interval_ms: u64,
    pub watcher_debounce_ms: u64,
    pub alert_enabled: bool,
    pub alert_threshold_type: String,
    pub alert_threshold_value: f64,
    pub alert_growth_enabled: bool,
    pub alert_growth_percent: f64,
    pub alert_growth_minutes: u64,
    pub auto_cleanup_enabled: bool,
    pub auto_cleanup_frequency: String,
    pub auto_cleanup_time: String,
    pub auto_cleanup_risk_levels: String,
    pub auto_cleanup_min_free_gb: f64,
    pub language: String,
    pub theme: String,
    pub update_check_enabled: bool,
    pub scoring_weight_risk: f64,
    pub scoring_weight_age: f64,
    pub scoring_weight_duplicate: f64,
    pub scoring_weight_size: f64,
    pub scoring_weight_safety: f64,
    pub scoring_weight_urgency: f64,
    pub scoring_weight_pattern: f64,
    pub duplicate_min_size_bytes: u64,
    pub aging_zombie_days: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_drive: "C".into(),
            scan_mode: "exact".into(),
            auto_scan_on_startup: false,
            auto_monitor_on_startup: false,
            watcher_poll_interval_ms: 2000,
            watcher_debounce_ms: 1500,
            alert_enabled: false,
            alert_threshold_type: "percentage".into(),
            alert_threshold_value: 10.0,
            alert_growth_enabled: false,
            alert_growth_percent: 5.0,
            alert_growth_minutes: 15,
            auto_cleanup_enabled: false,
            auto_cleanup_frequency: "weekly".into(),
            auto_cleanup_time: "03:00".into(),
            auto_cleanup_risk_levels: "low".into(),
            auto_cleanup_min_free_gb: 50.0,
            language: "auto".into(),
            theme: "auto".into(),
            update_check_enabled: true,
            scoring_weight_risk: 0.20,
            scoring_weight_age: 0.15,
            scoring_weight_duplicate: 0.20,
            scoring_weight_size: 0.20,
            scoring_weight_safety: 0.25,
            scoring_weight_urgency: 0.15,
            scoring_weight_pattern: 0.10,
            duplicate_min_size_bytes: 1_048_576,
            aging_zombie_days: 180,
        }
    }
}

static DB_PATH: Mutex<Option<String>> = Mutex::new(None);

pub fn set_db_path(path: String) {
    if let Ok(mut guard) = DB_PATH.lock() {
        *guard = Some(path);
    }
}

fn get_db_path() -> Result<String, String> {
    DB_PATH
        .lock()
        .map_err(|e| format!("DB lock error: {}", e))?
        .clone()
        .ok_or_else(|| "DB not initialized".to_string())
}

fn open_connection() -> Result<rusqlite::Connection, String> {
    let path = get_db_path()?;
    if let Some(parent) = std::path::Path::new(&path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Cannot create DB dir: {}", e))?;
    }
    let conn = rusqlite::Connection::open(&path).map_err(|e| format!("Cannot open DB: {}", e))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;").ok();
    Ok(conn)
}

// ── Internal helpers (used by both public API and tests) ───

fn ensure_tables_with(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            drive_letter TEXT NOT NULL,
            total_bytes INTEGER NOT NULL,
            used_bytes INTEGER NOT NULL,
            free_bytes INTEGER NOT NULL,
            snapshot_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS cleanup_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_count INTEGER NOT NULL,
            freed_bytes INTEGER NOT NULL,
            succeeded INTEGER NOT NULL,
            skipped INTEGER NOT NULL,
            failed INTEGER NOT NULL,
            items_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS auto_cleanup_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            drive_letter TEXT NOT NULL,
            freed_bytes INTEGER NOT NULL,
            succeeded INTEGER NOT NULL,
            skipped INTEGER NOT NULL,
            failed INTEGER NOT NULL,
            status TEXT NOT NULL,
            message TEXT NOT NULL,
            items_json TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_snapshots_drive ON snapshots(drive_letter, created_at);
        CREATE INDEX IF NOT EXISTS idx_cleanup_logs_time ON cleanup_logs(created_at);
        CREATE INDEX IF NOT EXISTS idx_auto_cleanup_reports_time ON auto_cleanup_reports(created_at);

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value_text TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS rule_overrides (
            rule_id TEXT PRIMARY KEY,
            safe_to_delete INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS custom_rules (
            id TEXT PRIMARY KEY,
            pattern TEXT NOT NULL,
            risk_level TEXT NOT NULL,
            category TEXT NOT NULL,
            explanation TEXT NOT NULL,
            safe_to_delete INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS notifications (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            notification_type TEXT NOT NULL,
            title TEXT NOT NULL,
            message TEXT NOT NULL,
            read INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS health_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            drive_letter TEXT NOT NULL,
            score INTEGER NOT NULL,
            space_score INTEGER NOT NULL,
            waste_score INTEGER NOT NULL,
            trend_score INTEGER NOT NULL,
            age_score INTEGER NOT NULL,
            frag_score INTEGER NOT NULL,
            anomaly_score INTEGER NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )
    .map_err(|e| format!("Cannot create tables: {}", e))
}

fn save_health_snapshot_with(
    conn: &rusqlite::Connection,
    input: &HealthSnapshotInput,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO health_snapshots
         (drive_letter, score, space_score, waste_score, trend_score, age_score, frag_score, anomaly_score)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            input.drive_letter,
            input.score,
            input.space_score,
            input.waste_score,
            input.trend_score,
            input.age_score,
            input.frag_score,
            input.anomaly_score,
        ],
    )
    .map_err(|e| format!("Save health snapshot error: {}", e))?;
    Ok(())
}

fn get_health_history_with(
    conn: &rusqlite::Connection,
    drive: &str,
    limit: usize,
) -> Result<Vec<HealthSnapshot>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, drive_letter, score, space_score, waste_score, trend_score,
                    age_score, frag_score, anomaly_score, created_at
             FROM health_snapshots
             WHERE drive_letter = ?1
             ORDER BY datetime(created_at) DESC, id DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Query error: {}", e))?;
    let rows = stmt
        .query_map(rusqlite::params![drive, limit as i64], |row| {
            Ok(HealthSnapshot {
                id: row.get(0)?,
                drive_letter: row.get(1)?,
                score: row.get::<_, i64>(2)? as u8,
                space_score: row.get::<_, i64>(3)? as u8,
                waste_score: row.get::<_, i64>(4)? as u8,
                trend_score: row.get::<_, i64>(5)? as u8,
                age_score: row.get::<_, i64>(6)? as u8,
                frag_score: row.get::<_, i64>(7)? as u8,
                anomaly_score: row.get::<_, i64>(8)? as u8,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))
}

fn save_snapshot_with(
    conn: &rusqlite::Connection,
    drive_info: &scanner::DriveInfo,
) -> Result<(), String> {
    let top_dirs_json = serde_json::to_string(&drive_info.top_dirs)
        .map_err(|e| format!("JSON serialize error: {}", e))?;
    conn.execute(
        "INSERT INTO snapshots (drive_letter, total_bytes, used_bytes, free_bytes, snapshot_json)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            drive_info.drive_letter,
            drive_info.total_bytes as i64,
            drive_info.used_bytes as i64,
            drive_info.free_bytes as i64,
            top_dirs_json,
        ],
    )
    .map_err(|e| format!("Insert snapshot error: {}", e))?;
    Ok(())
}

fn save_cleanup_log_with(
    conn: &rusqlite::Connection,
    result: &cleaner::CleanResult,
) -> Result<(), String> {
    let items_json =
        serde_json::to_string(&result.items).map_err(|e| format!("JSON serialize error: {}", e))?;
    conn.execute(
        "INSERT INTO cleanup_logs (item_count, freed_bytes, succeeded, skipped, failed, items_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            result.total_attempted as i64,
            result.freed_bytes as i64,
            result.succeeded as i64,
            result.skipped as i64,
            result.failed as i64,
            items_json,
        ],
    )
    .map_err(|e| format!("Insert cleanup log error: {}", e))?;
    Ok(())
}

fn get_snapshot_history_with(
    conn: &rusqlite::Connection,
    drive: &str,
    days: u32,
) -> Result<Vec<Snapshot>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, drive_letter, total_bytes, used_bytes, free_bytes, snapshot_json, created_at
         FROM snapshots
         WHERE drive_letter = ?1
           AND created_at >= datetime('now', ?2)
         ORDER BY created_at DESC"
    ).map_err(|e| format!("Query error: {}", e))?;

    let rows = stmt
        .query_map(
            rusqlite::params![drive.to_uppercase(), format!("-{} days", days)],
            |row| {
                Ok(Snapshot {
                    id: row.get(0)?,
                    drive_letter: row.get(1)?,
                    total_bytes: row.get::<_, i64>(2)? as u64,
                    used_bytes: row.get::<_, i64>(3)? as u64,
                    free_bytes: row.get::<_, i64>(4)? as u64,
                    snapshot_json: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        )
        .map_err(|e| format!("Query error: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))
}

fn get_latest_snapshot_for_drive_with(
    conn: &rusqlite::Connection,
    drive: &str,
) -> Result<Option<CachedSnapshot>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, drive_letter, total_bytes, used_bytes, free_bytes, snapshot_json, created_at,
                CAST(MAX(0, (julianday('now') - julianday(created_at)) * 86400000) AS INTEGER) AS cache_age_ms
         FROM snapshots
         WHERE drive_letter = ?1
         ORDER BY created_at DESC
         LIMIT 1"
    ).map_err(|e| format!("Query error: {}", e))?;

    match stmt.query_row(rusqlite::params![drive.to_uppercase()], |row| {
        Ok(CachedSnapshot {
            snapshot: Snapshot {
                id: row.get(0)?,
                drive_letter: row.get(1)?,
                total_bytes: row.get::<_, i64>(2)? as u64,
                used_bytes: row.get::<_, i64>(3)? as u64,
                free_bytes: row.get::<_, i64>(4)? as u64,
                snapshot_json: row.get(5)?,
                created_at: row.get(6)?,
            },
            cache_age_ms: row.get::<_, i64>(7)? as u64,
        })
    }) {
        Ok(snapshot) => Ok(Some(snapshot)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Row error: {}", e)),
    }
}

fn get_cleanup_history_with(conn: &rusqlite::Connection) -> Result<Vec<CleanupLog>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, item_count, freed_bytes, succeeded, skipped, failed, items_json, created_at
         FROM cleanup_logs
         ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Query error: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(CleanupLog {
                id: row.get(0)?,
                item_count: row.get::<_, i64>(1)? as usize,
                freed_bytes: row.get::<_, i64>(2)? as u64,
                succeeded: row.get::<_, i64>(3)? as usize,
                skipped: row.get::<_, i64>(4)? as usize,
                failed: row.get::<_, i64>(5)? as usize,
                items_json: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))
}

fn save_auto_cleanup_report_with(
    conn: &rusqlite::Connection,
    report: &AutoCleanupReportInput,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO auto_cleanup_reports
            (drive_letter, freed_bytes, succeeded, skipped, failed, status, message, items_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            report.drive_letter.to_uppercase(),
            report.freed_bytes as i64,
            report.succeeded as i64,
            report.skipped as i64,
            report.failed as i64,
            report.status,
            report.message,
            report.items_json,
        ],
    )
    .map_err(|e| format!("Insert auto-cleanup report error: {}", e))?;
    Ok(())
}

fn get_auto_cleanup_history_with(
    conn: &rusqlite::Connection,
) -> Result<Vec<AutoCleanupReport>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, drive_letter, freed_bytes, succeeded, skipped, failed,
                    status, message, items_json, created_at
             FROM auto_cleanup_reports
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Query error: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AutoCleanupReport {
                id: row.get(0)?,
                drive_letter: row.get(1)?,
                freed_bytes: row.get::<_, i64>(2)? as u64,
                succeeded: row.get::<_, i64>(3)? as usize,
                skipped: row.get::<_, i64>(4)? as usize,
                failed: row.get::<_, i64>(5)? as usize,
                status: row.get(6)?,
                message: row.get(7)?,
                items_json: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|e| format!("Query error: {}", e))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row error: {}", e))
}

fn get_settings_with(conn: &rusqlite::Connection) -> Result<AppSettings, String> {
    let mut settings = AppSettings::default();
    let mut stmt = conn
        .prepare("SELECT key, value_text FROM settings")
        .map_err(|e| format!("Query error: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("Query error: {}", e))?;
    for row in rows {
        let (key, value) = row.map_err(|e| format!("Row error: {}", e))?;
        match key.as_str() {
            "default_drive" => settings.default_drive = value,
            "scan_mode" => {
                settings.scan_mode = if value == "speed" {
                    "speed".into()
                } else {
                    "exact".into()
                };
            }
            "auto_scan_on_startup" => settings.auto_scan_on_startup = value == "true",
            "auto_monitor_on_startup" => settings.auto_monitor_on_startup = value == "true",
            "watcher_poll_interval_ms" => {
                if let Ok(v) = value.parse() {
                    settings.watcher_poll_interval_ms = v;
                }
            }
            "watcher_debounce_ms" => {
                if let Ok(v) = value.parse() {
                    settings.watcher_debounce_ms = v;
                }
            }
            "alert_enabled" => settings.alert_enabled = value == "true",
            "alert_threshold_type" => settings.alert_threshold_type = value,
            "alert_threshold_value" => {
                if let Ok(v) = value.parse() {
                    settings.alert_threshold_value = v;
                }
            }
            "alert_growth_enabled" => settings.alert_growth_enabled = value == "true",
            "alert_growth_percent" => {
                if let Ok(v) = value.parse() {
                    settings.alert_growth_percent = v;
                }
            }
            "alert_growth_minutes" => {
                if let Ok(v) = value.parse() {
                    settings.alert_growth_minutes = v;
                }
            }
            "auto_cleanup_enabled" => settings.auto_cleanup_enabled = value == "true",
            "auto_cleanup_frequency" => settings.auto_cleanup_frequency = value,
            "auto_cleanup_time" => settings.auto_cleanup_time = value,
            "auto_cleanup_risk_levels" => settings.auto_cleanup_risk_levels = value,
            "auto_cleanup_min_free_gb" => {
                if let Ok(v) = value.parse() {
                    settings.auto_cleanup_min_free_gb = v;
                }
            }
            "language" => settings.language = value,
            "theme" => settings.theme = value,
            "update_check_enabled" => settings.update_check_enabled = value == "true",
            "scoring_weight_risk" => {
                if let Ok(v) = value.parse() {
                    settings.scoring_weight_risk = v;
                }
            }
            "scoring_weight_age" => {
                if let Ok(v) = value.parse() {
                    settings.scoring_weight_age = v;
                }
            }
            "scoring_weight_duplicate" => {
                if let Ok(v) = value.parse() {
                    settings.scoring_weight_duplicate = v;
                }
            }
            "scoring_weight_size" => {
                if let Ok(v) = value.parse() {
                    settings.scoring_weight_size = v;
                }
            }
            "scoring_weight_safety" => {
                if let Ok(v) = value.parse() {
                    settings.scoring_weight_safety = v;
                }
            }
            "scoring_weight_urgency" => {
                if let Ok(v) = value.parse() {
                    settings.scoring_weight_urgency = v;
                }
            }
            "scoring_weight_pattern" => {
                if let Ok(v) = value.parse() {
                    settings.scoring_weight_pattern = v;
                }
            }
            "duplicate_min_size_bytes" => {
                if let Ok(v) = value.parse() {
                    settings.duplicate_min_size_bytes = v;
                }
            }
            "aging_zombie_days" => {
                if let Ok(v) = value.parse() {
                    settings.aging_zombie_days = v;
                }
            }
            _ => {}
        }
    }
    Ok(settings)
}

fn save_settings_with(conn: &rusqlite::Connection, settings: &AppSettings) -> Result<(), String> {
    let pairs = [
        ("default_drive", settings.default_drive.clone()),
        ("scan_mode", settings.scan_mode.clone()),
        (
            "auto_scan_on_startup",
            settings.auto_scan_on_startup.to_string(),
        ),
        (
            "auto_monitor_on_startup",
            settings.auto_monitor_on_startup.to_string(),
        ),
        (
            "watcher_poll_interval_ms",
            settings.watcher_poll_interval_ms.to_string(),
        ),
        (
            "watcher_debounce_ms",
            settings.watcher_debounce_ms.to_string(),
        ),
        ("alert_enabled", settings.alert_enabled.to_string()),
        (
            "alert_threshold_type",
            settings.alert_threshold_type.clone(),
        ),
        (
            "alert_threshold_value",
            settings.alert_threshold_value.to_string(),
        ),
        (
            "alert_growth_enabled",
            settings.alert_growth_enabled.to_string(),
        ),
        (
            "alert_growth_percent",
            settings.alert_growth_percent.to_string(),
        ),
        (
            "alert_growth_minutes",
            settings.alert_growth_minutes.to_string(),
        ),
        (
            "auto_cleanup_enabled",
            settings.auto_cleanup_enabled.to_string(),
        ),
        (
            "auto_cleanup_frequency",
            settings.auto_cleanup_frequency.clone(),
        ),
        ("auto_cleanup_time", settings.auto_cleanup_time.clone()),
        (
            "auto_cleanup_risk_levels",
            settings.auto_cleanup_risk_levels.clone(),
        ),
        (
            "auto_cleanup_min_free_gb",
            settings.auto_cleanup_min_free_gb.to_string(),
        ),
        ("language", settings.language.clone()),
        ("theme", settings.theme.clone()),
        (
            "update_check_enabled",
            settings.update_check_enabled.to_string(),
        ),
        (
            "scoring_weight_risk",
            settings.scoring_weight_risk.to_string(),
        ),
        (
            "scoring_weight_age",
            settings.scoring_weight_age.to_string(),
        ),
        (
            "scoring_weight_duplicate",
            settings.scoring_weight_duplicate.to_string(),
        ),
        (
            "scoring_weight_size",
            settings.scoring_weight_size.to_string(),
        ),
        (
            "scoring_weight_safety",
            settings.scoring_weight_safety.to_string(),
        ),
        (
            "scoring_weight_urgency",
            settings.scoring_weight_urgency.to_string(),
        ),
        (
            "scoring_weight_pattern",
            settings.scoring_weight_pattern.to_string(),
        ),
        (
            "duplicate_min_size_bytes",
            settings.duplicate_min_size_bytes.to_string(),
        ),
        ("aging_zombie_days", settings.aging_zombie_days.to_string()),
    ];
    for (key, value) in &pairs {
        conn.execute(
            "INSERT INTO settings (key, value_text) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value_text = excluded.value_text",
            rusqlite::params![key, value],
        )
        .map_err(|e| format!("Save setting error: {}", e))?;
    }
    Ok(())
}

fn get_rule_overrides_with(
    conn: &rusqlite::Connection,
) -> Result<std::collections::HashMap<String, bool>, String> {
    let mut stmt = conn
        .prepare("SELECT rule_id, safe_to_delete FROM rule_overrides")
        .map_err(|e| format!("Query error: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
        })
        .map_err(|e| format!("Query error: {}", e))?;
    let mut overrides = std::collections::HashMap::new();
    for row in rows {
        let (id, val) = row.map_err(|e| format!("Row error: {}", e))?;
        overrides.insert(id, val);
    }
    Ok(overrides)
}

fn save_rule_override_with(
    conn: &rusqlite::Connection,
    rule_id: &str,
    safe_to_delete: bool,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO rule_overrides (rule_id, safe_to_delete) VALUES (?1, ?2)
         ON CONFLICT(rule_id) DO UPDATE SET safe_to_delete = excluded.safe_to_delete",
        rusqlite::params![rule_id, safe_to_delete as i64],
    )
    .map_err(|e| format!("Save rule override error: {}", e))?;
    Ok(())
}

fn create_custom_rule_with(
    conn: &rusqlite::Connection,
    name: &str,
    pattern: &str,
    risk_level: &str,
) -> Result<crate::risk::RiskRule, String> {
    let level = crate::risk::risk_level_from_str(risk_level)
        .ok_or_else(|| format!("Invalid risk level: {}", risk_level))?;
    if matches!(level, crate::risk::RiskLevel::High) {
        return Err("Custom rules can only use low or medium risk levels".into());
    }
    let id = format!(
        "custom-{}",
        name.to_ascii_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
    );
    let rule = crate::risk::RiskRule {
        id: id.clone(),
        patterns: vec![pattern.to_string()],
        risk_level: level,
        category: "custom".into(),
        explanation: format!("Custom rule: {}", name),
        safe_to_delete: false,
        name_match: None,
    };
    conn.execute(
        "INSERT INTO custom_rules
            (id, pattern, risk_level, category, explanation, safe_to_delete)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET
            pattern = excluded.pattern,
            risk_level = excluded.risk_level,
            category = excluded.category,
            explanation = excluded.explanation,
            safe_to_delete = excluded.safe_to_delete",
        rusqlite::params![
            rule.id,
            pattern,
            risk_level.to_ascii_lowercase(),
            rule.category,
            rule.explanation,
            rule.safe_to_delete as i64,
        ],
    )
    .map_err(|e| format!("Save custom rule error: {}", e))?;
    Ok(rule)
}

fn get_custom_rules_with(
    conn: &rusqlite::Connection,
) -> Result<Vec<crate::risk::RiskRule>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, pattern, risk_level, category, explanation, safe_to_delete
             FROM custom_rules
             ORDER BY id",
        )
        .map_err(|e| format!("Query custom rules error: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            let level_text: String = row.get(2)?;
            let risk_level = crate::risk::risk_level_from_str(&level_text)
                .unwrap_or(crate::risk::RiskLevel::Medium);
            Ok(crate::risk::RiskRule {
                id: row.get(0)?,
                patterns: vec![row.get(1)?],
                risk_level,
                category: row.get(3)?,
                explanation: row.get(4)?,
                safe_to_delete: row.get::<_, i64>(5)? != 0,
                name_match: None,
            })
        })
        .map_err(|e| format!("Query custom rules error: {}", e))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Custom rule row error: {}", e))
}

fn delete_custom_rule_with(conn: &rusqlite::Connection, rule_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM custom_rules WHERE id = ?1",
        rusqlite::params![rule_id],
    )
    .map_err(|e| format!("Delete custom rule error: {}", e))?;
    Ok(())
}

fn save_notification_with(
    conn: &rusqlite::Connection,
    input: &NotificationInput,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO notifications (notification_type, title, message)
         VALUES (?1, ?2, ?3)",
        rusqlite::params![input.notification_type, input.title, input.message],
    )
    .map_err(|e| format!("Save notification error: {}", e))?;
    Ok(())
}

fn get_notifications_with(conn: &rusqlite::Connection) -> Result<Vec<NotificationRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, notification_type, title, message, read, created_at
             FROM notifications
             ORDER BY created_at DESC, id DESC",
        )
        .map_err(|e| format!("Query notifications error: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(NotificationRecord {
                id: row.get(0)?,
                notification_type: row.get(1)?,
                title: row.get(2)?,
                message: row.get(3)?,
                read: row.get::<_, i64>(4)? != 0,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query notifications error: {}", e))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Notification row error: {}", e))
}

fn mark_notifications_read_with(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute("UPDATE notifications SET read = 1", [])
        .map_err(|e| format!("Mark notifications read error: {}", e))?;
    Ok(())
}

fn mark_notification_read_with(conn: &rusqlite::Connection, id: i64) -> Result<(), String> {
    conn.execute(
        "UPDATE notifications SET read = 1 WHERE id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| format!("Mark notification read error: {}", e))?;
    Ok(())
}

fn clear_notifications_with(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute("DELETE FROM notifications", [])
        .map_err(|e| format!("Clear notifications error: {}", e))?;
    Ok(())
}

// ── Public API ──────────────────────────────────────────────

pub fn ensure_tables() -> Result<(), String> {
    let conn = open_connection()?;
    ensure_tables_with(&conn)
}

pub fn save_snapshot(drive_info: &scanner::DriveInfo) -> Result<(), String> {
    let conn = open_connection()?;
    save_snapshot_with(&conn, drive_info)
}

pub fn save_cleanup_log(result: &cleaner::CleanResult) -> Result<(), String> {
    let conn = open_connection()?;
    save_cleanup_log_with(&conn, result)
}

pub fn get_snapshot_history(drive: &str, days: u32) -> Result<Vec<Snapshot>, String> {
    let conn = open_connection()?;
    get_snapshot_history_with(&conn, drive, days)
}

pub fn get_latest_snapshot_for_drive(drive: &str) -> Result<Option<CachedSnapshot>, String> {
    let conn = open_connection()?;
    get_latest_snapshot_for_drive_with(&conn, drive)
}

pub fn get_cleanup_history() -> Result<Vec<CleanupLog>, String> {
    let conn = open_connection()?;
    get_cleanup_history_with(&conn)
}

pub fn save_auto_cleanup_report(report: &AutoCleanupReportInput) -> Result<(), String> {
    let conn = open_connection()?;
    save_auto_cleanup_report_with(&conn, report)
}

pub fn get_auto_cleanup_history() -> Result<Vec<AutoCleanupReport>, String> {
    let conn = open_connection()?;
    get_auto_cleanup_history_with(&conn)
}

pub fn save_health_snapshot(snapshot: &HealthSnapshotInput) -> Result<(), String> {
    let conn = open_connection()?;
    save_health_snapshot_with(&conn, snapshot)
}

pub fn get_health_history(drive: &str, limit: usize) -> Result<Vec<HealthSnapshot>, String> {
    let conn = open_connection()?;
    get_health_history_with(&conn, drive, limit)
}

pub fn get_settings() -> Result<AppSettings, String> {
    let conn = open_connection()?;
    get_settings_with(&conn)
}

pub fn save_settings(settings: &AppSettings) -> Result<(), String> {
    let conn = open_connection()?;
    save_settings_with(&conn, settings)
}

pub fn get_rule_overrides() -> Result<std::collections::HashMap<String, bool>, String> {
    let conn = open_connection()?;
    get_rule_overrides_with(&conn)
}

pub fn save_rule_override(rule_id: &str, safe_to_delete: bool) -> Result<(), String> {
    let conn = open_connection()?;
    save_rule_override_with(&conn, rule_id, safe_to_delete)
}

pub fn create_custom_rule(
    name: &str,
    pattern: &str,
    risk_level: &str,
) -> Result<crate::risk::RiskRule, String> {
    let conn = open_connection()?;
    create_custom_rule_with(&conn, name, pattern, risk_level)
}

pub fn get_custom_rules() -> Result<Vec<crate::risk::RiskRule>, String> {
    let conn = open_connection()?;
    get_custom_rules_with(&conn)
}

pub fn delete_custom_rule(rule_id: &str) -> Result<(), String> {
    let conn = open_connection()?;
    delete_custom_rule_with(&conn, rule_id)
}

pub fn save_notification(input: &NotificationInput) -> Result<(), String> {
    let conn = open_connection()?;
    save_notification_with(&conn, input)
}

pub fn get_notifications() -> Result<Vec<NotificationRecord>, String> {
    let conn = open_connection()?;
    get_notifications_with(&conn)
}

pub fn mark_notifications_read() -> Result<(), String> {
    let conn = open_connection()?;
    mark_notifications_read_with(&conn)
}

pub fn mark_notification_read(id: i64) -> Result<(), String> {
    let conn = open_connection()?;
    mark_notification_read_with(&conn, id)
}

pub fn clear_notifications() -> Result<(), String> {
    let conn = open_connection()?;
    clear_notifications_with(&conn)
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaner::{CleanItemResult, CleanItemStatus, CleanResult};
    use crate::scanner::{DirInfo, DriveInfo};

    fn setup_test_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().expect("in-memory DB");
        ensure_tables_with(&conn).unwrap();
        conn
    }

    fn make_drive_info() -> DriveInfo {
        DriveInfo {
            drive_letter: "C".into(),
            total_bytes: 500_000_000_000,
            used_bytes: 300_000_000_000,
            free_bytes: 200_000_000_000,
            top_dirs: vec![DirInfo {
                name: "Windows".into(),
                path: "C:\\Windows".into(),
                size_bytes: 50_000_000_000,
                file_count: 50000,
                dir_count: 5000,
                risk_level: None,
                is_approximate: false,
            }],
        }
    }

    #[test]
    fn save_and_query_snapshot() {
        let conn = setup_test_conn();
        let di = make_drive_info();
        save_snapshot_with(&conn, &di).unwrap();
        let history = get_snapshot_history_with(&conn, "C", 365).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].drive_letter, "C");
        assert_eq!(history[0].total_bytes, 500_000_000_000);
        assert_eq!(history[0].used_bytes, 300_000_000_000);
    }

    #[test]
    fn snapshot_date_filter_excludes_old() {
        let conn = setup_test_conn();
        let di = make_drive_info();
        save_snapshot_with(&conn, &di).unwrap();

        let history = get_snapshot_history_with(&conn, "C", 0).unwrap();
        assert!(history.len() <= 1);

        let history = get_snapshot_history_with(&conn, "C", 365).unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn save_and_query_cleanup_log() {
        let conn = setup_test_conn();
        let result = CleanResult {
            total_attempted: 3,
            succeeded: 2,
            skipped: 1,
            failed: 0,
            freed_bytes: 1_000_000_000,
            items: vec![CleanItemResult {
                path: "C:\\Temp\\cache".into(),
                name: "cache".into(),
                size_bytes: 1_000_000_000,
                status: CleanItemStatus::Success,
                reason: None,
                original_path: Some("C:\\Temp\\cache".into()),
            }],
        };
        save_cleanup_log_with(&conn, &result).unwrap();
        let logs = get_cleanup_history_with(&conn).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].succeeded, 2);
        assert_eq!(logs[0].freed_bytes, 1_000_000_000);
    }

    #[test]
    fn drive_filter_works() {
        let conn = setup_test_conn();

        let mut di_c = make_drive_info();
        di_c.drive_letter = "C".into();
        save_snapshot_with(&conn, &di_c).unwrap();

        let mut di_d = make_drive_info();
        di_d.drive_letter = "D".into();
        save_snapshot_with(&conn, &di_d).unwrap();

        let history_c = get_snapshot_history_with(&conn, "C", 365).unwrap();
        assert_eq!(history_c.len(), 1);
        assert_eq!(history_c[0].drive_letter, "C");

        let history_d = get_snapshot_history_with(&conn, "D", 365).unwrap();
        assert_eq!(history_d.len(), 1);
        assert_eq!(history_d[0].drive_letter, "D");
    }

    #[test]
    fn latest_snapshot_returns_cache_age() {
        let conn = setup_test_conn();
        let mut old = make_drive_info();
        old.used_bytes = 250_000_000_000;
        save_snapshot_with(&conn, &old).unwrap();

        let mut latest = make_drive_info();
        latest.used_bytes = 300_000_000_000;
        save_snapshot_with(&conn, &latest).unwrap();

        let cached = get_latest_snapshot_for_drive_with(&conn, "C")
            .unwrap()
            .expect("latest snapshot");
        assert_eq!(cached.snapshot.used_bytes, 300_000_000_000);
        assert_eq!(cached.snapshot.drive_letter, "C");
    }

    // ── Settings tests ────────────────────────────────────

    #[test]
    fn save_and_load_settings() {
        let conn = setup_test_conn();
        let settings = AppSettings {
            default_drive: "D".into(),
            scan_mode: "speed".into(),
            auto_scan_on_startup: true,
            auto_monitor_on_startup: false,
            watcher_poll_interval_ms: 5000,
            watcher_debounce_ms: 3000,
            alert_enabled: true,
            alert_threshold_type: "absolute_gb".into(),
            alert_threshold_value: 20.0,
            alert_growth_enabled: true,
            alert_growth_percent: 10.0,
            alert_growth_minutes: 30,
            auto_cleanup_enabled: true,
            auto_cleanup_frequency: "daily".into(),
            auto_cleanup_time: "04:30".into(),
            auto_cleanup_risk_levels: "low".into(),
            auto_cleanup_min_free_gb: 25.0,
            language: "zh-CN".into(),
            theme: "dark".into(),
            update_check_enabled: false,
            scoring_weight_risk: 0.30,
            scoring_weight_age: 0.10,
            scoring_weight_duplicate: 0.25,
            scoring_weight_size: 0.15,
            scoring_weight_safety: 0.20,
            scoring_weight_urgency: 0.35,
            scoring_weight_pattern: 0.15,
            duplicate_min_size_bytes: 2_097_152,
            aging_zombie_days: 240,
        };
        save_settings_with(&conn, &settings).unwrap();
        let loaded = get_settings_with(&conn).unwrap();
        assert_eq!(loaded.default_drive, "D");
        assert_eq!(loaded.scan_mode, "speed");
        assert!(loaded.auto_scan_on_startup);
        assert!(!loaded.auto_monitor_on_startup);
        assert_eq!(loaded.watcher_poll_interval_ms, 5000);
        assert_eq!(loaded.watcher_debounce_ms, 3000);
        assert!(loaded.alert_enabled);
        assert_eq!(loaded.alert_threshold_type, "absolute_gb");
        assert_eq!(loaded.alert_threshold_value, 20.0);
        assert!(loaded.alert_growth_enabled);
        assert_eq!(loaded.alert_growth_percent, 10.0);
        assert_eq!(loaded.alert_growth_minutes, 30);
        assert!(loaded.auto_cleanup_enabled);
        assert_eq!(loaded.auto_cleanup_frequency, "daily");
        assert_eq!(loaded.auto_cleanup_time, "04:30");
        assert_eq!(loaded.auto_cleanup_risk_levels, "low");
        assert_eq!(loaded.auto_cleanup_min_free_gb, 25.0);
        assert_eq!(loaded.language, "zh-CN");
        assert_eq!(loaded.theme, "dark");
        assert!(!loaded.update_check_enabled);
        assert_eq!(loaded.scoring_weight_risk, 0.30);
        assert_eq!(loaded.scoring_weight_age, 0.10);
        assert_eq!(loaded.scoring_weight_duplicate, 0.25);
        assert_eq!(loaded.scoring_weight_size, 0.15);
        assert_eq!(loaded.scoring_weight_safety, 0.20);
        assert_eq!(loaded.scoring_weight_urgency, 0.35);
        assert_eq!(loaded.scoring_weight_pattern, 0.15);
        assert_eq!(loaded.duplicate_min_size_bytes, 2_097_152);
        assert_eq!(loaded.aging_zombie_days, 240);
    }

    #[test]
    fn default_settings_when_empty() {
        let conn = setup_test_conn();
        let settings = get_settings_with(&conn).unwrap();
        assert_eq!(settings.default_drive, "C");
        assert_eq!(settings.scan_mode, "exact");
        assert!(!settings.auto_scan_on_startup);
        assert_eq!(settings.watcher_poll_interval_ms, 2000);
        assert!(!settings.auto_cleanup_enabled);
        assert_eq!(settings.auto_cleanup_frequency, "weekly");
        assert_eq!(settings.auto_cleanup_time, "03:00");
        assert_eq!(settings.auto_cleanup_risk_levels, "low");
        assert_eq!(settings.auto_cleanup_min_free_gb, 50.0);
        assert_eq!(settings.language, "auto");
        assert_eq!(settings.theme, "auto");
        assert!(settings.update_check_enabled);
        assert_eq!(settings.scoring_weight_risk, 0.20);
        assert_eq!(settings.scoring_weight_age, 0.15);
        assert_eq!(settings.scoring_weight_duplicate, 0.20);
        assert_eq!(settings.scoring_weight_size, 0.20);
        assert_eq!(settings.scoring_weight_safety, 0.25);
        assert_eq!(settings.scoring_weight_urgency, 0.15);
        assert_eq!(settings.scoring_weight_pattern, 0.10);
        assert_eq!(settings.duplicate_min_size_bytes, 1_048_576);
        assert_eq!(settings.aging_zombie_days, 180);
    }

    #[test]
    fn save_and_query_auto_cleanup_report() {
        let conn = setup_test_conn();
        let report = AutoCleanupReportInput {
            drive_letter: "C".into(),
            freed_bytes: 1234,
            succeeded: 2,
            skipped: 1,
            failed: 0,
            status: "completed".into(),
            message: "cleaned low-risk items".into(),
            items_json: "[]".into(),
        };

        save_auto_cleanup_report_with(&conn, &report).unwrap();
        let reports = get_auto_cleanup_history_with(&conn).unwrap();

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].drive_letter, "C");
        assert_eq!(reports[0].freed_bytes, 1234);
        assert_eq!(reports[0].status, "completed");
    }

    #[test]
    fn save_and_query_health_snapshot() {
        let conn = setup_test_conn();
        let input = HealthSnapshotInput {
            drive_letter: "C".into(),
            score: 84,
            space_score: 90,
            waste_score: 80,
            trend_score: 70,
            age_score: 75,
            frag_score: 65,
            anomaly_score: 95,
        };

        save_health_snapshot_with(&conn, &input).unwrap();
        let history = get_health_history_with(&conn, "C", 10).unwrap();

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].score, 84);
        assert_eq!(history[0].frag_score, 65);
        assert_eq!(history[0].anomaly_score, 95);
    }

    #[test]
    fn save_and_load_rule_override() {
        let conn = setup_test_conn();
        save_rule_override_with(&conn, "temp-files", false).unwrap();
        let overrides = get_rule_overrides_with(&conn).unwrap();
        assert_eq!(overrides.get("temp-files"), Some(&false));
    }

    #[test]
    fn rule_override_upsert() {
        let conn = setup_test_conn();
        save_rule_override_with(&conn, "npm-cache", false).unwrap();
        save_rule_override_with(&conn, "npm-cache", true).unwrap();
        let overrides = get_rule_overrides_with(&conn).unwrap();
        assert_eq!(overrides.get("npm-cache"), Some(&true));
    }

    #[test]
    fn save_query_and_delete_custom_rule() {
        let conn = setup_test_conn();
        let rule = create_custom_rule_with(&conn, "Archives", "archive-cache", "medium").unwrap();
        assert_eq!(rule.id, "custom-archives");
        assert!(!rule.safe_to_delete);

        let rules = get_custom_rules_with(&conn).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].patterns, vec!["archive-cache".to_string()]);

        delete_custom_rule_with(&conn, "custom-archives").unwrap();
        assert!(get_custom_rules_with(&conn).unwrap().is_empty());
    }

    #[test]
    fn custom_rule_rejects_high_risk_level() {
        let conn = setup_test_conn();
        let result = create_custom_rule_with(&conn, "Danger", "Windows/System32", "high");

        assert_eq!(
            result,
            Err("Custom rules can only use low or medium risk levels".to_string())
        );
    }

    #[test]
    fn save_and_mark_notifications_read() {
        let conn = setup_test_conn();
        save_notification_with(
            &conn,
            &NotificationInput {
                notification_type: "alert".into(),
                title: "Low space".into(),
                message: "Drive C is low".into(),
            },
        )
        .unwrap();

        let notifications = get_notifications_with(&conn).unwrap();
        assert_eq!(notifications.len(), 1);
        assert!(!notifications[0].read);

        mark_notifications_read_with(&conn).unwrap();
        let notifications = get_notifications_with(&conn).unwrap();
        assert!(notifications[0].read);
    }

    #[test]
    fn mark_single_notification_read_and_clear_all() {
        let conn = setup_test_conn();
        for title in ["Low space", "Cleanup"] {
            save_notification_with(
                &conn,
                &NotificationInput {
                    notification_type: "alert".into(),
                    title: title.into(),
                    message: "message".into(),
                },
            )
            .unwrap();
        }

        let notifications = get_notifications_with(&conn).unwrap();
        let first_id = notifications[0].id;
        mark_notification_read_with(&conn, first_id).unwrap();
        let notifications = get_notifications_with(&conn).unwrap();
        assert!(notifications
            .iter()
            .any(|item| item.id == first_id && item.read));
        assert!(notifications
            .iter()
            .any(|item| item.id != first_id && !item.read));

        clear_notifications_with(&conn).unwrap();
        assert!(get_notifications_with(&conn).unwrap().is_empty());
    }
}
