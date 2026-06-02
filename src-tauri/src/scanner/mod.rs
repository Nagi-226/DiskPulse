use serde::{Deserialize, Serialize};
use std::cmp::{Ordering as CmpOrdering, Reverse};
use std::collections::BinaryHeap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

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

/// Shared input for pluggable scan stages.
pub struct ScanContext<'a> {
    pub root: PathBuf,
    pub cancel: Option<&'a AtomicBool>,
}

/// Common output for directory measurement stages.
pub struct ScanOutput {
    pub size_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
}

/// Extension point for replacing scan phases without rewriting callers.
pub trait ScanStage {
    fn execute(&self, ctx: &ScanContext<'_>) -> Result<ScanOutput, String>;
}

/// Default measurement stage backed by the current walker implementation.
pub struct MeasureStage;

impl ScanStage for MeasureStage {
    fn execute(&self, ctx: &ScanContext<'_>) -> Result<ScanOutput, String> {
        let (size_bytes, file_count, dir_count) = calculate_dir_size(&ctx.root, ctx.cancel)?;
        Ok(ScanOutput {
            size_bytes,
            file_count,
            dir_count,
        })
    }
}

/// Individual file candidate returned by the large file finder.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_epoch_ms: u64,
    #[serde(default)]
    pub hard_link_count: u64,
    #[serde(default)]
    pub size_on_disk_bytes: Option<u64>,
}

impl Ord for FileEntry {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        (
            self.size_bytes,
            self.modified_epoch_ms,
            self.hard_link_count,
            self.size_on_disk_bytes,
            &self.path,
            &self.name,
        )
            .cmp(&(
                other.size_bytes,
                other.modified_epoch_ms,
                other.hard_link_count,
                other.size_on_disk_bytes,
                &other.path,
                &other.name,
            ))
    }
}

impl PartialOrd for FileEntry {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

/// Progress snapshot emitted by the large file finder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeFileProgress {
    pub drive_letter: String,
    pub dirs_processed: usize,
    pub dirs_total: usize,
    pub files_found: usize,
    pub current_path: Option<String>,
}

/// Scan a drive and return its overview information.
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

fn get_drive_space(path: &Path) -> (u64, u64) {
    let drive = drive_key_from_path(path);
    let providers = crate::platform::providers();
    let total_bytes = providers.disk_info.total_bytes(&drive).unwrap_or(0);
    let free_bytes = providers.disk_info.free_bytes(&drive).unwrap_or(0);
    (total_bytes, free_bytes)
}

