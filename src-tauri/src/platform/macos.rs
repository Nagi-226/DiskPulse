use crate::platform::{
    CleanupProvider, DirScanner, DiskInfoProvider, FileIdentity, FileMetaAnalyzer, FsWatcher,
    RestoreResult, SystemInfo, TrashResult, WatcherGuard,
};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_void};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct MacOsDiskInfoProvider;
pub struct MacOsFsWatcher;
pub struct MacOsWalkStage;
pub struct MacOsCleanupProvider;
pub struct MacOsFileMetaAnalyzer;
pub struct MacOsSystemInfo;

impl DiskInfoProvider for MacOsDiskInfoProvider {
    fn total_bytes(&self, drive: &str) -> Result<u64, String> {
        df_bytes(drive, 1)
    }
    fn free_bytes(&self, drive: &str) -> Result<u64, String> {
        df_bytes(drive, 3)
    }
    fn list_drives(&self) -> Result<Vec<String>, String> {
        Ok(vec!["/".into(), "/Volumes".into()])
    }
    fn filesystem_type(&self, _drive: &str) -> Result<String, String> {
        Ok("apfs".into())
    }
}

impl FsWatcher for MacOsFsWatcher {
    fn start(
        &self,
        config: crate::watcher::WatcherConfig,
        on_batch: Box<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
    ) -> Result<WatcherGuard, String> {
        let on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static> =
            Arc::from(on_batch);
        let active_dirs: Vec<String> = config
            .directories
            .iter()
            .filter(|dir| Path::new(dir.as_str()).exists())
            .cloned()
            .collect();

        if active_dirs.is_empty() {
            return Ok(WatcherGuard::empty());
        }

        start_fsevents(active_dirs, config.debounce_ms.min(250), on_batch.clone()).or_else(|_| {
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

impl DirScanner for MacOsWalkStage {
    fn name(&self) -> &'static str {
        "macos-walk"
    }
    fn execute(
        &self,
        ctx: &crate::scanner::ScanContext<'_>,
    ) -> Result<crate::scanner::ScanOutput, String> {
        let stage = crate::scanner::MeasureStage;
        crate::scanner::ScanStage::execute(&stage, ctx)
    }
}

impl CleanupProvider for MacOsCleanupProvider {
    fn move_to_trash(&self, path: &str) -> TrashResult {
        let result = trash::delete(path);
        TrashResult {
            success: result.is_ok(),
            original_path: path.into(),
            reason: if result.is_ok() {
                None
            } else {
                Some(format!("NSFileManager trash failed: {}", result.unwrap_err()))
            },
        }
    }
    fn restore_from_trash(&self, original_path: &str) -> RestoreResult {
        RestoreResult {
            success: false,
            original_path: original_path.into(),
            restored_path: None,
            reason: Some("macOS trash restore is not implemented".into()),
        }
    }
    fn is_available(&self) -> bool {
        true
    }
}

impl FileMetaAnalyzer for MacOsFileMetaAnalyzer {
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

impl SystemInfo for MacOsSystemInfo {
    fn os_name(&self) -> Result<String, String> {
        Ok("macOS".into())
    }
    fn os_version(&self) -> Result<String, String> {
        let output = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_err(|e| format!("sw_vers failed: {e}"))?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().into())
    }
    fn cpu_count(&self) -> usize {
        std::thread::available_parallelism()
            .map(|c| c.get())
            .unwrap_or(1)
    }
    fn total_ram_bytes(&self) -> Result<u64, String> {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .map_err(|e| format!("sysctl hw.memsize failed: {e}"))?;
        Ok(String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(0))
    }
    fn app_data_dir(&self) -> Result<String, String> {
        std::env::var("HOME")
            .map(|home| format!("{home}/Library/Application Support"))
            .map_err(|e| format!("app data dir: {e}"))
    }
}

fn df_bytes(path: &str, column: usize) -> Result<u64, String> {
    let output = std::process::Command::new("df")
        .args(["-k", path])
        .output()
        .map_err(|e| format!("df failed: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .nth(1)
        .and_then(|line| line.split_whitespace().nth(column))
        .and_then(|value| value.parse::<u64>().ok())
        .map(|kb| kb * 1024)
        .ok_or_else(|| "cannot parse df output".into())
}

type CFArrayRef = *const c_void;
type CFRunLoopRef = *const c_void;
type CFStringRef = *const c_void;
type FSEventStreamRef = *mut c_void;

#[repr(C)]
struct FSEventStreamContext {
    version: isize,
    info: *mut c_void,
    retain: Option<unsafe extern "C" fn(*const c_void) -> *const c_void>,
    release: Option<unsafe extern "C" fn(*const c_void)>,
    copy_description: Option<unsafe extern "C" fn(*const c_void) -> *const c_void>,
}

type FSEventStreamCallback = unsafe extern "C" fn(
    FSEventStreamRef,
    *mut c_void,
    usize,
    *mut c_void,
    *const u32,
    *const u64,
);

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayCreate(
        allocator: *const c_void,
        values: *const *const c_void,
        num_values: isize,
        callbacks: *const c_void,
    ) -> CFArrayRef;
    fn CFRelease(cf: *const c_void);
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopRun();
    fn CFRunLoopStop(rl: CFRunLoopRef);
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
}

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn FSEventStreamCreate(
        allocator: *const c_void,
        callback: FSEventStreamCallback,
        context: *mut FSEventStreamContext,
        paths_to_watch: CFArrayRef,
        since_when: u64,
        latency: c_double,
        flags: u32,
    ) -> FSEventStreamRef;
    fn FSEventStreamInvalidate(stream_ref: FSEventStreamRef);
    fn FSEventStreamRelease(stream_ref: FSEventStreamRef);
    fn FSEventStreamScheduleWithRunLoop(
        stream_ref: FSEventStreamRef,
        run_loop: CFRunLoopRef,
        run_loop_mode: CFStringRef,
    );
    fn FSEventStreamStart(stream_ref: FSEventStreamRef) -> u8;
    fn FSEventStreamStop(stream_ref: FSEventStreamRef);
}

const CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const FSEVENT_STREAM_EVENT_ID_SINCE_NOW: u64 = u64::MAX;
const FSEVENT_CREATE_FLAG_NO_DEFER: u32 = 0x0000_0002;
const FSEVENT_CREATE_FLAG_FILE_EVENTS: u32 = 0x0000_0010;
const FSEVENT_FLAG_ITEM_CREATED: u32 = 0x0000_0100;
const FSEVENT_FLAG_ITEM_REMOVED: u32 = 0x0000_0200;
const FSEVENT_FLAG_ITEM_RENAMED: u32 = 0x0000_0800;
const FSEVENT_FLAG_ITEM_MODIFIED: u32 = 0x0000_1000;
const FSEVENT_FLAG_ITEM_IS_DIR: u32 = 0x0002_0000;

struct MacOsStreamState {
    roots: Vec<String>,
    on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
}

fn start_fsevents(
    roots: Vec<String>,
    debounce_ms: u64,
    on_batch: Arc<dyn Fn(crate::watcher::FsChangeBatch) + Send + Sync + 'static>,
) -> Result<WatcherGuard, String> {
    let run_loop = Arc::new(Mutex::new(0usize));
    let thread_run_loop = run_loop.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let handle = thread::Builder::new()
        .name("diskpulse-fsevents".into())
        .spawn(move || unsafe {
            let state = Box::into_raw(Box::new(MacOsStreamState {
                roots: roots.clone(),
                on_batch,
            }));
            let c_paths: Vec<CString> = roots
                .iter()
                .filter_map(|path| CString::new(path.as_str()).ok())
                .collect();
            let cf_paths: Vec<CFStringRef> = c_paths
                .iter()
                .map(|path| {
                    CFStringCreateWithCString(
                        std::ptr::null(),
                        path.as_ptr(),
                        CF_STRING_ENCODING_UTF8,
                    )
                })
                .filter(|value| !value.is_null())
                .collect();
            if cf_paths.is_empty() {
                let _ = Box::from_raw(state);
                let _ = started_tx.send(Err("no valid FSEvents paths".to_string()));
                return;
            }

            let cf_values: Vec<*const c_void> = cf_paths.iter().map(|value| *value).collect();
            let paths_array = CFArrayCreate(
                std::ptr::null(),
                cf_values.as_ptr(),
                cf_values.len() as isize,
                std::ptr::null(),
            );
            for value in &cf_paths {
                CFRelease(*value);
            }
            if paths_array.is_null() {
                let _ = Box::from_raw(state);
                let _ = started_tx.send(Err("CFArrayCreate failed".to_string()));
                return;
            }

            let mut context = FSEventStreamContext {
                version: 0,
                info: state.cast(),
                retain: None,
                release: None,
                copy_description: None,
            };
            let stream = FSEventStreamCreate(
                std::ptr::null(),
                fsevents_callback,
                &mut context,
                paths_array,
                FSEVENT_STREAM_EVENT_ID_SINCE_NOW,
                (debounce_ms.max(50) as f64) / 1000.0,
                FSEVENT_CREATE_FLAG_NO_DEFER | FSEVENT_CREATE_FLAG_FILE_EVENTS,
            );
            CFRelease(paths_array);
            if stream.is_null() {
                let _ = Box::from_raw(state);
                let _ = started_tx.send(Err("FSEventStreamCreate failed".to_string()));
                return;
            }

            let mode_name = CString::new("kCFRunLoopDefaultMode").expect("static run-loop mode");
            let mode = CFStringCreateWithCString(
                std::ptr::null(),
                mode_name.as_ptr(),
                CF_STRING_ENCODING_UTF8,
            );
            let current_run_loop = CFRunLoopGetCurrent();
            if let Ok(mut guard) = thread_run_loop.lock() {
                *guard = current_run_loop as usize;
            }
            FSEventStreamScheduleWithRunLoop(stream, current_run_loop, mode);
            if FSEventStreamStart(stream) == 0 {
                FSEventStreamInvalidate(stream);
                FSEventStreamRelease(stream);
                CFRelease(mode);
                let _ = Box::from_raw(state);
                let _ = started_tx.send(Err("FSEventStreamStart failed".to_string()));
                return;
            }

            let _ = started_tx.send(Ok(()));
            CFRunLoopRun();
            FSEventStreamStop(stream);
            FSEventStreamInvalidate(stream);
            FSEventStreamRelease(stream);
            CFRelease(mode);
            let _ = Box::from_raw(state);
        })
        .map_err(|e| format!("spawn FSEvents watcher: {e}"))?;

    started_rx
        .recv()
        .map_err(|e| format!("FSEvents startup channel closed: {e}"))??;

    Ok(WatcherGuard::new(
        move || {
            if let Ok(guard) = run_loop.lock() {
                let raw = *guard;
                if raw != 0 {
                    unsafe { CFRunLoopStop(raw as CFRunLoopRef) };
                }
            }
        },
        vec![handle],
    ))
}

unsafe extern "C" fn fsevents_callback(
    _stream: FSEventStreamRef,
    client_info: *mut c_void,
    num_events: usize,
    event_paths: *mut c_void,
    event_flags: *const u32,
    _event_ids: *const u64,
) {
    if client_info.is_null() || event_paths.is_null() || event_flags.is_null() || num_events == 0 {
        return;
    }
    let state = &*(client_info as *const MacOsStreamState);
    let paths = event_paths as *const *const c_char;
    let flags = std::slice::from_raw_parts(event_flags, num_events);
    let mut events = Vec::new();

    for (index, flag) in flags.iter().enumerate() {
        let raw_path = *paths.add(index);
        if raw_path.is_null() {
            continue;
        }
        let path = CStr::from_ptr(raw_path).to_string_lossy().to_string();
        let metadata = std::fs::metadata(&path).ok();
        let kind = if flag & FSEVENT_FLAG_ITEM_REMOVED != 0 || flag & FSEVENT_FLAG_ITEM_RENAMED != 0
        {
            crate::watcher::FsEventKind::Removed
        } else if flag & FSEVENT_FLAG_ITEM_CREATED != 0 {
            crate::watcher::FsEventKind::Added
        } else if flag & FSEVENT_FLAG_ITEM_MODIFIED != 0 {
            crate::watcher::FsEventKind::Modified
        } else {
            crate::watcher::FsEventKind::Modified
        };
        events.push(crate::watcher::FsEvent {
            kind,
            path,
            is_directory: flag & FSEVENT_FLAG_ITEM_IS_DIR != 0,
            size_bytes: metadata.as_ref().map(|meta| meta.len()).unwrap_or(0),
            previous_size_bytes: None,
        });
    }

    if events.is_empty() {
        return;
    }
    let watched_dir = state
        .roots
        .iter()
        .find(|root| events.iter().any(|event| event.path.starts_with(root.as_str())))
        .cloned()
        .unwrap_or_else(|| state.roots.first().cloned().unwrap_or_default());
    let event_count = events.len();
    (state.on_batch)(crate::watcher::FsChangeBatch {
        watched_dir,
        events,
        event_count,
        timestamp_ms: now_ms(),
    });
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(unix)]
fn unix_nlink(path: &str) -> Result<u64, String> {
    Ok(std::fs::metadata(path)
        .map_err(|e| format!("metadata: {e}"))?
        .nlink())
}
