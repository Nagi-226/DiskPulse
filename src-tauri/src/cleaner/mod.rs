use crate::risk::{RiskItem, RiskLevel};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A cleanup candidate validated for preview and execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanItem {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub risk_level: RiskLevel,
    pub safe_to_delete: bool,
}

/// Per-item cleanup status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CleanItemStatus {
    /// Successfully moved to Recycle Bin.
    Success,
    /// Skipped — no error, but blocked by safety rules.
    Skipped,
    /// Failed with an error reason.
    Failed,
}

/// Detailed result for a single cleaned item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanItemResult {
    pub path: String,
    pub name: String,
    pub size_bytes: u64,
    pub status: CleanItemStatus,
    pub reason: Option<String>,
}

/// Validation result before any cleanup action is allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanValidationResult {
    pub allowed: bool,
    pub valid_items: usize,
    pub blocked_items: usize,
    pub total_bytes: u64,
    pub blocked_reason: Option<String>,
}

/// Cleanup preview payload returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanPreview {
    pub accepted: Vec<CleanItem>,
    pub blocked: Vec<CleanItem>,
    pub validation: CleanValidationResult,
    /// Items that failed pre-delete safety checks (locked files, missing paths, etc.)
    pub unsafe_items: Vec<CleanItemResult>,
}

/// Cleanup execution result with per-item tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanResult {
    pub total_attempted: usize,
    pub succeeded: usize,
    pub skipped: usize,
    pub failed: usize,
    pub freed_bytes: u64,
    pub items: Vec<CleanItemResult>,
}

/// Progress event emitted during batch cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanProgress {
    pub current: usize,
    pub total: usize,
    pub current_item: Option<String>,
    pub status: Option<String>,
}

impl From<&RiskItem> for CleanItem {
    fn from(item: &RiskItem) -> Self {
        Self {
            name: item.name.clone(),
            path: item.path.clone(),
            size_bytes: item.size_bytes,
            risk_level: item.risk_level.clone(),
            safe_to_delete: item.safe_to_delete,
        }
    }
}

// ── Safety validation ──────────────────────────────────────

fn is_path_allowed(path: &str) -> bool {
    let normalized = format!("/{}/", path.replace('\\', "/").to_lowercase());

    normalized.contains("/temp/")
        || normalized.contains("/tmp/")
        || normalized.contains("/cache/")
        || normalized.contains("npm-cache")
        || normalized.contains(".npm")
        || normalized.contains("/dxcache/")
        || normalized.contains("/deliveryoptimization/")
        || normalized.contains("/downloads/")
        || normalized.contains("/logs/")
}

fn is_path_safe_for_cleanup(path: &str) -> bool {
    let normalized = format!("/{}/", path.replace('\\', "/").to_lowercase());
    !(normalized.contains("/windows/")
        || normalized.contains("/program files/")
        || normalized.contains("/program files (x86)/")
        || normalized.contains("/system volume information/")
        || normalized.contains("/$recycle.bin/")
        || normalized.contains("/windows/installer/"))
}

fn check_path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

fn check_file_locked(path: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_SHARE_READ,
        FILE_SHARE_WRITE, FILE_SHARE_DELETE, OPEN_EXISTING,
    };

    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let handle: Result<HANDLE, _> = unsafe {
        CreateFileW(
            PCWSTR(wide.as_ptr()),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            None,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            None,
        )
        .map_err(|_| ())
    };

    if let Ok(h) = handle {
        unsafe { windows::Win32::Foundation::CloseHandle(h).ok() };
        false
    } else {
        true
    }
}

/// Rule-based checks only — no filesystem access. Used during preview.
fn rule_check(item: &CleanItem) -> Result<(), String> {
    if !is_path_safe_for_cleanup(&item.path) {
        return Err("Path is in a system-protected location".into());
    }
    if !is_path_allowed(&item.path) {
        return Err("Path does not match whitelist pattern".into());
    }
    Ok(())
}

/// Filesystem checks — run right before deletion.
fn runtime_check(item: &CleanItem) -> Result<(), String> {
    if !check_path_exists(&item.path) {
        return Err("Path no longer exists".into());
    }
    if check_file_locked(&item.path) {
        return Err("File is locked or in use by another process".into());
    }
    Ok(())
}

/// Run all pre-delete checks (rule + runtime).
fn safety_check(item: &CleanItem) -> Result<(), String> {
    rule_check(item)?;
    runtime_check(item)?;
    Ok(())
}

// ── Validation & preview ───────────────────────────────────

