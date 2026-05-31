# CLAUDE.md — DiskPulse Project Context

> Context sync order: `PROGRESS.md` → `CLAUDE.md` → `CHANGELOG.md`
> Read PROGRESS.md first for version status, then this file for architecture/details.

## Project Identity

- **Name**: DiskPulse
- **Tagline**: Real-time disk space monitor & safe cleanup tool for Windows 11
- **Type**: Open source desktop application (MIT License)
- **Repository**: E:\Github Project\DiskPulse
- **Current Version**: v0.3.0

## Tech Stack (LOCKED — do not change without explicit user approval)

| Layer | Technology | Version |
|-------|-----------|---------|
| Desktop Framework | Tauri | 2.x |
| Backend Language | Rust | 1.94+ |
| Frontend Framework | React | 19.x |
| Frontend Language | TypeScript | 5.x |
| Visualization | ECharts 6 + D3 7 | — |
| Styling | Tailwind CSS | 4.x |
| Local Database | SQLite (rusqlite) | 0.31+ |
| Icons | lucide-react | — |
| Win32 API | windows crate | 0.58+ |

## Architecture Overview

```
Frontend (React/TS)  <-->  Tauri IPC  <-->  Rust Backend
     |                                      |
  ECharts/D3                          walkdir + rayon
  Tailwind CSS                        rusqlite (SQLite)
  lucide-react                        windows-rs (Win32)
```

### Rust Module Structure (src-tauri/src/)
- `main.rs` — App entry, Tauri setup, command registration
- `scanner/` — Parallel directory traversal, drive info collection
- `watcher/` — Polling-based FS monitoring with configurable interval/debounce
- `risk/` — Risk classification engine (rule-based, 16 rules)
- `cleaner/` — Safe cleanup orchestration (Recycle Bin integration)
- `db/` — SQLite storage (snapshots, cleanup logs, settings, rule overrides)
- `alert/` — Disk space alert monitor with configurable thresholds and notifications
- `prediction/` — Disk usage linear regression and forecast computation
- `scheduler/` - Auto-cleanup scheduling, LOW-risk filtering, reports, and events

### Frontend Structure (src/)
- `App.tsx` — Dashboard UI (treemap, ring chart, live feed, nav sidebar)
- `pages/Cleanup` — Risk-grouped cleanup report + one-click clean
- `pages/History` — Trend charts + snapshot history + cleanup timeline
- `pages/Settings` — Preferences, risk rules configuration, about
- `components/` — Shared UI components (Treemap, CleanupPreview, PredictionCard, LargeFileFinder, AutoCleanupStatus, Icons)
- `hooks/` — Custom React hooks (useDriveScan, useFsEvents, useLargeFileFinder)

## Critical Safety Rules (NEVER VIOLATE)

1. **All file deletes MUST go to Recycle Bin** — never permanent delete
2. **Path validation before every delete** — verify path is in allowed targets
3. **Skip locked files gracefully** — never force-delete in-use files
4. **Whitelist-only cleanup** — only delete items matching known-safe patterns
5. **Protected patterns NEVER deleted**: Windows Installer, WeChat/QQ data,
   system32, registry hives, Program Files (unless user explicitly approved)
6. **Developer cache awareness** — detect and protect active project directories

## Risk Classification System

| Level | Color | Examples | Delete Policy |
|-------|-------|---------|---------------|
| LOW | Green | Temp files, browser cache, NVIDIA DXCache, npm/pip/cargo cache | One-click safe cleanup |
| MEDIUM | Yellow | Old downloads, cursor worktrees, large logs, WinSxS (DISM) | Confirm before cleanup |
| HIGH | Red | Windows Installer, chat history, system files, Program Files | Display only, no cleanup button |

## Development Conventions

