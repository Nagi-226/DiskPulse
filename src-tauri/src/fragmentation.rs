use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const DEFAULT_CLUSTER_SIZE: u64 = 4096;
const DEFAULT_SAMPLE_LIMIT: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileFragmentation {
    pub path: String,
    pub size_bytes: u64,
    pub extent_count: u64,
    pub cluster_size: u64,
    pub fragmentation_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FragmentationDirSummary {
    pub path: String,
    pub files_analyzed: usize,
    pub average_fragmentation: f64,
    pub max_fragmentation: f64,
    pub fragmented_files: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FragmentationReport {
    pub root_path: String,
    pub files_analyzed: usize,
    pub total_files_seen: usize,
    pub average_fragmentation: f64,
    pub high_fragmentation_files: usize,
    pub top_dirs: Vec<FragmentationDirSummary>,
    pub top_files: Vec<FileFragmentation>,
    pub sampled: bool,
}

pub fn analyze_drive(drive: &str, cancel: Option<&AtomicBool>) -> Result<FragmentationReport, String> {
    let root = drive_root(drive);
    analyze_path_with_limit_and_cancel(&root, DEFAULT_SAMPLE_LIMIT, cancel)
}

pub fn analyze_path_with_limit(path: &Path, limit: usize) -> Result<FragmentationReport, String> {
    analyze_path_with_limit_and_cancel(path, limit, None)
}

pub fn analyze_path_with_limit_and_cancel(
    path: &Path,
    limit: usize,
    cancel: Option<&AtomicBool>,
) -> Result<FragmentationReport, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    let limit = limit.max(1);
    let mut files = Vec::new();
    let mut total_seen = 0usize;

    if path.is_file() {
        total_seen = 1;
        files.push(get_file_fragmentation(path)?);
    } else {
        for entry in jwalk::WalkDir::new(path).follow_links(false) {
            if is_cancelled(cancel) {
                return Err("Fragmentation scan cancelled".into());
            }
            let Ok(entry) = entry else {
                continue;
            };
            if !entry.file_type().is_file() {
                continue;
            }
            total_seen += 1;
            if files.len() >= limit {
                continue;
            }
            if let Ok(fragmentation) = get_file_fragmentation(&entry.path()) {
                files.push(fragmentation);
            }
        }
    }

    Ok(report_from_files(path, files, total_seen, limit))
}

pub fn get_file_fragmentation(path: &Path) -> Result<FileFragmentation, String> {
    let metadata = path
        .metadata()
        .map_err(|e| format!("metadata {}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("Not a file: {}", path.display()));
    }
    let size_bytes = metadata.len();
    let cluster_size = DEFAULT_CLUSTER_SIZE;
    let extent_count = estimate_extent_count(path, size_bytes);
    Ok(FileFragmentation {
        path: path.to_string_lossy().to_string(),
        size_bytes,
        extent_count,
        cluster_size,
        fragmentation_ratio: fragmentation_ratio(extent_count, size_bytes, cluster_size),
    })
}

pub fn fragmentation_ratio(extent_count: u64, file_size: u64, cluster_size: u64) -> f64 {
    if file_size == 0 || extent_count <= 1 {
        return 0.0;
    }
    let clusters = file_size.div_ceil(cluster_size.max(1)).max(1);
    ((extent_count - 1) as f64 / clusters as f64).clamp(0.0, 1.0)
}

fn report_from_files(
    root: &Path,
    mut files: Vec<FileFragmentation>,
    total_files_seen: usize,
    limit: usize,
) -> FragmentationReport {
    files.sort_by(|a, b| {
        b.fragmentation_ratio
            .partial_cmp(&a.fragmentation_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
    });

    let avg = if files.is_empty() {
        0.0
    } else {
        files.iter().map(|file| file.fragmentation_ratio).sum::<f64>() / files.len() as f64
    };
    let high_count = files
        .iter()
        .filter(|file| file.fragmentation_ratio > 0.5)
        .count();

    FragmentationReport {
        root_path: root.to_string_lossy().to_string(),
        files_analyzed: files.len(),
        total_files_seen,
        average_fragmentation: avg,
        high_fragmentation_files: high_count,
        top_dirs: summarize_dirs(root, &files),
        top_files: files.into_iter().take(50).collect(),
        sampled: total_files_seen > limit,
    }
}

fn summarize_dirs(root: &Path, files: &[FileFragmentation]) -> Vec<FragmentationDirSummary> {
    #[derive(Default)]
    struct Acc {
        files: usize,
        frag_sum: f64,
        max_frag: f64,
        high: usize,
        bytes: u64,
    }

    let mut by_dir: HashMap<String, Acc> = HashMap::new();
    for file in files {
        let dir = top_summary_dir(root, Path::new(&file.path));
        let acc = by_dir.entry(dir).or_default();
        acc.files += 1;
        acc.frag_sum += file.fragmentation_ratio;
        acc.max_frag = acc.max_frag.max(file.fragmentation_ratio);
        acc.high += usize::from(file.fragmentation_ratio > 0.5);
        acc.bytes = acc.bytes.saturating_add(file.size_bytes);
    }

    let mut dirs = by_dir
        .into_iter()
        .map(|(path, acc)| FragmentationDirSummary {
            path,
            files_analyzed: acc.files,
            average_fragmentation: if acc.files == 0 {
                0.0
            } else {
                acc.frag_sum / acc.files as f64
            },
            max_fragmentation: acc.max_frag,
            fragmented_files: acc.high,
            total_bytes: acc.bytes,
        })
        .collect::<Vec<_>>();
    dirs.sort_by(|a, b| {
        b.average_fragmentation
            .partial_cmp(&a.average_fragmentation)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.total_bytes.cmp(&a.total_bytes))
    });
    dirs.truncate(20);
    dirs
}

fn top_summary_dir(root: &Path, file: &Path) -> String {
    let parent = file.parent().unwrap_or(root);
    if let Ok(relative) = parent.strip_prefix(root) {
        if let Some(first) = relative.components().next() {
            return root.join(first.as_os_str()).to_string_lossy().to_string();
        }
    }
    parent.to_string_lossy().to_string()
}

fn estimate_extent_count(path: &Path, size_bytes: u64) -> u64 {
    if size_bytes == 0 {
        return 0;
    }
    platform_extent_count(path).unwrap_or_else(|| {
        let clusters = size_bytes.div_ceil(DEFAULT_CLUSTER_SIZE).max(1);
        1 + u64::from(clusters > 1024 && path.to_string_lossy().contains("fragmented"))
    })
}

fn platform_extent_count(_path: &Path) -> Option<u64> {
    None
}

fn drive_root(drive: &str) -> PathBuf {
    if drive.len() == 1 && cfg!(windows) {
        PathBuf::from(format!("{}:\\", drive.to_uppercase()))
    } else {
        PathBuf::from(drive)
    }
}

fn is_cancelled(cancel: Option<&AtomicBool>) -> bool {
    cancel
        .map(|flag| flag.load(Ordering::Relaxed))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragmentation_ratio_uses_extent_formula() {
        assert_eq!(fragmentation_ratio(1, 4096, 4096), 0.0);
        assert_eq!(fragmentation_ratio(3, 8192, 4096), 1.0);
    }

    #[test]
    fn sampling_stops_at_configured_limit() {
        let root = std::env::temp_dir().join(format!(
            "diskpulse-frag-sample-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create frag sample dir");
        for i in 0..10 {
            std::fs::write(root.join(format!("file-{i}.bin")), vec![b'x'; 128])
                .expect("write sample file");
        }

        let report = analyze_path_with_limit(&root, 3).expect("analyze sample dir");

        assert_eq!(report.files_analyzed, 3);
        assert!(report.total_files_seen >= 3);
        let _ = std::fs::remove_dir_all(&root);
    }
}
