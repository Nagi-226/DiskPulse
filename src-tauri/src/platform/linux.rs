use crate::platform::{
    CleanupProvider, DirScanner, DiskInfoProvider, FileIdentity, FileMetaAnalyzer, FsWatcher,
    RestoreResult, SystemInfo, TrashResult, WatcherGuard,
};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct LinuxDiskInfoProvider;
pub struct LinuxFsWatcher;
pub struct LinuxWalkStage;
pub struct LinuxCleanupProvider;
pub struct LinuxFileMetaAnalyzer;
pub struct LinuxSystemInfo;

impl DiskInfoProvider for LinuxDiskInfoProvider {
    fn total_bytes(&self, drive: &str) -> Result<u64, String> {
        df_bytes(drive, 1)
    }
    fn free_bytes(&self, drive: &str) -> Result<u64, String> {
        df_bytes(drive, 3)
    }
    fn list_drives(&self) -> Result<Vec<String>, String> {
        let mounts = std::fs::read_to_string("/proc/mounts")
            .map_err(|e| format!("read /proc/mounts: {e}"))?;
        Ok(mounts
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1).map(str::to_string))
            .collect())
    }
    fn filesystem_type(&self, drive: &str) -> Result<String, String> {
        let mounts = std::fs::read_to_string("/proc/mounts")
            .map_err(|e| format!("read /proc/mounts: {e}"))?;
        for line in mounts.lines() {
            let mut parts = line.split_whitespace();
            let _device = parts.next();
            let mount = parts.next();
            let fs = parts.next();
            if mount == Some(drive) {
                return Ok(fs.unwrap_or("unknown").to_string());
            }
        }
        Ok("unknown".into())
    }
}

impl FsWatcher for LinuxFsWatcher {
    fn start(
        &self,
        config: crate::watcher::WatcherConfig,
        on_batch: Box<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
    ) -> Result<WatcherGuard, String> {
        let on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static> =
            Arc::from(on_batch);
        start_inotify_watcher(config.clone(), on_batch.clone()).or_else(|_| {
            Ok(WatcherGuard::from_polling(crate::watcher::start_watching(
                config,
                move |batch| on_batch(batch),
            )))
        })
    }
    fn stop(&self, guard: &mut WatcherGuard) -> Result<(), String> {
        guard.stop();
        Ok(())
    }
}

impl DirScanner for LinuxWalkStage {
    fn name(&self) -> &'static str {
        "linux-walk"
    }
    fn execute(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> Result<crate::scanner::ScanOutput, String> {
        let stage = crate::scanner::MeasureStage;
        crate::scanner::ScanStage::execute(&stage, ctx)
    }
}

impl CleanupProvider for LinuxCleanupProvider {
    fn move_to_trash(&self, path: &str) -> TrashResult {
        let result = std::process::Command::new("gio")
            .args(["trash", path])
            .status();
        let success = result.map(|status| status.success()).unwrap_or(false);
        TrashResult {
            success,
            original_path: path.into(),
            reason: if success {
                None
            } else {
                Some("gio trash failed or unavailable".into())
            },
        }
    }
    fn restore_from_trash(&self, original_path: &str) -> RestoreResult {
        RestoreResult {
            success: false,
            original_path: original_path.into(),
            restored_path: None,
            reason: Some("Linux trash restore is not implemented".into()),
        }
    }
    fn is_available(&self) -> bool {
        true
    }
}

impl FileMetaAnalyzer for LinuxFileMetaAnalyzer {
    fn hard_link_count(&self, path: &str) -> Result<u64, String> {
        unix_nlink(path)
    }
    fn is_sparse(&self, path: &str) -> Result<bool, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("metadata: {e}"))?;
        Ok(self
            .size_on_disk(path)?
            .is_some_and(|disk| disk < meta.len()))
    }
    fn size_on_disk(&self, path: &str) -> Result<Option<u64>, String> {
        Ok(Some(
            std::fs::metadata(path)
                .map_err(|e| format!("metadata: {e}"))?
                .blocks()
                * 512,
        ))
    }
    fn file_identity(&self, path: &str) -> Result<Option<FileIdentity>, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("metadata: {e}"))?;
        Ok(Some(FileIdentity {
            volume_serial: meta.dev(),
            file_index: meta.ino(),
        }))
    }
}