/// Validate cleanup candidates and separate allowed vs blocked items.
/// Also runs pre-delete safety checks (existence, file lock).
pub fn preview_cleanup(items: Vec<CleanItem>) -> CleanPreview {
    let mut accepted = Vec::new();
    let mut blocked = Vec::new();
    let mut unsafe_items = Vec::new();
    let mut total_bytes = 0u64;

    for item in items {
        let allowed = item.safe_to_delete
            && matches!(item.risk_level, RiskLevel::Low | RiskLevel::Medium)
            && is_path_allowed(&item.path)
            && is_path_safe_for_cleanup(&item.path);

        if !allowed {
            blocked.push(item);
            continue;
        }

        // Rule-based checks only (no filesystem access)
        if let Err(reason) = rule_check(&item) {
            unsafe_items.push(CleanItemResult {
                path: item.path.clone(),
                name: item.name.clone(),
                size_bytes: item.size_bytes,
                status: CleanItemStatus::Skipped,
                reason: Some(reason),
            });
            blocked.push(item);
            continue;
        }

        total_bytes += item.size_bytes;
        accepted.push(item);
    }

    let validation = CleanValidationResult {
        allowed: !accepted.is_empty(),
        valid_items: accepted.len(),
        blocked_items: blocked.len(),
        total_bytes,
        blocked_reason: if blocked.is_empty() {
            None
        } else {
            Some(format!(
                "{} item(s) blocked by safety rules (including {} with pre-delete issues)",
                blocked.len(),
                unsafe_items.len()
            ))
        },
    };

    CleanPreview {
        accepted,
        blocked,
        validation,
        unsafe_items,
    }
}

// ── Recycle Bin operations ─────────────────────────────────

/// Move a file or directory to the Recycle Bin via SHFileOperationW.
/// Returns true on success.
fn recycle_bin_delete(path: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, SHFILEOPSTRUCTW, FO_DELETE, FOF_ALLOWUNDO,
        FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT,
    };

    let wide_path: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .chain(std::iter::once(0))
        .collect();

    let mut op = SHFILEOPSTRUCTW::default();
    op.wFunc = FO_DELETE;
    op.pFrom = windows::core::PCWSTR(wide_path.as_ptr());
    op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16;

    unsafe { SHFileOperationW(&mut op) == 0 && !op.fAnyOperationsAborted.as_bool() }
}

// ── Cleanup execution ──────────────────────────────────────

