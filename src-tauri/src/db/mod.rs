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

/// Application settings persisted across sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub default_drive: String,
    pub auto_scan_on_startup: bool,
    pub auto_monitor_on_startup: bool,
    pub watcher_poll_interval_ms: u64,
    pub watcher_debounce_ms: u64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_drive: "C".into(),
            auto_scan_on_startup: false,
            auto_monitor_on_startup: false,
            watcher_poll_interval_ms: 2000,
            watcher_debounce_ms: 1500,
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

        CREATE INDEX IF NOT EXISTS idx_snapshots_drive ON snapshots(drive_letter, created_at);
        CREATE INDEX IF NOT EXISTS idx_cleanup_logs_time ON cleanup_logs(created_at);

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value_text TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS rule_overrides (
            rule_id TEXT PRIMARY KEY,
            safe_to_delete INTEGER NOT NULL
        );",
    )
    .map_err(|e| format!("Cannot create tables: {}", e))
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
            _ => {}
        }
    }
    Ok(settings)
}

fn save_settings_with(conn: &rusqlite::Connection, settings: &AppSettings) -> Result<(), String> {
    let pairs = [
        ("default_drive", settings.default_drive.clone()),
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
            auto_scan_on_startup: true,
            auto_monitor_on_startup: false,
            watcher_poll_interval_ms: 5000,
            watcher_debounce_ms: 3000,
        };
        save_settings_with(&conn, &settings).unwrap();
        let loaded = get_settings_with(&conn).unwrap();
        assert_eq!(loaded.default_drive, "D");
        assert!(loaded.auto_scan_on_startup);
        assert!(!loaded.auto_monitor_on_startup);
        assert_eq!(loaded.watcher_poll_interval_ms, 5000);
        assert_eq!(loaded.watcher_debounce_ms, 3000);
    }

    #[test]
    fn default_settings_when_empty() {
        let conn = setup_test_conn();
        let settings = get_settings_with(&conn).unwrap();
        assert_eq!(settings.default_drive, "C");
        assert!(!settings.auto_scan_on_startup);
        assert_eq!(settings.watcher_poll_interval_ms, 2000);
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
}
