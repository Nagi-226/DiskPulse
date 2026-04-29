use serde::{Deserialize, Serialize};
use std::path::Path;

/// Scan progress snapshot emitted during long scans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub drive_letter: String,
    pub processed: usize,
    pub total: usize,
    pub current_path: Option<String>,
}

/// Drive overview information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub drive_letter: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub top_dirs: Vec<DirInfo>,
}

/// Directory size information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub risk_level: Option<String>,
}

/// Scan a drive and return its overview information.
pub fn scan_drive(drive_letter: &str) -> Result<DriveInfo, String> {
    scan_drive_with_progress(drive_letter, |_| {})
}

/// Scan a drive and report progress through a callback.
pub fn scan_drive_with_progress<F>(drive_letter: &str, mut on_progress: F) -> Result<DriveInfo, String>
where
    F: FnMut(ScanProgress),
{
    let drive_path = format!("{}:\\", drive_letter.to_uppercase());
    let path = Path::new(&drive_path);

    if !path.exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }

    let (total_space, free_space) = get_drive_space(path);
    let used_space = total_space.saturating_sub(free_space);

    let top_dirs = scan_top_level_dirs(path, drive_letter, &mut on_progress)?;

    Ok(DriveInfo {
        drive_letter: drive_letter.to_uppercase(),
        total_bytes: total_space,
        used_bytes: used_space,
        free_bytes: free_space,
        top_dirs,
    })
}

/// Get drive space using Win32 GetDiskFreeSpaceExW
fn get_drive_space(path: &Path) -> (u64, u64) {
    use std::os::windows::ffi::OsStrExt;

    let wide: Vec<u16> = std::ffi::OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut free_bytes_available: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free_bytes: u64 = 0;

    let result = unsafe {
        windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            windows::core::PCWSTR(wide.as_ptr()),
            Some(&mut free_bytes_available as *mut u64 as *mut _),
            Some(&mut total_bytes as *mut u64 as *mut _),
            Some(&mut total_free_bytes as *mut u64 as *mut _),
        )
    };

    if result.is_ok() {
        (total_bytes, free_bytes_available)
    } else {
        (0, 0)
    }
}

/// Scan top-level directories of a drive
fn scan_top_level_dirs<F>(
    root: &Path,
    drive_letter: &str,
    on_progress: &mut F,
) -> Result<Vec<DirInfo>, String>
where
    F: FnMut(ScanProgress),
{
    let mut dirs: Vec<DirInfo> = Vec::new();

    let entries = std::fs::read_dir(root).map_err(|e| format!("Cannot read root: {}", e))?;
    let entries: Vec<_> = entries.flatten().collect();
    let total = entries.len();
    let mut processed = 0usize;

    for entry in entries {
        processed += 1;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if !path.is_dir() {
            continue;
        }

        if is_protected_root_dir(&name) {
            continue;
        }

        let (size, file_count, dir_count) = calculate_dir_size(&path);

        dirs.push(DirInfo {
            name,
            path: path.to_string_lossy().to_string(),
            size_bytes: size,
            file_count,
            dir_count,
            risk_level: None,
        });

        on_progress(ScanProgress {
            drive_letter: drive_letter.to_uppercase(),
            processed,
            total,
            current_path: Some(path.to_string_lossy().to_string()),
        });
    }

    dirs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(dirs)
}

fn is_protected_root_dir(name: &str) -> bool {
    matches!(name, "System Volume Information" | "$Recycle.Bin")
}

/// Recursively calculate directory size using walkdir + rayon
fn calculate_dir_size(path: &Path) -> (u64, u64, u64) {
    use rayon::prelude::*;
    use walkdir::WalkDir;

    let entries: Vec<walkdir::DirEntry> = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .collect();

    let file_size: u64 = entries
        .par_iter()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();

    let file_count = entries.iter().filter(|e| e.file_type().is_file()).count() as u64;
    let dir_count = entries
        .iter()
        .filter(|e| e.file_type().is_dir() && e.path() != path)
        .count() as u64;

    (file_size, file_count, dir_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_dirs_are_skipped() {
        assert!(is_protected_root_dir("System Volume Information"));
        assert!(is_protected_root_dir("$Recycle.Bin"));
        assert!(!is_protected_root_dir("Users"));
    }

    #[test]
    fn progress_serializes() {
        let progress = ScanProgress {
            drive_letter: "C".to_string(),
            processed: 1,
            total: 10,
            current_path: Some(r"C:\Users".to_string()),
        };

        let json = serde_json::to_string(&progress).expect("serialize progress");
        assert!(json.contains("\"processed\":1"));
        assert!(json.contains("\"total\":10"));
    }

    #[test]
    fn scan_drive_fails_for_missing_drive() {
        let result = scan_drive("Z");
        assert!(result.is_err());
    }
}
