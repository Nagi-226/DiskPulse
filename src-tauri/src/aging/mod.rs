use crate::scanner::{self, FileEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, SystemTime};

const DAY_SECONDS: u64 = 24 * 60 * 60;
const ZOMBIE_THRESHOLD_DAYS: u64 = 180;
const HOTSPOT_THRESHOLD_DAYS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgeBucket {
    pub id: String,
    pub label: String,
    pub min_days: u64,
    pub max_days: Option<u64>,
    pub total_bytes: u64,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hotspot {
    pub path: String,
    pub recent_bytes: u64,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileAge {
    pub path: String,
    pub age_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgingReport {
    pub drive_letter: String,
    pub buckets: Vec<AgeBucket>,
    pub zombies_total_size: u64,
    pub zombie_files: Vec<FileEntry>,
    pub hotspots: Vec<Hotspot>,
    #[serde(default, skip_serializing)]
    pub file_ages: Vec<FileAge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgingScanProgress {
    pub drive_letter: String,
    pub files_processed: usize,
    pub buckets: Vec<AgeBucket>,
    pub current_path: Option<String>,
}

#[derive(Debug, Clone)]
struct FileTimeRecord {
    path: String,
    size_bytes: u64,
    created: SystemTime,
    modified: SystemTime,
    accessed: SystemTime,
}

impl FileTimeRecord {
    fn is_zombie(&self, now: SystemTime, threshold_days: u64) -> bool {
        age_days(now, self.accessed) >= threshold_days
    }
}

pub fn analyze_file_aging(drive_letter: &str) -> Result<AgingReport, String> {
    let zombie_threshold_days = crate::db::get_settings()
        .map(|settings| settings.aging_zombie_days)
        .unwrap_or(ZOMBIE_THRESHOLD_DAYS);
    analyze_file_aging_with_threshold_and_cancel(drive_letter, zombie_threshold_days, |_| {}, None)
}

pub fn analyze_file_aging_with_progress_and_cancel<F>(
    drive_letter: &str,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<AgingReport, String>
where
    F: Fn(AgingScanProgress),
{
    let zombie_threshold_days = crate::db::get_settings()
        .map(|settings| settings.aging_zombie_days)
        .unwrap_or(ZOMBIE_THRESHOLD_DAYS);
    analyze_file_aging_with_threshold_and_cancel(
        drive_letter,
        zombie_threshold_days,
        on_progress,
        cancel,
    )
}

pub fn analyze_file_aging_with_threshold_and_cancel<F>(
    drive_letter: &str,
    zombie_threshold_days: u64,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<AgingReport, String>
where
    F: Fn(AgingScanProgress),
{
    let drive_path = format!("{}:\\", drive_letter.to_uppercase());
    let root = Path::new(&drive_path);
    if !root.exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }
    analyze_file_aging_under_root(
        root,
        drive_letter,
        zombie_threshold_days,
        on_progress,
        cancel,
    )
}

fn analyze_file_aging_under_root<F>(
    root: &Path,
    drive_letter: &str,
    zombie_threshold_days: u64,
    on_progress: F,
    cancel: Option<&AtomicBool>,
) -> Result<AgingReport, String>
where
    F: Fn(AgingScanProgress),
{
    if !root.exists() {
        return Err(format!("Directory does not exist: {}", root.display()));
    }
    if !root.is_dir() {
        return Err(format!("Not a directory: {}", root.display()));
    }

    let now = SystemTime::now();
    let drive_letter = drive_letter.to_uppercase();
    let mut buckets = empty_buckets();
    let mut zombie_files = Vec::new();
    let mut file_ages = Vec::new();
    let mut hotspots: HashMap<String, Hotspot> = HashMap::new();
    let mut files_processed = 0usize;

    for entry in jwalk::WalkDir::new(root).follow_links(false) {
        if scanner::is_cancelled(cancel) {
            return Err("Aging scan cancelled".to_string());
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

        let record = FileTimeRecord {
            path: entry.path().to_string_lossy().to_string(),
            size_bytes: metadata.len(),
            created: metadata.created().unwrap_or(now),
            modified: metadata.modified().unwrap_or(now),
            accessed: metadata
                .accessed()
                .unwrap_or_else(|_| metadata.modified().unwrap_or(now)),
        };

        let _created_age_days = age_days(now, record.created);
        let modified_age_days = age_days(now, record.modified);
        add_to_bucket(&mut buckets, modified_age_days, record.size_bytes);
        file_ages.push(FileAge {
            path: record.path.clone(),
            age_days: modified_age_days,
        });
        if record.is_zombie(now, zombie_threshold_days) {
            zombie_files.push(file_entry_from_record(&record));
        }
        if modified_age_days <= HOTSPOT_THRESHOLD_DAYS {
            add_hotspot(&mut hotspots, &entry.path(), record.size_bytes);
        }

        files_processed += 1;
        emit_progress(
            &on_progress,
            &drive_letter,
            files_processed,
            &buckets,
            Some(&entry.path()),
        );
        if scanner::is_cancelled(cancel) {
            return Err("Aging scan cancelled".to_string());
        }
    }

    zombie_files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    let zombies_total_size = zombie_files.iter().map(|file| file.size_bytes).sum();
    file_ages.sort_by(|a, b| a.path.cmp(&b.path));
    let mut hotspots: Vec<Hotspot> = hotspots.into_values().collect();
    hotspots.sort_by(|a, b| b.recent_bytes.cmp(&a.recent_bytes));

    Ok(AgingReport {
        drive_letter,
        buckets,
        zombies_total_size,
        zombie_files,
        hotspots,
        file_ages,
    })
}

fn empty_buckets() -> Vec<AgeBucket> {
    vec![
        AgeBucket {
            id: "lt_7d".into(),
            label: "<7d".into(),
            min_days: 0,
            max_days: Some(6),
            total_bytes: 0,
            file_count: 0,
        },
        AgeBucket {
            id: "d7_30".into(),
            label: "7-30d".into(),
            min_days: 7,
            max_days: Some(30),
            total_bytes: 0,
            file_count: 0,
        },
        AgeBucket {
            id: "m1_3".into(),
            label: "1-3mo".into(),
            min_days: 31,
            max_days: Some(90),
            total_bytes: 0,
            file_count: 0,
        },
        AgeBucket {
            id: "m3_6".into(),
            label: "3-6mo".into(),
            min_days: 91,
            max_days: Some(180),
            total_bytes: 0,
            file_count: 0,
        },
        AgeBucket {
            id: "m6_12".into(),
            label: "6-12mo".into(),
            min_days: 181,
            max_days: Some(365),
            total_bytes: 0,
            file_count: 0,
        },
        AgeBucket {
            id: "y1_3".into(),
            label: "1-3yr".into(),
            min_days: 366,
            max_days: Some(1095),
            total_bytes: 0,
            file_count: 0,
        },
        AgeBucket {
            id: "gt_3y".into(),
            label: ">3yr".into(),
            min_days: 1096,
            max_days: None,
            total_bytes: 0,
            file_count: 0,
        },
    ]
}

fn bucket_for_age_days(days: u64) -> &'static str {
    match days {
        0..=6 => "lt_7d",
        7..=30 => "d7_30",
        31..=90 => "m1_3",
        91..=180 => "m3_6",
        181..=365 => "m6_12",
        366..=1095 => "y1_3",
        _ => "gt_3y",
    }
}

fn add_to_bucket(buckets: &mut [AgeBucket], days: u64, size_bytes: u64) {
    let id = bucket_for_age_days(days);
    if let Some(bucket) = buckets.iter_mut().find(|bucket| bucket.id == id) {
        bucket.total_bytes = bucket.total_bytes.saturating_add(size_bytes);
        bucket.file_count += 1;
    }
}

fn add_hotspot(hotspots: &mut HashMap<String, Hotspot>, path: &Path, size_bytes: u64) {
    let dir = parent_dir(path);
    let hotspot = hotspots.entry(dir.clone()).or_insert(Hotspot {
        path: dir,
        recent_bytes: 0,
        file_count: 0,
    });
    hotspot.recent_bytes = hotspot.recent_bytes.saturating_add(size_bytes);
    hotspot.file_count += 1;
}

fn parent_dir(path: &Path) -> String {
    path.parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn file_entry_from_record(record: &FileTimeRecord) -> FileEntry {
    let path = Path::new(&record.path);
    let meta = crate::platform::providers().file_meta;
    FileEntry {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: record.path.clone(),
        size_bytes: record.size_bytes,
        modified_epoch_ms: record
            .modified
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0),
        hard_link_count: meta.hard_link_count(&record.path).unwrap_or(1),
        size_on_disk_bytes: meta.size_on_disk(&record.path).ok().flatten(),
        file_category: Some(
            crate::fileclass::category_id(&crate::fileclass::classify_path(path)).into(),
        ),
    }
}

fn age_days(now: SystemTime, then: SystemTime) -> u64 {
    now.duration_since(then).unwrap_or(Duration::ZERO).as_secs() / DAY_SECONDS
}

fn emit_progress<F>(
    on_progress: &F,
    drive_letter: &str,
    files_processed: usize,
    buckets: &[AgeBucket],
    current_path: Option<&Path>,
) where
    F: Fn(AgingScanProgress),
{
    on_progress(AgingScanProgress {
        drive_letter: drive_letter.to_string(),
        files_processed,
        buckets: buckets.to_vec(),
        current_path: current_path.map(|path| path.to_string_lossy().to_string()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn aging_bucket_classifies_expected_ranges() {
        assert_eq!(bucket_for_age_days(3), "lt_7d");
        assert_eq!(bucket_for_age_days(20), "d7_30");
        assert_eq!(bucket_for_age_days(70), "m1_3");
        assert_eq!(bucket_for_age_days(150), "m3_6");
        assert_eq!(bucket_for_age_days(250), "m6_12");
        assert_eq!(bucket_for_age_days(800), "y1_3");
        assert_eq!(bucket_for_age_days(1200), "gt_3y");
    }

    #[test]
    fn file_record_marks_zombies_after_threshold() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(400 * 24 * 60 * 60);
        let old_access = now - Duration::from_secs(220 * 24 * 60 * 60);
        let record = FileTimeRecord {
            path: "C:\\Archive\\old.iso".into(),
            size_bytes: 10,
            created: now,
            modified: now,
            accessed: old_access,
        };

        assert!(record.is_zombie(now, 180));
    }
}
