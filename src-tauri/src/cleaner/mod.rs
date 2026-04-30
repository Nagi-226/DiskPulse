use crate::risk::{RiskItem, RiskLevel};
use serde::{Deserialize, Serialize};

/// A cleanup candidate that is safe enough to preview and validate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanItem {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub risk_level: RiskLevel,
    pub safe_to_delete: bool,
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
}

/// Cleanup execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanResult {
    pub deleted_items: Vec<CleanItem>,
    pub failed_items: Vec<CleanItem>,
    pub deleted_bytes: u64,
    pub failed_reason: Option<String>,
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

fn is_path_allowed(path: &str) -> bool {
    let normalized = format!("{}/", path.replace('\\', "/").to_lowercase());

    normalized.contains("/temp/")
        || normalized.contains("/tmp/")
        || normalized.contains("/cache/")
        || normalized.contains("/.npm/")
        || normalized.contains("/dxcache/")
        || normalized.contains("/deliveryoptimization/")
        || normalized.contains("/downloads/")
        || normalized.contains("/logs/")
}

fn is_path_safe_for_cleanup(path: &str) -> bool {
    let normalized = format!("{}/", path.replace('\\', "/").to_lowercase());
    !(normalized.contains("/windows/")
        || normalized.contains("/program files/")
        || normalized.contains("/program files (x86)/")
        || normalized.contains("/system volume information/")
        || normalized.contains("/$recycle.bin/")
        || normalized.contains("/windows/installer/"))
}

/// Validate cleanup candidates and separate allowed vs blocked items.
pub fn preview_cleanup(items: Vec<CleanItem>) -> CleanPreview {
    let mut accepted = Vec::new();
    let mut blocked = Vec::new();
    let mut total_bytes = 0u64;

    for item in items {
        let allowed = item.safe_to_delete
            && matches!(item.risk_level, RiskLevel::Low | RiskLevel::Medium)
            && is_path_allowed(&item.path)
            && is_path_safe_for_cleanup(&item.path);

        if allowed {
            total_bytes += item.size_bytes;
            accepted.push(item);
        } else {
            blocked.push(item);
        }
    }

    let validation = CleanValidationResult {
        allowed: !accepted.is_empty() && blocked.is_empty(),
        valid_items: accepted.len(),
        blocked_items: blocked.len(),
        total_bytes,
        blocked_reason: if blocked.is_empty() {
            None
        } else {
            Some("One or more items failed the whitelist safety check".into())
        },
    };

    CleanPreview {
        accepted,
        blocked,
        validation,
    }
}

/// Execute cleanup through the Windows Recycle Bin only.
pub fn clean_items(items: Vec<CleanItem>) -> Result<CleanResult, String> {
    let preview = preview_cleanup(items);
    if !preview.validation.allowed {
        return Ok(CleanResult {
            deleted_items: Vec::new(),
            failed_items: preview.accepted.into_iter().chain(preview.blocked).collect(),
            deleted_bytes: 0,
            failed_reason: preview.validation.blocked_reason,
        });
    }

    let mut deleted_items = Vec::new();
    let mut failed_items = Vec::new();
    let mut deleted_bytes = 0u64;

    for item in preview.accepted {
        if recycle_bin_delete(&item.path) {
            deleted_bytes += item.size_bytes;
            deleted_items.push(item);
        } else {
            failed_items.push(item);
        }
    }

    let failed_reason = if failed_items.is_empty() {
        None
    } else {
        Some("Some items could not be moved to the Recycle Bin".into())
    };

    Ok(CleanResult {
        deleted_items,
        failed_items,
        deleted_bytes,
        failed_reason,
    })
}

fn recycle_bin_delete(path: &str) -> bool {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{SHFILEOPSTRUCTW, SHFileOperationW, FO_DELETE, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT};

    let mut wide_path: Vec<u16> = std::ffi::OsStr::new(path).encode_wide().collect();
    wide_path.push(0);
    wide_path.push(0);

    let mut op = SHFILEOPSTRUCTW::default();
    op.wFunc = FO_DELETE;
    op.pFrom = windows::core::PCWSTR(wide_path.as_ptr());
    op.fFlags = (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16;

    unsafe { SHFileOperationW(&mut op) == 0 && !op.fAnyOperationsAborted.as_bool() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(path: &str, safe: bool, risk_level: RiskLevel) -> CleanItem {
        CleanItem {
            name: "item".into(),
            path: path.into(),
            size_bytes: 1024,
            risk_level,
            safe_to_delete: safe,
        }
    }

    #[test]
    fn preview_accepts_whitelisted_safe_item() {
        let preview = preview_cleanup(vec![make_item("C:\\Users\\me\\AppData\\Local\\Temp\\cache", true, RiskLevel::Low)]);
        assert_eq!(preview.validation.valid_items, 1);
        assert_eq!(preview.validation.blocked_items, 0);
        assert!(preview.validation.allowed);
    }

    #[test]
    fn preview_blocks_non_whitelisted_item() {
        let preview = preview_cleanup(vec![make_item("C:\\Program Files\\App", true, RiskLevel::High)]);
        assert_eq!(preview.validation.valid_items, 0);
        assert_eq!(preview.validation.blocked_items, 1);
        assert!(!preview.validation.allowed);
    }
}