impl SystemInfo for LinuxSystemInfo {
    fn os_name(&self) -> Result<String, String> {
        Ok("Linux".into())
    }
    fn os_version(&self) -> Result<String, String> {
        Ok(std::fs::read_to_string("/proc/version").unwrap_or_default())
    }
    fn cpu_count(&self) -> usize {
        std::thread::available_parallelism()
            .map(|c| c.get())
            .unwrap_or(1)
    }
    fn total_ram_bytes(&self) -> Result<u64, String> {
        let meminfo = std::fs::read_to_string("/proc/meminfo")
            .map_err(|e| format!("read /proc/meminfo: {e}"))?;
        for line in meminfo.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
                return Ok(kb * 1024);
            }
        }
        Ok(0)
    }
    fn app_data_dir(&self) -> Result<String, String> {
        std::env::var("XDG_DATA_HOME")
            .or_else(|_| std::env::var("HOME").map(|home| format!("{home}/.local/share")))
            .map_err(|e| format!("app data dir: {e}"))
    }
}

fn df_bytes(path: &str, column: usize) -> Result<u64, String> {
    let output = std::process::Command::new("df")
        .args(["-B1", path])
        .output()
        .map_err(|e| format!("df failed: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(column))
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "cannot parse df output".into())
}

#[cfg(unix)]
fn unix_nlink(path: &str) -> Result<u64, String> {
    Ok(std::fs::metadata(path)
        .map_err(|e| format!("metadata: {e}"))?
        .nlink())
}

fn start_inotify_watcher(
    config: crate::watcher::WatcherConfig,
    on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
) -> Result<WatcherGuard, String> {
    let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if fd < 0 {
        return Err(format!(
            "inotify_init1 failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut wd_to_dir = HashMap::new();
    for dir in config
        .directories
        .iter()
        .filter(|dir| std::path::Path::new(dir).exists())
    {
        let c_dir = std::ffi::CString::new(dir.as_str())
            .map_err(|_| format!("path contains NUL byte: {dir}"))?;
        let wd = unsafe {
            inotify_add_watch(
                fd,
                c_dir.as_ptr(),
                IN_CREATE
                    | IN_DELETE
                    | IN_MODIFY
                    | IN_MOVED_FROM
                    | IN_MOVED_TO
                    | IN_ATTRIB
                    | IN_CLOSE_WRITE,
            )
        };
        if wd >= 0 {
            wd_to_dir.insert(wd, dir.clone());
        }
    }

    if wd_to_dir.is_empty() {
        unsafe {
            close(fd);
        }
        return Err("no inotify watch could be registered".into());
    }

    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_thread = cancel.clone();
    let debounce_ms = config.debounce_ms.min(500);
    let handle = thread::Builder::new()
        .name("diskpulse-inotify".into())
        .spawn(move || {
            run_inotify_loop(fd, wd_to_dir, debounce_ms, cancel_thread, on_batch);
        })
        .map_err(|e| {
            unsafe {
                close(fd);
            }
            format!("spawn inotify watcher failed: {e}")
        })?;

    Ok(WatcherGuard::new(
        move || {
            cancel.store(true, Ordering::Relaxed);
        },
        vec![handle],
    ))
}

fn run_inotify_loop(
    fd: i32,
    wd_to_dir: HashMap<i32, String>,
    debounce_ms: u64,
    cancel: Arc<AtomicBool>,
    on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
) {
    let mut poll_fd = PollFd {
        fd,
        events: POLLIN,
        revents: 0,
    };
    let mut buffer = vec![0u8; 64 * 1024];
    let mut pending: HashMap<String, Vec<crate::watcher::FsEvent>> = HashMap::new();
    let mut first_pending_at: Option<Instant> = None;
    let flush_after = Duration::from_millis(debounce_ms);

    while !cancel.load(Ordering::Relaxed) {
        let ready = unsafe { poll(&mut poll_fd, 1, 100) };
        if ready <= 0 {
            maybe_flush_linux(&mut pending, &mut first_pending_at, flush_after, &on_batch);
            continue;
        }

        let bytes = unsafe { read(fd, buffer.as_mut_ptr().cast(), buffer.len()) };
        if bytes <= 0 {
            continue;
        }

        for event in parse_inotify_events(&buffer[..bytes as usize], &wd_to_dir) {
            pending.entry(event.0).or_default().push(event.1);
            if first_pending_at.is_none() {
                first_pending_at = Some(Instant::now());
            }
        }
        maybe_flush_linux(&mut pending, &mut first_pending_at, flush_after, &on_batch);
    }

    flush_linux_batches(&mut pending, &on_batch);
    unsafe {
        close(fd);
    }
}

fn parse_inotify_events(
    buffer: &[u8],
    wd_to_dir: &HashMap<i32, String>,
) -> Vec<(String, crate::watcher::FsEvent)> {
    let mut events = Vec::new();
    let mut offset = 0usize;
    while offset + std::mem::size_of::<InotifyEvent>() <= buffer.len() {
        let raw = unsafe { &*(buffer.as_ptr().add(offset).cast::<InotifyEvent>()) };
        let name_start = offset + std::mem::size_of::<InotifyEvent>();
        let name_end = name_start
            .saturating_add(raw.len as usize)
            .min(buffer.len());
        let name_bytes = &buffer[name_start..name_end];
        let name = std::ffi::CStr::from_bytes_until_nul(name_bytes)
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(dir) = wd_to_dir.get(&raw.wd) {
            let full_path = std::path::Path::new(dir)
                .join(name)
                .to_string_lossy()
                .to_string();
            let metadata = std::fs::metadata(&full_path).ok();
            let kind = if raw.mask & (IN_DELETE | IN_MOVED_FROM) != 0 {
                crate::watcher::FsEventKind::Removed
            } else if raw.mask & (IN_CREATE | IN_MOVED_TO) != 0 {
                crate::watcher::FsEventKind::Added
            } else {
                crate::watcher::FsEventKind::Modified
            };
            events.push((
                dir.clone(),
                crate::watcher::FsEvent {
                    kind,
                    path: full_path,
                    is_directory: raw.mask & IN_ISDIR != 0
                        || metadata.as_ref().is_some_and(|m| m.is_dir()),
                    size_bytes: metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    previous_size_bytes: None,
                },
            ));
        }
        offset = name_end;
    }
    events
}

fn maybe_flush_linux(
    pending: &mut HashMap<String, Vec<crate::watcher::FsEvent>>,
    first_pending_at: &mut Option<Instant>,
    flush_after: Duration,
    on_batch: &Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
) {
    if first_pending_at.is_some_and(|started| started.elapsed() >= flush_after) {
        flush_linux_batches(pending, on_batch);
        *first_pending_at = None;
    }
}

fn flush_linux_batches(
    pending: &mut HashMap<String, Vec<crate::watcher::FsEvent>>,
    on_batch: &Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
) {
    for (watched_dir, events) in std::mem::take(pending) {
        if events.is_empty() {
            continue;
        }
        let count = events.len();
        on_batch(crate::watcher::FsChangeBatch {
            watched_dir,
            events,
            event_count: count,
            timestamp_ms: now_ms(),
        });
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[repr(C)]
struct InotifyEvent {
    wd: i32,
    mask: u32,
    cookie: u32,
    len: u32,
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

const IN_NONBLOCK: i32 = 0o4000;
const IN_CLOEXEC: i32 = 0o2000000;
const IN_MODIFY: u32 = 0x0000_0002;
const IN_ATTRIB: u32 = 0x0000_0004;
const IN_CLOSE_WRITE: u32 = 0x0000_0008;
const IN_MOVED_FROM: u32 = 0x0000_0040;
const IN_MOVED_TO: u32 = 0x0000_0080;
const IN_CREATE: u32 = 0x0000_0100;
const IN_DELETE: u32 = 0x0000_0200;
const IN_ISDIR: u32 = 0x4000_0000;
const POLLIN: i16 = 0x0001;

unsafe extern "C" {
    fn inotify_init1(flags: i32) -> i32;
    fn inotify_add_watch(fd: i32, pathname: *const std::os::raw::c_char, mask: u32) -> i32;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
    fn read(fd: i32, buf: *mut std::os::raw::c_void, count: usize) -> isize;
    fn close(fd: i32) -> i32;
}
