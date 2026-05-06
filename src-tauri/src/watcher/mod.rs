use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// File system event kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FsEventKind {
    Added,
    Removed,
    Modified,
}

/// A single file system event from the watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsEvent {
    pub kind: FsEventKind,
    pub path: String,
    pub is_directory: bool,
    pub size_bytes: u64,
}

/// Aggregated changes for a batch window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsChangeBatch {
    pub watched_dir: String,
    pub events: Vec<FsEvent>,
    pub event_count: usize,
    pub timestamp_ms: u64,
}

/// Configuration for the file system watcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    pub directories: Vec<String>,
    pub poll_interval_ms: u64,
    pub debounce_ms: u64,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            directories: default_watch_dirs(),
            poll_interval_ms: 2000,
            debounce_ms: 1500,
        }
    }
}

pub fn default_watch_dirs() -> Vec<String> {
    let mut dirs = Vec::new();
    if let Ok(temp) = std::env::var("TEMP") {
        dirs.push(temp);
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        dirs.push(format!("{}\\Downloads", userprofile));
    }
    if let Ok(localappdata) = std::env::var("LOCALAPPDATA") {
        dirs.push(localappdata);
    }
    dirs
}

/// Start watching the configured directories by polling.
/// Returns a `WatcherGuard` that stops watching when dropped.
pub fn start_watching<F>(config: WatcherConfig, on_batch: F) -> WatcherGuard
where
    F: Fn(FsChangeBatch) + Send + Sync + 'static,
{
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let poll_interval = Duration::from_millis(config.poll_interval_ms);
    let debounce_ms = config.debounce_ms;
    let on_batch = Arc::new(on_batch);

    let active_dirs: Vec<String> = config
        .directories
        .into_iter()
        .filter(|d| std::path::Path::new(d).exists())
        .collect();

    let mut handles = Vec::new();

    for dir in active_dirs {
        let dir_clone = dir.clone();
        let cancel = cancel_flag.clone();
        let on_batch = on_batch.clone();

        // Snapshot: path -> (size, is_dir, modified_time)
        type Snapshot = HashMap<String, (u64, bool, u64)>;

        let handle = thread::Builder::new()
            .name(format!("diskpulse-watch-{}", sanitize_name(&dir_clone)))
            .spawn(move || {
                let mut last_snapshot: Snapshot = snapshot_dir(&dir_clone);
                let mut last_flush = Instant::now();
                let mut pending_events: Vec<FsEvent> = Vec::new();

                loop {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(poll_interval);

                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }

                    let current = snapshot_dir(&dir_clone);
                    let mut batch_events: Vec<FsEvent> = Vec::new();

                    // Detect added and modified
                    for (path, &(size, is_dir, mtime)) in &current {
                        match last_snapshot.get(path) {
                            None => {
                                batch_events.push(FsEvent {
                                    kind: FsEventKind::Added,
                                    path: path.clone(),
                                    is_directory: is_dir,
                                    size_bytes: size,
                                });
                            }
                            Some(&(old_size, _, old_mtime)) => {
                                if old_size != size || old_mtime != mtime {
                                    batch_events.push(FsEvent {
                                        kind: FsEventKind::Modified,
                                        path: path.clone(),
                                        is_directory: is_dir,
                                        size_bytes: size,
                                    });
                                }
                            }
                        }
                    }

                    // Detect removed
                    for path in last_snapshot.keys() {
                        if !current.contains_key(path) {
                            batch_events.push(FsEvent {
                                kind: FsEventKind::Removed,
                                path: path.clone(),
                                is_directory: false,
                                size_bytes: 0,
                            });
                        }
                    }

                    last_snapshot = current;

                    if !batch_events.is_empty() {
                        pending_events.extend(batch_events);
                    }

                    // Flush after debounce window
                    let elapsed = last_flush.elapsed().as_millis() as u64;
                    if !pending_events.is_empty() && elapsed >= debounce_ms {
                        let batch = FsChangeBatch {
                            watched_dir: dir_clone.clone(),
                            events: std::mem::take(&mut pending_events),
                            event_count: 0,
                            timestamp_ms: now_ms(),
                        };
                        let count = batch.events.len();
                        let mut batch_with_count = batch;
                        batch_with_count.event_count = count;
                        on_batch(batch_with_count);
                        last_flush = Instant::now();
                    }
                }

                // Final flush
                if !pending_events.is_empty() {
                    let batch = FsChangeBatch {
                        watched_dir: dir_clone,
                        events: pending_events,
                        event_count: 0,
                        timestamp_ms: now_ms(),
                    };
                    on_batch(batch);
                }
            })
            .ok();

        if let Some(h) = handle {
            handles.push(h);
        }
    }

    WatcherGuard {
        cancel_flag,
        handles: Arc::new(Mutex::new(handles)),
    }
}

fn snapshot_dir(dir: &str) -> HashMap<String, (u64, bool, u64)> {
    let mut map = HashMap::new();
    let path = std::path::Path::new(dir);
    if !path.exists() {
        return map;
    }
    // Only scan one level deep to keep polling fast
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let key = entry_path.to_string_lossy().to_string();
            if let Ok(meta) = entry.metadata() {
                let size = meta.len();
                let is_dir = meta.is_dir();
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                map.insert(key, (size, is_dir, mtime));
            }
        }
    }
    map
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn sanitize_name(path: &str) -> String {
    path.replace(['\\', ':', '/'], "_")
        .chars()
        .take(30)
        .collect()
}

/// Guard that stops watching when dropped.
pub struct WatcherGuard {
    cancel_flag: Arc<AtomicBool>,
    handles: Arc<Mutex<Vec<thread::JoinHandle<()>>>>,
}

impl WatcherGuard {
    pub fn stop(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }
}

impl Drop for WatcherGuard {
    fn drop(&mut self) {
        self.stop();
        if let Ok(mut handles) = self.handles.lock() {
            for h in handles.drain(..) {
                let _ = h.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_config_has_dirs() {
        let config = WatcherConfig::default();
        assert!(!config.directories.is_empty());
    }

    #[test]
    fn watcher_guard_stop() {
        let guard = start_watching(
            WatcherConfig {
                directories: vec![],
                poll_interval_ms: 100,
                debounce_ms: 50,
            },
            |_| {},
        );
        guard.stop();
        assert!(guard.cancel_flag.load(Ordering::Relaxed));
    }

    #[test]
    fn fs_event_serializes() {
        let event = FsEvent {
            kind: FsEventKind::Added,
            path: "C:\\Temp\\test.txt".into(),
            is_directory: false,
            size_bytes: 128,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Added"));
    }

    #[test]
    fn batch_serializes() {
        let batch = FsChangeBatch {
            watched_dir: "C:\\Temp".into(),
            events: vec![FsEvent {
                kind: FsEventKind::Added,
                path: "C:\\Temp\\new.txt".into(),
                is_directory: false,
                size_bytes: 256,
            }],
            event_count: 1,
            timestamp_ms: 1700000000000,
        };
        let json = serde_json::to_string(&batch).unwrap();
        assert!(json.contains("C:\\\\Temp"));
        assert!(json.contains("new.txt"));
    }
}