- **Branch naming**: `feature/v0.0.X-description` or `fix/description`
- **Commit format**: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`
- **Rust style**: `rustfmt` + `clippy` must pass (0 warnings), no `unwrap()` in production code
- **TypeScript style**: Strict mode, no `any` types, ESLint + Prettier
- **Testing**: Rust unit tests in each module (56 tests, all passing)
- **Performance target**: Scan 500GB drive in < 5 seconds

## Key Tauri Commands (IPC API)

```rust
// Scanner
fn scan_drive(app: AppHandle, drive: String) -> Result<DriveInfo, String>
fn scan_drive_meta(drive: String) -> Result<DriveMeta, String>
fn scan_drive_dirs(app: AppHandle, drive: String) -> Result<Vec<DirInfo>, String>
fn cancel_scan() -> Result<(), String>
fn find_large_files(app: AppHandle, drive: String, min_size: u64, limit: usize) -> Result<Vec<FileEntry>, String>
fn cancel_large_file_scan() -> Result<(), String>
fn list_drives() -> Result<Vec<String>, String>
fn scan_directory(path: String) -> Result<Vec<DirInfo>, String>

// Risk
fn classify_risks(scan: DriveInfo) -> Result<RiskReport, String>

// Cleaner
fn preview_cleanup(items: Vec<CleanItem>) -> Result<CleanPreview, String>
fn clean_items(app: AppHandle, items: Vec<CleanItem>) -> Result<CleanResult, String>
fn undo_cleanup(original_paths: Vec<String>) -> Result<RestoreResult, String>

// Watcher
fn start_fs_watcher(app: AppHandle) -> Result<String, String>
fn stop_fs_watcher() -> Result<String, String>

// History
fn get_snapshot_history(drive: String, days: u32) -> Result<Vec<Snapshot>, String>
fn get_cleanup_history() -> Result<Vec<CleanupLog>, String>
fn predict_disk_usage(drive: String, days: u32) -> Result<Prediction, String>
fn run_auto_cleanup_now(app: AppHandle) -> Result<CleanResult, String>
fn get_auto_cleanup_status() -> Result<AutoCleanupStatus, String>
fn get_auto_cleanup_history() -> Result<Vec<AutoCleanupReport>, String>

// Settings
fn get_settings() -> Result<AppSettings, String>
fn save_settings(settings: AppSettings) -> Result<(), String>
fn get_rules() -> Result<Vec<RiskRule>, String>
fn save_rule_override(rule_id: String, safe_to_delete: bool) -> Result<(), String>

// Alert
fn start_alert_monitor(app: AppHandle) -> Result<String, String>
fn stop_alert_monitor() -> Result<String, String>