/// Execute cleanup with progress reporting and cancellation support.
///
/// `cancel_token` — when set to `true`, stops processing after the current item completes.
/// `on_progress` — called after each item to report current/total.
pub fn clean_items_with_progress<F>(
    items: Vec<CleanItem>,
    cancel_token: Option<Arc<AtomicBool>>,
    mut on_progress: F,
) -> CleanResult
where
    F: FnMut(CleanProgress),
{
    let preview = preview_cleanup(items);
    let total = preview.accepted.len();

    if total == 0 {
        return CleanResult {
            total_attempted: 0,
            succeeded: 0,
            skipped: preview.unsafe_items.len(),
            failed: 0,
            freed_bytes: 0,
            items: preview.unsafe_items,
        };
    }

    let mut item_results = Vec::with_capacity(total);
    let mut succeeded = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut freed_bytes = 0u64;

    // Push any pre-check failures first
    for r in preview.unsafe_items {
        skipped += 1;
        item_results.push(r);
    }

    for (i, item) in preview.accepted.into_iter().enumerate() {
        // Check cancellation
        if let Some(ref token) = cancel_token {
            if token.load(Ordering::Relaxed) {
                item_results.push(CleanItemResult {
                    path: item.path.clone(),
                    name: item.name.clone(),
                    size_bytes: item.size_bytes,
                    status: CleanItemStatus::Skipped,
                    reason: Some("Cancelled by user".into()),
                });
                skipped += 1;
                on_progress(CleanProgress {
                    current: i + 1,
                    total,
                    current_item: Some(item.name.clone()),
                    status: Some("cancelled".into()),
                });
                break;
            }
        }

        on_progress(CleanProgress {
            current: i + 1,
            total,
            current_item: Some(item.name.clone()),
            status: Some("deleting".into()),
        });

        // Full safety check (rule + runtime) right before deletion
        if let Err(reason) = safety_check(&item) {
            item_results.push(CleanItemResult {
                path: item.path,
                name: item.name,
                size_bytes: item.size_bytes,
                status: CleanItemStatus::Skipped,
                reason: Some(reason),
            });
            skipped += 1;
            continue;
        }

        let success = recycle_bin_delete(&item.path);

        if success {
            freed_bytes += item.size_bytes;
            succeeded += 1;
            item_results.push(CleanItemResult {
                path: item.path,
                name: item.name,
                size_bytes: item.size_bytes,
                status: CleanItemStatus::Success,
                reason: None,
            });
        } else {
            failed += 1;
            item_results.push(CleanItemResult {
                path: item.path,
                name: item.name,
                size_bytes: item.size_bytes,
                status: CleanItemStatus::Failed,
                reason: Some("SHFileOperationW failed — file may be in use".into()),
            });
        }
    }

    CleanResult {
        total_attempted: total,
        succeeded,
        skipped,
        failed,
        freed_bytes,
        items: item_results,
    }
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(path: &str, safe: bool, risk_level: RiskLevel) -> CleanItem {
        CleanItem {
            name: path.split('\\').last().unwrap_or("item").into(),
            path: path.into(),
            size_bytes: 1024,
            risk_level,
            safe_to_delete: safe,
        }
    }

    // ── preview_cleanup ──

    #[test]
    fn preview_accepts_whitelisted_safe_item() {
        let preview = preview_cleanup(vec![make_item(
            "C:\\Users\\me\\AppData\\Local\\Temp\\cache",
            true,
            RiskLevel::Low,
        )]);
        assert_eq!(preview.validation.valid_items, 1);
        assert_eq!(preview.validation.blocked_items, 0);
        assert!(preview.validation.allowed);
    }

    #[test]
    fn preview_blocks_non_whitelisted_item() {
        let preview = preview_cleanup(vec![make_item(
            "C:\\Program Files\\App",
            true,
            RiskLevel::High,
        )]);
        assert_eq!(preview.validation.valid_items, 0);
        assert_eq!(preview.validation.blocked_items, 1);
        assert!(!preview.validation.allowed);
    }

    #[test]
    fn preview_blocks_missing_path() {
        let preview = preview_cleanup(vec![make_item(
            "Z:\\Definitely\\Does\\Not\\Exist\\Dir",
            true,
            RiskLevel::Low,
        )]);
        assert_eq!(preview.validation.valid_items, 0);
        assert!(!preview.validation.allowed);
    }

    #[test]
    fn preview_handles_empty_list() {
        let preview = preview_cleanup(vec![]);
        assert_eq!(preview.validation.valid_items, 0);
        assert_eq!(preview.validation.blocked_items, 0);
        assert!(!preview.validation.allowed);
    }

    #[test]
    fn preview_blocks_system_paths() {
        let system_paths = vec![
            "C:\\Windows\\System32",
            "C:\\Program Files\\App",
            "C:\\Program Files (x86)\\App",
            "C:\\System Volume Information",
            "C:\\$Recycle.Bin\\stuff",
            "C:\\Windows\\Installer\\blah",
        ];
        for path in system_paths {
            let preview = preview_cleanup(vec![make_item(path, true, RiskLevel::Low)]);
            assert!(
                !preview.validation.allowed,
                "Path should be blocked: {}",
                path
            );
        }
    }

    // ── safety_check ──

    #[test]
    fn safety_check_fails_for_nonexistent_path() {
        let item = make_item("Z:\\NoSuchDir", true, RiskLevel::Low);
        let result = safety_check(&item);
        assert!(result.is_err());
    }

    #[test]
    fn safety_check_blocks_system_protected() {
        let item = make_item("C:\\Windows\\System32\\fake-file.dll", true, RiskLevel::Low);
        let result = safety_check(&item);
        assert!(result.is_err());
    }

    #[test]
    fn empty_items_list_returns_immediately() {
        let result = clean_items_with_progress(vec![], None, |_| {});
        assert_eq!(result.total_attempted, 0);
        assert_eq!(result.succeeded, 0);
        assert!(result.items.is_empty());
    }

    // ── cancellation ──

    #[test]
    fn cancellation_stops_cleanly() {
        let cancel = Arc::new(AtomicBool::new(false));
        // Cancel immediately
        cancel.store(true, Ordering::Relaxed);

        let items = vec![make_item(
            "C:\\Users\\me\\AppData\\Local\\Temp\\test",
            true,
            RiskLevel::Low,
        )];

        let result = clean_items_with_progress(items, Some(cancel.clone()), |_| {});
        assert_eq!(result.succeeded, 0);
        assert!(result.items.iter().any(|r| r.reason.as_deref() == Some("Cancelled by user")));
    }

    // ── is_path_allowed / is_path_safe ──

    #[test]
    fn whitelist_matches_temp_paths() {
        assert!(is_path_allowed("C:\\Users\\me\\AppData\\Local\\Temp\\stuff"));
    }

    #[test]
    fn whitelist_matches_cache_paths() {
        assert!(is_path_allowed("C:\\Users\\me\\AppData\\Local\\npm-cache\\lodash"));
    }

    #[test]
    fn whitelist_rejects_unknown_paths() {
        assert!(!is_path_allowed("C:\\MyImportantData"));
    }

    #[test]
    fn safe_check_blocks_windows() {
        assert!(!is_path_safe_for_cleanup("C:\\Windows\\some-file"));
    }
}
