use crate::scanner::{self, FileEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DuplicateGroup {
    pub group_id: String,
    pub total_size_wasted: u64,
    #[serde(default)]
    pub hard_link_count: usize,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateScanProgress {
    pub drive_letter: String,
    pub phase: String,
    pub files_processed: usize,
    pub groups_found: usize,
    pub current_path: Option<String>,
    #[serde(default)]
    pub hard_link_count: usize,
}

pub fn scan_duplicates_with_progress_and_cancel<F>(
    drive_letter: &str,
    min_size: u64,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<DuplicateGroup>, String>
where
    F: Fn(DuplicateScanProgress),
{
    let drive_path = format!("{}:\\", drive_letter.to_uppercase());
    let root = Path::new(&drive_path);
    if !root.exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }
    scan_duplicates_under_root(root, drive_letter, min_size, on_progress, cancel)
}

pub fn scan_duplicates_in_directory<F>(
    root: &Path,
    min_size: u64,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<DuplicateGroup>, String>
where
    F: Fn(DuplicateScanProgress),
{
    scan_duplicates_under_root(root, "BENCH", min_size, on_progress, cancel)
}

fn scan_duplicates_under_root<F>(
    root: &Path,
    drive_letter: &str,
    min_size: u64,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<DuplicateGroup>, String>
where
    F: Fn(DuplicateScanProgress),
{
    if !root.exists() {
        return Err(format!("Directory does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()));
    }

    let drive_letter = drive_letter.to_uppercase();
    let mut files_by_size: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    let mut seen_identities: HashMap<crate::platform::FileIdentity, String> = HashMap::new();
    let mut processed = 0usize;
    let mut hard_link_count = 0usize;

    emit_progress(
        &on_progress,
        &drive_letter,
        "size_grouping",
        processed,
        0,
        hard_link_count,
        Some(root),
    );
    for entry in jwalk::WalkDir::new(root).follow_links(false) {
        if scanner::is_cancelled(cancel) {
            return Err("Duplicate scan cancelled".to_string());
        }

        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().is_file() {
            continue;
        }

        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.len() < min_size {
            continue;
        }

        processed += 1;
        let path = entry.path();
        let path_text = path.to_string_lossy().to_string();
        if let Ok(Some(identity)) = crate::platform::providers()
            .file_meta
            .file_identity(&path_text)
        {
            if seen_identities.insert(identity, path_text).is_some() {
                hard_link_count += 1;
                emit_progress(
                    &on_progress,
                    &drive_letter,
                    "size_grouping",
                    processed,
                    0,
                    hard_link_count,
                    Some(&path),
                );
                continue;
            }
        }

        files_by_size
            .entry(metadata.len())
            .or_default()
            .push(file_entry_from_path(&path, metadata.len(), &metadata));

        emit_progress(
            &on_progress,
            &drive_letter,
            "size_grouping",
            processed,
            0,
            hard_link_count,
            Some(&path),
        );
        if scanner::is_cancelled(cancel) {
            return Err("Duplicate scan cancelled".to_string());
        }
    }

    let mut groups_by_prefix: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();
    for (size, entries) in files_by_size {
        if entries.len() < 2 {
            continue;
        }
        for entry in entries {
            if scanner::is_cancelled(cancel) {
                return Err("Duplicate scan cancelled".to_string());
            }
            let prefix = hash_prefix_4kb(Path::new(&entry.path))?;
            groups_by_prefix
                .entry((size, prefix))
                .or_default()
                .push(entry);
        }
    }

    let mut groups_by_full_hash: HashMap<(u64, String), Vec<FileEntry>> = HashMap::new();
    for ((size, _prefix), entries) in groups_by_prefix {
        if entries.len() < 2 {
            continue;
        }
        for entry in entries {
            if scanner::is_cancelled(cancel) {
                return Err("Duplicate scan cancelled".to_string());
            }
            let full_hash = hash_file(Path::new(&entry.path))?;
            groups_by_full_hash
                .entry((size, full_hash))
                .or_default()
                .push(entry);
        }
    }

    let mut groups = Vec::new();
    for ((size, hash), mut files) in groups_by_full_hash {
        if files.len() < 2 {
            continue;
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        let group_id = hash.chars().take(16).collect::<String>();
        groups.push(DuplicateGroup {
            group_id,
            total_size_wasted: size.saturating_mul(files.len().saturating_sub(1) as u64),
            hard_link_count,
            files,
        });
        emit_progress(
            &on_progress,
            &drive_letter,
            "complete",
            processed,
            groups.len(),
            hard_link_count,
            None,
        );
    }
    groups.sort_by(|a, b| b.total_size_wasted.cmp(&a.total_size_wasted));
    Ok(groups)
}

fn file_entry_from_path(path: &Path, size_bytes: u64, metadata: &std::fs::Metadata) -> FileEntry {
    let path_text = path.to_string_lossy().to_string();
    let meta = crate::platform::providers().file_meta;
    FileEntry {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: path_text.clone(),
        size_bytes,
        modified_epoch_ms: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0),
        hard_link_count: meta.hard_link_count(&path_text).unwrap_or(1),
        size_on_disk_bytes: meta.size_on_disk(&path_text).ok().flatten(),
        file_category: Some(
            crate::fileclass::category_id(&crate::fileclass::classify_path(path)).into(),
        ),
    }
}

fn hash_prefix_4kb(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).map_err(|e| format!("Open file error: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 4096];
    let read = std::io::Read::read(&mut file, &mut buffer)
        .map_err(|e| format!("Read file error: {}", e))?;
    hasher.update(&buffer[..read]);
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).map_err(|e| format!("Open file error: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let read = std::io::Read::read(&mut file, &mut buffer)
            .map_err(|e| format!("Read file error: {}", e))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn emit_progress<F>(
    on_progress: &F,
    drive_letter: &str,
    phase: &str,
    files_processed: usize,
    groups_found: usize,
    hard_link_count: usize,
    current_path: Option<&Path>,
) where
    F: Fn(DuplicateScanProgress),
{
    on_progress(DuplicateScanProgress {
        drive_letter: drive_letter.to_string(),
        phase: phase.to_string(),
        files_processed,
        groups_found,
        current_path: current_path.map(|path| path.to_string_lossy().to_string()),
        hard_link_count,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn write_file(path: &std::path::Path, data: &[u8]) {
        std::fs::write(path, data).expect("write duplicate test file");
    }

    #[test]
    fn duplicate_scan_groups_identical_files_and_counts_waste() {
        let root = std::env::temp_dir().join(format!(
            "diskpulse-duplicates-groups-{}",
            std::process::id()
        ));
        let nested = root.join("nested");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&nested).expect("create duplicate test dir");
        write_file(&root.join("a.bin"), b"same payload");
        write_file(&nested.join("b.bin"), b"same payload");
        write_file(&nested.join("unique.bin"), b"different payload");

        let groups =
            scan_duplicates_under_root(&root, "T", 1, |_| {}, None).expect("scan duplicates");

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[0].total_size_wasted, b"same payload".len() as u64);
        assert_eq!(groups[0].hard_link_count, 0);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_scan_respects_min_size() {
        let root = std::env::temp_dir().join(format!(
            "diskpulse-duplicates-min-size-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create duplicate test dir");
        write_file(&root.join("a.bin"), b"tiny");
        write_file(&root.join("b.bin"), b"tiny");

        let groups =
            scan_duplicates_under_root(&root, "T", 10, |_| {}, None).expect("scan duplicates");

        assert!(groups.is_empty());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_scan_can_cancel_from_progress_callback() {
        let root = std::env::temp_dir().join(format!(
            "diskpulse-duplicates-cancel-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create duplicate test dir");
        write_file(&root.join("a.bin"), b"same payload");
        write_file(&root.join("b.bin"), b"same payload");

        let cancel = AtomicBool::new(false);
        let result = scan_duplicates_under_root(
            &root,
            "T",
            1,
            |progress| {
                if progress.files_processed > 0 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
            Some(&cancel),
        );

        assert_eq!(result, Err("Duplicate scan cancelled".to_string()));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn duplicate_scan_skips_hard_links_with_same_file_identity() {
        let root = std::env::temp_dir().join(format!(
            "diskpulse-duplicates-hardlink-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create duplicate test dir");
        let original = root.join("a.bin");
        let linked = root.join("b.bin");
        write_file(&original, b"same inode payload");
        if std::fs::hard_link(&original, &linked).is_err() {
            let _ = std::fs::remove_dir_all(&root);
            return;
        }

        let last_hard_links = std::cell::Cell::new(0usize);
        let groups = scan_duplicates_under_root(
            &root,
            "T",
            1,
            |progress| {
                last_hard_links.set(progress.hard_link_count);
            },
            None,
        )
        .expect("scan duplicates");

        assert!(groups.is_empty());
        assert_eq!(last_hard_links.get(), 1);

        let _ = std::fs::remove_dir_all(&root);
    }
}
