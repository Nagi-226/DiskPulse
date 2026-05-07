use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Phase of the drive scan pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanPhase {
    Walking,
    Measuring,
    Complete,
}

/// Scan progress snapshot emitted during long scans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub drive_letter: String,
    pub processed: usize,
    pub total: usize,
    pub current_path: Option<String>,
    pub phase: ScanPhase,
    pub partial_results: Option<Vec<DirInfo>>,
}

/// Fast drive metadata returned before the expensive directory walk finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveMeta {
    pub drive_letter: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub cached_top_dirs: Option<Vec<DirInfo>>,
    pub cache_age_ms: Option<u64>,
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

/// Scan a drive and return its overview information (used in tests).
#[allow(dead_code)]
pub fn scan_drive(drive_letter: &str) -> Result<DriveInfo, String> {
    scan_drive_with_progress(drive_letter, |_| {})
}

/// Return drive space immediately, optionally enriched with cached top-level dirs.
pub fn scan_drive_meta(
    drive_letter: &str,
    cached_top_dirs: Option<Vec<DirInfo>>,
    cache_age_ms: Option<u64>,
) -> Result<DriveMeta, String> {
    let drive_path = format!("{}:\\", drive_letter.to_uppercase());
    let path = Path::new(&drive_path);

    if !path.exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }

    let (total_space, free_space) = get_drive_space(path);
    let used_space = total_space.saturating_sub(free_space);

    Ok(DriveMeta {
        drive_letter: drive_letter.to_uppercase(),
        total_bytes: total_space,
        used_bytes: used_space,
        free_bytes: free_space,
        cached_top_dirs,
        cache_age_ms,
    })
}

/// Scan a drive and report progress through a callback.
pub fn scan_drive_with_progress<F>(drive_letter: &str, on_progress: F) -> Result<DriveInfo, String>
where
    F: Fn(ScanProgress) + Sync,
{
    scan_drive_with_progress_and_cancel(drive_letter, on_progress, None)
}

/// Scan a drive and support cancellation through a shared flag.
pub fn scan_drive_with_progress_and_cancel<F>(
    drive_letter: &str,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<DriveInfo, String>
where
    F: Fn(ScanProgress) + Sync,
{
    let drive_path = format!("{}:\\", drive_letter.to_uppercase());
    let path = Path::new(&drive_path);

    if !path.exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }

    let (total_space, free_space) = get_drive_space(path);
    let used_space = total_space.saturating_sub(free_space);

    let top_dirs = scan_top_level_dirs(path, drive_letter, &on_progress, cancel)?;

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
    on_progress: &F,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<DirInfo>, String>
where
    F: Fn(ScanProgress) + Sync,
{
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let entries = std::fs::read_dir(root).map_err(|e| format!("Cannot read root: {}", e))?;
    let entries: Vec<_> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_symlink = e.file_type().map(|ft| ft.is_symlink()).unwrap_or(false);
            let path = e.path();
            if path.is_dir() && !is_symlink && !is_protected_root_dir(&name) {
                Some((path, name))
            } else {
                None
            }
        })
        .collect();
    let total = entries.len();
    let processed = AtomicUsize::new(0);
    let drive_letter = drive_letter.to_uppercase();

    on_progress(ScanProgress {
        drive_letter: drive_letter.clone(),
        processed: 0,
        total,
        current_path: Some(root.to_string_lossy().to_string()),
        phase: ScanPhase::Walking,
        partial_results: None,
    });

    let mut dirs: Vec<DirInfo> = entries
        .par_iter()
        .map(|(path, name)| {
            if is_cancelled(cancel) {
                return Err("Scan cancelled".to_string());
            }

            on_progress(ScanProgress {
                drive_letter: drive_letter.clone(),
                processed: processed.load(Ordering::Relaxed),
                total,
                current_path: Some(path.to_string_lossy().to_string()),
                phase: ScanPhase::Measuring,
                partial_results: None,
            });

            let (size, file_count, dir_count) = calculate_dir_size(path, cancel)?;

            let dir = DirInfo {
                name: name.clone(),
                path: path.to_string_lossy().to_string(),
                size_bytes: size,
                file_count,
                dir_count,
                risk_level: None,
            };
            let done = processed.fetch_add(1, Ordering::SeqCst) + 1;

            on_progress(ScanProgress {
                drive_letter: drive_letter.clone(),
                processed: done,
                total,
                current_path: Some(path.to_string_lossy().to_string()),
                phase: ScanPhase::Complete,
                partial_results: Some(vec![dir.clone()]),
            });

            Ok(dir)
        })
        .collect::<Result<Vec<_>, _>>()?;

    dirs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(dirs)
}