// App
fn app_version() -> String
```

### IPC Events (Frontend Listeners)

| Event | Payload | Emitted By |
|-------|---------|------------|
| `scan-progress` | `ScanProgress` | `scan_drive`, `scan_drive_dirs` |
| `large-file-progress` | `LargeFileProgress` | `find_large_files` |
| `clean-progress` | `CleanProgress` | `clean_items` |
| `fs-event-batch` | `FsChangeBatch` | `start_fs_watcher` |
| `disk-space-alert` | `DiskSpaceAlertPayload` | `start_alert_monitor` |
| `auto-cleanup-complete` | `CleanResult` | `run_auto_cleanup_now`, scheduler |
| `auto-cleanup-scheduled` | `AutoCleanupStatus` | scheduler |
| `auto-scan` | `String` (drive letter) | auto-startup |
| `tray-quick-scan` | `()` | tray menu |
| `tray-toggle-monitor` | `()` | tray menu |

## Current Development State

- **Phase**: v0.3.0 — Production release complete; MSI + NSIS generated
- **Last Updated**: 2026-05-31

### All Features Complete (v0.0.1 — v0.2.0)
| Version | Feature | Status |
|---------|---------|--------|
| v0.0.1 | Project scaffold, scanner, Aurora design system | ✅ |
| v0.0.2 | Scanner progress callback, multi-drive, tests | ✅ |
| v0.0.3 | ECharts treemap, drill-down navigation | ✅ |
| v0.0.4 | Risk classification engine (16 rules) | ✅ |
| v0.0.5 | Cleanup report page, risk-grouped layout | ✅ |
| v0.0.6 | Safety pipeline, progress events, modals, undo | ✅ |
| v0.0.7 | FS watcher, live monitoring, system tray | ✅ |
| v0.0.8 | SQLite history, trend charts, cleanup timeline | ✅ |
| v0.0.9 | Settings page, preferences, rules config, about | ✅ |
| v0.1.0 | Production release: build verified, MSI + NSIS generated | ✅ |
| v0.2.0 | Performance & UX: instant startup, parallel scan, cache, watcher refresh, cancel | ✅ |

### v0.2.5–0.3.0 Roadmap
> Full plan: `docs/v0.3.0-plan.md`

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.2.5 | Alerts + Prediction | Alert monitor, low-space notifications, sudden growth, prediction card, forecast chart | ✅ |
| v0.2.6 | Large Files: Backend | `FileEntry`, `find_large_files`, `large-file-progress`, cancel, 3+ tests | ✅ |
| v0.2.7 | Large Files: Frontend | UI tab, sortable table, `useLargeFileFinder` hook, cleanup integration | ✅ |
| v0.2.8 | Auto-Cleanup: Backend | `scheduler` module, `auto_cleanup_reports` table, 5 new settings, safety invariant | ✅ |
| v0.2.9 | Auto-Cleanup: Frontend | Automation settings tab, `AutoCleanupStatus` card, history section | ✅ |
| v0.3.0 | Production Release | Integration polish, build verified, MSI + NSIS generated | ✅ |

**v0.2.5 (Complete)**:
- `alert` module with `AlertConfig`, threshold checks (percentage + absolute GB)
- Sudden growth detection with configurable time window and growth percent
- Windows native notification via tauri-plugin-notification
- New `disk-space-alert` IPC event + in-app toast banner in dashboard
- Alerts settings tab: enable/disable, threshold type/values, sudden growth params
- `prediction` module with dependency-free linear regression over SQLite snapshot history
- `predict_disk_usage` IPC command returns growth rate, confidence, projected 95% date
- Dashboard `PredictionCard` between drive ring and treemap
- History trend chart includes dashed forecast line and forecast summary
- 7 unit tests (4 alert + 3 prediction)


**v0.2.9 (Complete)**:
- Automation settings tab with enable toggle, frequency, run time, min-free-GB threshold, LOW-only invariant, Save Automation, and Run Now.
- Dashboard `AutoCleanupStatus` card wired to status, run-now, and scheduler events.
- History page auto-cleanup report timeline via `get_auto_cleanup_history`.
- Dashboard toast feedback for `auto-cleanup-complete` and `auto-cleanup-scheduled`.
- Verified cargo check/test/clippy, TypeScript typecheck, and web build.

**v0.3.0 (Complete)**:
- Auto-cleanup scheduler settings now apply immediately after save; stale scheduler threads are cancelled/re-applied.
- Version bumped to 0.3.0 across npm, Cargo, Cargo.lock, and Tauri config.
- Release verification passed: cargo check/test/clippy, npm typecheck/build:web, release exe launch smoke, and tauri build.
- Generated artifacts: MSI and NSIS installers under `src-tauri/target/release/bundle/`.

## Environment

- **OS**: Windows 11
- **Rust**: 1.94.1 (stable)
- **Node.js**: v23.9.0
- **npm**: 10.9.2
- **Dev machine user**: FJL03
- **Project path**: E:\Github Project\DiskPulse

## Known User Preferences

- User is a developer using Cursor IDE
- Prefers caution with C: drive operations (safety-first approach)
- Values beautiful UI (approved the cleanup report HTML design)
- Existing GitHub projects use PascalCase or snake_case naming
- Has Rust, Node.js, and full dev toolchain installed