fn drive_key_from_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if text.len() >= 2 && text.as_bytes().get(1) == Some(&b':') {
        text[..1].to_ascii_uppercase()
    } else {
        text.to_string()
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

            let providers = crate::platform::providers();
            let output = providers.dir_scanner.execute(&ScanContext {
                root: path.clone(),
                cancel,
            })?;

            let dir = DirInfo {
                name: name.clone(),
                path: path.to_string_lossy().to_string(),
                size_bytes: output.size_bytes,
                file_count: output.file_count,
                dir_count: output.dir_count,
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

/// Shared cancellation check used by scanner, duplicates, and aging modules.
pub fn is_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel
        .map(|flag| flag.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn is_protected_root_dir(name: &str) -> bool {
    matches!(name, "System Volume Information" | "$Recycle.Bin")
}

/// Scan a drive for the largest individual files above a minimum size.
pub fn find_large_files_with_progress_and_cancel<F>(
    drive_letter: &str,
    min_size: u64,
    limit: usize,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<FileEntry>, String>
where
    F: Fn(LargeFileProgress),
{
    let drive_path = format!("{}:\\", drive_letter.to_uppercase());
    let path = Path::new(&drive_path);

    if !path.exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }

    find_large_files_under_root(path, drive_letter, min_size, limit, on_progress, cancel)
}

fn find_large_files_under_root<F>(
    root: &Path,
    drive_letter: &str,
    min_size: u64,
    limit: usize,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<FileEntry>, String>
where
    F: Fn(LargeFileProgress),
{
    if limit == 0 {
        return Ok(Vec::new());
    }
    if !root.exists() {
        return Err(format!("Directory does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()));
    }

    let mut top_dirs = Vec::new();
    let mut root_files = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|e| format!("Cannot read root: {}", e))? {
        let Ok(entry) = entry else {
            continue;
        };
        let file_type = entry.file_type().ok();
        let is_symlink = file_type
            .as_ref()
            .map(|ft| ft.is_symlink())
            .unwrap_or(false);
        if is_symlink {
            continue;
        }

        let path = entry.path();
        if file_type.as_ref().map(|ft| ft.is_dir()).unwrap_or(false) {
            let name = entry.file_name().to_string_lossy().to_string();
            if !is_protected_root_dir(&name) {
                top_dirs.push(path);
            }
        } else if file_type.as_ref().map(|ft| ft.is_file()).unwrap_or(false) {
            root_files.push(path);
        }
    }

    let drive_letter = drive_letter.to_uppercase();
    let dirs_total = top_dirs.len();
    let mut dirs_processed = 0usize;
    let mut heap: BinaryHeap<Reverse<FileEntry>> = BinaryHeap::new();

    emit_large_file_progress(
        &on_progress,
        &drive_letter,
        dirs_processed,
        dirs_total,
        heap.len(),
        Some(root),
    );
    if is_cancelled(cancel) {
        return Err("Large file scan cancelled".to_string());
    }

    for file_path in root_files {
        if is_cancelled(cancel) {
            return Err("Large file scan cancelled".to_string());
        }
        push_file_candidate(&mut heap, &file_path, min_size, limit);
    }

    emit_large_file_progress(
        &on_progress,
        &drive_letter,
        dirs_processed,
        dirs_total,
        heap.len(),
        Some(root),
    );
    if is_cancelled(cancel) {
        return Err("Large file scan cancelled".to_string());
    }

    for top_dir in top_dirs {
        if is_cancelled(cancel) {
            return Err("Large file scan cancelled".to_string());
        }

        for entry in jwalk::WalkDir::new(&top_dir).follow_links(false) {
            if is_cancelled(cancel) {
                return Err("Large file scan cancelled".to_string());
            }

            let Ok(entry) = entry else {
                continue;
            };
            if entry.file_type().is_file() {
                push_file_candidate(&mut heap, &entry.path(), min_size, limit);
            }
        }

        dirs_processed += 1;
        emit_large_file_progress(
            &on_progress,
            &drive_letter,
            dirs_processed,
            dirs_total,
            heap.len(),
            Some(&top_dir),
        );
        if is_cancelled(cancel) {
            return Err("Large file scan cancelled".to_string());
        }
    }

    let mut files: Vec<FileEntry> = heap.into_iter().map(|Reverse(entry)| entry).collect();
    files.sort_by(|a, b| b.cmp(a));
    Ok(files)
}

fn push_file_candidate(
    heap: &mut BinaryHeap<Reverse<FileEntry>>,
    path: &Path,
    min_size: u64,
    limit: usize,
) {
    let Ok(metadata) = path.metadata() else {
        return;
    };
    if !metadata.is_file() || metadata.len() < min_size {
        return;
    }

    let path_text = path.to_string_lossy().to_string();
    let meta = crate::platform::providers().file_meta;
    let entry = FileEntry {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: path_text.clone(),
        size_bytes: metadata.len(),
        modified_epoch_ms: modified_epoch_ms(&metadata),
        hard_link_count: meta.hard_link_count(&path_text).unwrap_or(1),
        size_on_disk_bytes: meta.size_on_disk(&path_text).ok().flatten(),
    };

    heap.push(Reverse(entry));
    if heap.len() > limit {
        let _ = heap.pop();
    }
}

fn modified_epoch_ms(metadata: &std::fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn emit_large_file_progress<F>(
    on_progress: &F,
    drive_letter: &str,
    dirs_processed: usize,
    dirs_total: usize,
    files_found: usize,
    current_path: Option<&Path>,
) where
    F: Fn(LargeFileProgress),
{
    on_progress(LargeFileProgress {
        drive_letter: drive_letter.to_string(),
        dirs_processed,
        dirs_total,
        files_found,
        current_path: current_path.map(|path| path.to_string_lossy().to_string()),
    });
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
    let providers = crate::platform::providers();
    let output = providers.dir_scanner.execute(&ScanContext {
        root: path.to_path_buf(),
        cancel,
    })?;

    Ok(DirInfo {
        name,
        path: path.to_string_lossy().to_string(),
        size_bytes: output.size_bytes,
        file_count: output.file_count,
        dir_count: output.dir_count,
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

    let entries: Vec<_> = jwalk::WalkDir::new(path)
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
    fn scan_stage_trait_executes_measure_stage() {
        let root =
            std::env::temp_dir().join(format!("diskpulse-scan-stage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create scan stage dir");
        write_sized_file(&root.join("stage.bin"), 12);

        let ctx = ScanContext {
            root: root.clone(),
            cancel: None,
        };
        let output = MeasureStage.execute(&ctx).expect("measure stage");

        assert_eq!(output.size_bytes, 12);
        assert_eq!(output.file_count, 1);

        let _ = std::fs::remove_dir_all(&root);
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

    fn write_sized_file(path: &Path, size: usize) {
        std::fs::write(path, vec![b'x'; size]).expect("write sized test file");
    }

    #[test]
    fn find_large_files_returns_top_n_descending() {
        let root =
            std::env::temp_dir().join(format!("diskpulse-large-files-top-{}", std::process::id()));
        let nested = root.join("nested");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&nested).expect("create test dir");
        write_sized_file(&root.join("small.bin"), 10);
        write_sized_file(&nested.join("medium.bin"), 20);
        write_sized_file(&nested.join("large.bin"), 30);

        let files =
            find_large_files_under_root(&root, "T", 1, 2, |_| {}, None).expect("find large files");

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "large.bin");
        assert_eq!(files[0].size_bytes, 30);
        assert_eq!(files[1].name, "medium.bin");
        assert_eq!(files[1].size_bytes, 20);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn find_large_files_filters_by_min_size() {
        let root =
            std::env::temp_dir().join(format!("diskpulse-large-files-min-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create test dir");
        write_sized_file(&root.join("below.bin"), 9);
        write_sized_file(&root.join("at.bin"), 10);

        let files = find_large_files_under_root(&root, "T", 10, 10, |_| {}, None)
            .expect("find large files");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "at.bin");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn find_large_files_can_cancel_from_progress_callback() {
        let root = std::env::temp_dir().join(format!(
            "diskpulse-large-files-cancel-{}",
            std::process::id()
        ));
        let nested = root.join("nested");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&nested).expect("create test dir");
        write_sized_file(&root.join("a.bin"), 10);
        write_sized_file(&nested.join("b.bin"), 20);

        let cancel = AtomicBool::new(false);
        let result = find_large_files_under_root(
            &root,
            "T",
            1,
            10,
            |progress| {
                if progress.files_found > 0 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            Some(&cancel),
        );

        assert_eq!(result, Err("Large file scan cancelled".to_string()));

        let _ = std::fs::remove_dir_all(&root);
    }
}