fn is_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel
        .map(|flag| flag.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn is_protected_root_dir(name: &str) -> bool {
    matches!(name, "System Volume Information" | "$Recycle.Bin")
}

/// Re-scan one cached top-level directory without walking the whole drive.
pub fn scan_top_level_dir(path: &str, cancel: Option<&AtomicBool>) -> Result<DirInfo, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| format!("Cannot determine directory name: {}", path.display()))?;
    let (size, file_count, dir_count) = calculate_dir_size(path, cancel)?;

    Ok(DirInfo {
        name,
        path: path.to_string_lossy().to_string(),
        size_bytes: size,
        file_count,
        dir_count,
        risk_level: None,
    })
}

/// Scan a specific directory for subdirectories (used for drill-down navigation)
pub fn scan_directory(path: &str) -> Result<Vec<DirInfo>, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("Directory does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Not a directory: {}", path.display()));
    }

    let entries: Vec<_> = std::fs::read_dir(path)
        .map_err(|e| format!("Cannot read directory: {}", e))?
        .flatten()
        .collect();

    let mut dirs: Vec<DirInfo> = Vec::new();
    let mut files_size: u64 = 0;
    let mut files_count: u64 = 0;

    for entry in entries {
        let entry_path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if entry_path.is_dir() {
            if name.starts_with('$') || name == "System Volume Information" {
                continue;
            }
            let (size, file_count, dir_count) = calculate_dir_size(&entry_path, None)?;
            dirs.push(DirInfo {
                name,
                path: entry_path.to_string_lossy().to_string(),
                size_bytes: size,
                file_count,
                dir_count,
                risk_level: None,
            });
        } else {
            // Count direct files in this directory
            if let Ok(meta) = entry_path.metadata() {
                files_size += meta.len();
                files_count += 1;
            }
        }
    }

    // If there are direct files, add a virtual entry for them
    if files_count > 0 {
        dirs.push(DirInfo {
            name: "(files)".into(),
            path: path.to_string_lossy().to_string(),
            size_bytes: files_size,
            file_count: files_count,
            dir_count: 0,
            risk_level: None,
        });
    }

    dirs.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    Ok(dirs)
}

/// Recursively calculate directory size using walkdir + rayon
fn calculate_dir_size(path: &Path, cancel: Option<&AtomicBool>) -> Result<(u64, u64, u64), String> {
    use rayon::prelude::*;
    use walkdir::WalkDir;

    let entries: Vec<walkdir::DirEntry> = WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .take_while(|_| !is_cancelled(cancel))
        .filter_map(|e| e.ok())
        .collect();

    if is_cancelled(cancel) {
        return Err("Scan cancelled".to_string());
    }

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

    Ok((file_size, file_count, dir_count))
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
            phase: ScanPhase::Measuring,
            partial_results: None,
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

    #[test]
    fn scan_top_level_dir_counts_nested_files() {
        let root =
            std::env::temp_dir().join(format!("diskpulse-scan-top-dir-{}", std::process::id()));
        let nested = root.join("nested");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&nested).expect("create nested test dir");
        std::fs::write(root.join("a.txt"), b"abc").expect("write root file");
        std::fs::write(nested.join("b.txt"), b"hello").expect("write nested file");

        let info = scan_top_level_dir(&root.to_string_lossy(), None).expect("scan temp dir");
        assert_eq!(info.size_bytes, 8);
        assert_eq!(info.file_count, 2);
        assert_eq!(info.dir_count, 1);

        let _ = std::fs::remove_dir_all(&root);
    }
}
