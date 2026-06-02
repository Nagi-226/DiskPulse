# Changelog

All notable changes to DiskPulse will be documented in this file.

## [0.6.0] - 2026-06-02

> Full roadmap: `docs/v0.6.0-plan.md`

### Cross-Platform Performance Foundation

**6-Trait Platform Architecture:**
- Defined `DiskInfoProvider`, `FsWatcher`, `DirScanner`, `CleanupProvider`, `FileMetaAnalyzer`, `SystemInfo` traits in `platform/mod.rs`.
- Shared types: `FileIdentity`, `TrashResult`, `RestoreResult`, `WatcherGuard`, `PlatformProviders`.
- Compile-time `#[cfg(target_os)]` dispatch via `platform::providers()`.
- All scanner, watcher, cleanup, and drive listing logic routed through traits.

**Windows Native Performance:**
- `ReadDirectoryChangesW` native watcher with overlapped I/O and debounce (< 50ms latency, ~0% CPU idle).
- `WindowsPollWatcher` fallback for non-admin or network drives.
- `GetFileInformationByHandle` file metadata: hard-link count, sparse flag, file identity (volume serial + file index).
- `GetCompressedFileSizeW` for sparse file size-on-disk reporting.
- `FSCTL_SET_SPARSE` test coverage for sparse file regression.
- `JwalkStage` via `DirScanner` trait, `MftStage` as technical reserve (feature-gated).

**Hard-Link-Aware Duplicate Detection:**
- Files with identical `FileIdentity` skipped before SHA-256 hashing.
- `hard_link_count` surfaced in duplicate scan progress and `FileEntry`.
- `size_on_disk_bytes` added to `FileEntry` + LargeFileFinder UI display.

**Linux Support:**
- `statvfs`-based disk info, inotify native watcher (FFI), `/proc/mounts` drive listing.
- `gio trash` cleanup provider, `statx`-based file metadata, `/proc/meminfo` RAM reporting.
- Polling fallback if inotify fails.

**macOS Support:**
- `df`-based disk info, `osascript` trash cleanup, `stat`-based file metadata.
- `sysctl hw.memsize` RAM reporting, `sw_vers` OS version.
- Watcher uses safe polling fallback (FSEvents pending native CI validation).

**CI/CD:**
- GitHub Actions matrix workflow for `windows-latest`, `ubuntu-latest`, `macos-latest`.
- Artifact upload: MSI + NSIS (Windows), .deb + .AppImage (Linux), .dmg (macOS).

**Technical Reserve:**
- `MftStage` in `platform/windows_mft.rs` — compiled behind `mft-scanner` feature flag, not yet wired.

### Verification

- `cargo test`: 86/86 passed (up from 81).
- `cargo clippy -- -D warnings`: 0 warnings.
- `npm run typecheck`: 0 errors.
- `npm run build:web`: passed (Vite chunk-size warning only).
- `npm run tauri build`: Windows MSI + NSIS generated.
- Linux cross-compilation blocked by GTK sysroot on Windows dev machine — CI matrix handles native Linux builds.

## [0.5.0] - 2026-06-02

> Full roadmap: `docs/v0.5.0-plan.md`

### Integration Excellence

- Wired aging analysis into recommendations so `RecommendationInput.age_days` uses real per-file age data.
- Wired duplicate waste and zombie bytes into `get_disk_health`, making health checks a full cross-module scan.
- Completed CLI integration: `export <drive> <format> <type>`, `clean <drive> --dry-run`, LOW-risk cleanup execution, JSON/quiet flags, and exit-code handling.
- Added configurable recommendation weights plus duplicate and zombie thresholds in Settings -> Recommendations.
- Completed CleanupWizard's 5-step flow with scan progress, review, safe LOW-risk selection, execution, and summary states.
- Completed NotificationCenter polling, unread badge, persisted event notifications, per-item dismiss, and clear-all support.
- Added synthetic performance benchmark: `cargo bench --bench performance`.
- Bumped app/package versions to 0.5.0 across npm, Cargo, Cargo.lock, and Tauri config.

### Verification

- `cargo test`: 81/81 passed.
- `npm run typecheck`: passed.
- `cargo clippy -- -D warnings`, `npm run build:web`, and `npm run tauri build`: passed.
- Generated `DiskPulse_0.5.0_x64_en-US.msi` (SHA256 `7F3193F32EC59A4394F4ED5F355C55CBB924DE1E320AA5D210E4CF4EED55CD83`) and `DiskPulse_0.5.0_x64-setup.exe` (SHA256 `F1DCBFCA5BF3670DC6B662B42B4A54E98CBC9B37105065EC628DDC0CC2AFAAAB`).

## [0.4.0] - 2026-06-01

> Full roadmap: `docs/v0.4.0-plan.md`

### Production Release

- Bumped app/package versions to 0.4.0 across npm, Cargo, Cargo.lock, and Tauri config.
- Completed release hardening for custom rules, notification persistence, CLI parsing/execution smoke, report export, and installer generation.
- Verified `cargo test` (73/73), `cargo clippy -- -D warnings`, `npm run typecheck`, `npm run build:web`, and `npm run tauri build`.
- Generated release artifacts:
  - `src-tauri/target/release/bundle/msi/DiskPulse_0.4.0_x64_en-US.msi`
  - `src-tauri/target/release/bundle/nsis/DiskPulse_0.4.0_x64-setup.exe`
- Artifact hashes:
  - MSI SHA256: `3AAB14EC84C7794734BB9FD3E341A2F75F58E408DB1761E0E3E6552B6D1CC184`
  - NSIS SHA256: `62BCE631815A70646359991F4FBD29B5FF7472D374F96950C74E3396F39D1C8C`

### v0.4.0 Theme: Extensible Intelligence Platform

Transform DiskPulse from a monitoring & cleanup tool into an extensible disk intelligence platform — with plugin-style architecture, multi-dimensional space analysis, and guided optimization.

#### Planned Versions

| Version | Focus | Key Features |
|---------|-------|-------------|
| v0.3.1 | i18n | `react-i18next`, en/zh-CN locales, language setting |
| v0.3.2 | Themes | CSS variable tokens, Light/Dark themes, ThemeProvider |
| v0.3.3 | Performance | jwalk, streaming scan, incremental update, ScanStage trait, memory < 100MB |
| v0.3.4 | Duplicates | 3-phase detection (size→4KB→SHA-256), DuplicateFinder, cleanup integration |
| v0.3.5 | Aging | 7 aging buckets, zombie finder, growth hotspots, ECharts stacked bar |
| v0.3.6 | Recommendations | Weighted scoring model, disk health gauge, RecommendationCard |
| v0.3.7 | Rules + Export | RiskRule trait + registry, custom rule editor, CSV/JSON export |
| v0.3.8 | Wizard + Notify | 5-step CleanupWizard, NotificationCenter with SQLite storage |
| v0.3.9 | CLI + Platform | 5 subcommands (scan/duplicates/health/clean/export), cross-platform traits |
| v0.4.0 | Release | Integration tests, benchmarks, MSI + NSIS, docs |

**Extensibility Architecture (6 extension points):**
1. Risk Rule Registry (`trait RiskRule`) — new rules without touching core
2. Scanner Pipeline (`trait ScanStage`) — new scan types as plugins
3. Notification Channel (`trait NotifyChannel`) — Slack, Email, etc.
4. Cleanup Provider (`trait CleanupProvider`) — per-platform implementations
5. i18n Resource Bundle (JSON) — new language = new JSON file
6. Theme Token System (CSS variables) — new theme = new variable set

### Known Issues — Resolved in v0.5.0

| # | Issue | Resolution | Priority |
|---|-------|------------|----------|
| 1 | `RecommendationInput.age_days` always `None` | ✅ Wired aging analysis into `get_recommendations()` | 🔴 → ✅ |
| 2 | `get_disk_health()` passes hardcoded `0` for duplicate/zombie data | ✅ Full health check now scans duplicates + aging | 🔴 → ✅ |
| 3 | CLI `export` subcommand hardcodes `"C"` drive | ✅ Added `drive` field to `CliCommand::Export` | 🟡 → ✅ |
| 4 | Scoring weights and `min_size` constants are magic numbers | ✅ 7 new `AppSettings` fields + Settings UI | 🟡 → ✅ |
| 5 | `CleanupWizard` + `NotificationCenter` are UI shells | ✅ 5-step wizard + real-time polling + badge | 🟡 → ✅ |

## [0.3.9] - 2026-06-01

### Extensible Intelligence Follow-Up Slice

- Added `recommendations` backend module with weighted scoring, ranked recommendations, disk health scoring, and 3 unit tests.
- Added dashboard `RecommendationCard` with Top 5 recommendations, disk health gauge, and safe-candidate handoff into `CleanupPreview`.
- Added `report` backend module with CSV/JSON report export for scan reports, cleanup history, and duplicate results.
- Added `CleanupWizard` UI shell and `NotificationCenter` panel shell for the v0.3.8 guided cleanup/notification workflow.
- Added `cli` parser module and `platform` trait module as the first v0.3.9 CLI/platform abstraction slice.
- Registered new IPC commands: `get_recommendations`, `get_disk_health`, `export_scan_report`, `export_cleanup_history`, and `export_duplicates`.

### Remaining Before v0.4.0 Release

- Custom risk rule registry and editor are still pending beyond the report-export slice.
- Notification SQLite persistence and full notification event history are still pending.
- CLI execution mode currently parses commands; full command execution, JSON/quiet output, and exit-code contract need hardening.
- Full `npm run tauri build` release packaging was not run in this slice.

## [0.3.5] - 2026-06-01

### Foundation + Intelligence Slice

- Added i18n foundation with `react-i18next`, English and Simplified Chinese resource bundles, and persisted `AppSettings.language`.
- Added Aurora theme system with CSS-variable light/dark tokens, `ThemeProvider`, sidebar quick toggle, Settings Appearance tab, and persisted `AppSettings.theme`.
- Added scanner extensibility foundation with `ScanStage`, `ScanContext`, `MeasureStage`, and `jwalk`-backed directory measurement.
- Added duplicate file detection module with size grouping, first-4KB SHA-256 prefilter, full-file SHA-256 confirmation, progress events, cancellation, and `DuplicateFinder` UI.
- Added file aging analysis module with 7 aging buckets, zombie file candidates, recent growth hotspots, progress events, cancellation, and `AgingAnalysis` UI.
- Registered new IPC commands: `scan_duplicates`, `cancel_duplicate_scan`, `analyze_file_aging`, and `cancel_aging_scan`.
- Kept duplicate and zombie cleanup handoff routed through `CleanupPreview`; external candidates default to review-required safety posture.

### Verification

- `npm run tauri dev` launch smoke: Vite served `http://localhost:1420/` with HTTP 200 and Rust app launched.
- `cargo check` passed.
- `cargo test` passed: 62/62.
- `cargo clippy -- -D warnings` passed.
- `npm run typecheck` passed.
- `npm run build:web` passed with the existing chunk-size warning.

## [0.3.0] — 2026-05-31

### Production Release

- Bumped app/package versions to 0.3.0 across npm, Cargo, Cargo.lock, and Tauri config.
- Polished auto-cleanup settings integration so scheduler changes are applied immediately after saving, without requiring app restart.
- Added scheduler cancellation/re-apply path to prevent stale auto-cleanup threads after settings changes.
- Verified release smoke: cargo check, cargo test (56/56), cargo clippy, npm typecheck, web build, release exe launch, and Tauri bundle build.
- Generated release artifacts: `DiskPulse_0.3.0_x64_en-US.msi` and `DiskPulse_0.3.0_x64-setup.exe`.

**Artifacts:**
- MSI SHA256: `48F124C83A1FCCCE9C175B6A5778FBCCB1E3433CABCD917134035C85F53208E4`
- NSIS SHA256: `55589EED8D6BAABE393AB29AED081FB185CC07D7E4A46EA02F9826E65DCED094`

## [0.2.9] — 2026-05-31

### Auto-Cleanup — Frontend

- Added Automation settings tab with enable toggle, frequency, run time, minimum-free-space threshold, LOW-only safety copy, Save Automation, and Run Now actions.
- Added `AutoCleanupStatus` dashboard card backed by `get_auto_cleanup_status`, `run_auto_cleanup_now`, and auto-cleanup scheduler events.
- Added dashboard toast handling for `auto-cleanup-complete` and `auto-cleanup-scheduled` events.
- Added auto-cleanup report timeline to History via `get_auto_cleanup_history`.
- Kept the frontend aligned with the backend safety invariant: automatic cleanup is locked to LOW-risk, whitelisted candidates and still uses Recycle Bin cleanup.
- Verified `cargo check`, `cargo test` (56/56), `cargo clippy -- -D warnings`, `npm run typecheck`, and `npm run build:web` (chunk-size warning only).

**Next:**
- [0.3.0] — Production release: integration polish, build verified, MSI + NSIS

## [0.2.8] — 2026-05-31

### Auto-Cleanup — Backend

- Added `scheduler` Rust module with schedule calculation, status model, run-now orchestration, and scheduler thread startup.
- Added `auto_cleanup_reports` SQLite table plus save/query CRUD.
- Added 5 persisted `AppSettings` fields for auto-cleanup configuration.
- Added `run_auto_cleanup_now`, `get_auto_cleanup_status`, and `get_auto_cleanup_history` Tauri commands.
- Added `auto-cleanup-complete` and `auto-cleanup-scheduled` event emission.
- Enforced safety invariant: automatic cleanup only includes LOW-risk safe candidates and still uses the existing Recycle Bin cleanup pipeline.
- Added 5 tests covering schedule calculation, LOW-risk filtering, DB report CRUD, and settings round-trip/defaults.

**Next:**
- [0.2.9] — Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] — Production release: integration polish, build verified, MSI + NSIS

## [0.2.7] — 2026-05-31

### Large File Finder — Frontend

- Added `useLargeFileFinder` hook for `find_large_files`, `large-file-progress`, and cancellation lifecycle.
- Added `LargeFileFinder` UI with drive selector, minimum-size filter, result limit, scan progress, and sortable table.
- Added "Large Files" sidebar navigation entry.
- Added selected-file handoff into `CleanupPreview` via `additionalItems`, keeping the existing whitelist safety pipeline.
- Verified manual C: scan for files over 500MB: 6 files found in 76 seconds.

**Next:**
- [0.2.9] — Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] — Production release: integration polish, build verified, MSI + NSIS

## [0.2.6] — 2026-05-31

### Large File Finder — Backend

- Added `FileEntry` and `LargeFileProgress` shared backend models.
- Added large-file scanner using `walkdir` plus a bounded `BinaryHeap<Reverse<FileEntry>>` top-N selection.
- Added `large-file-progress` IPC event emission during scans.
- Added `find_large_files` and `cancel_large_file_scan` Tauri commands.
- Added frontend TypeScript types for the upcoming v0.2.7 UI hook/component.
- Added 3 scanner tests covering top-N ordering, min-size filtering, and cancellation.

**Next:**
- [0.2.8] — Auto-Cleanup: Backend scheduler (scheduler module, DB table, commands, tests)
- [0.2.9] — Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] — Production release: integration polish, build verified, MSI + NSIS

## [0.2.5] — 2026-05-07

### Intelligent Insights — Alerts & Prediction

> Full plan: `docs/v0.3.0-plan.md`

**Sprint 1 — Disk Space Alerts:**
- Disk space alert monitor — background thread with configurable check interval
- Low space threshold notification via tauri-plugin-notification (percentage or absolute GB)
- Sudden growth detection with configurable time window and growth percent
- New `alert` Rust module with `AlertConfig`, threshold checks, 4 unit tests
- Settings UI: new "Alerts" tab — enable/disable, threshold type/value, growth params
- Dashboard: in-app alert toast banner with auto-dismiss
- 6 new `AppSettings` fields for alert configuration

**Sprint 3 — Disk Usage Prediction:**
- New `prediction` Rust module with simple OLS linear regression over SQLite snapshots
- `predict_disk_usage` IPC command returning forecast status, confidence, growth rate, and projected 95% date
- Dashboard prediction card between drive ring and treemap
- History trend chart extended with dashed forecast line and forecast summary
- 3 unit tests for date parsing, growth projection, and insufficient-history behavior

**Upcoming:**
- [0.2.8] — Auto-Cleanup: Backend scheduler (scheduler module, DB table, commands, tests)
- [0.2.9] — Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] — Production release: integration polish, build verified, MSI + NSIS

## [0.2.0] — 2026-05-07

### Performance & UX Optimization

**Completed**:
- Split scan: `scan_drive_meta` (<50ms) + `scan_drive_dirs` (background) commands
- `useDriveScan` lazy loading hook with request cancellation
- Rayon-parallel top-level directory scanning with incremental `partial_results`
- Phase-based scan progress (Walking → Measuring → Complete)
- SQLite `DriveMeta` caching with freshness badges (Live / Cached / Metadata)
- Skeleton treemap placeholder during background scan
- `cancel_scan` command with AtomicBool cancellation token + UI cancel button
- Watcher cache refresh: detect FS changes → selective dirty top-level directory re-scan → refreshed treemap cache event

**Deferred**:
- jwalk parallel walkdir evaluation (optional)

## [0.0.9] — 2026-05-05

### Added
- Settings page with General/Rules/About tabs
- General preferences (default drive, auto-scan, auto-monitor, watcher params)
- Risk rules configuration with search, filter, and safe-to-delete toggle
- About page with version info and tech stack grid
- Settings persistence via SQLite (key-value store)

## [0.1.0] — 2026-05-06

### Production Release

First production-ready release. All core features implemented and tested.

#### Disk Scanning
- Parallel directory traversal with walkdir + rayon (500GB in < 5s target)
- Progress callback system with real-time frontend updates
- Multi-drive support with Win32 GetLogicalDrives detection
- Drill-down navigation via ECharts treemap

#### Risk Classification
- 16 built-in risk rules (temporary files, browser/GPU/dev caches, downloads, logs, system files)
- Three-tier classification: Low (safe to clean), Medium (review required), High (display only)
- Developer project awareness (detects `.git`, `package.json`, `Cargo.toml`, etc.)

#### Safe Cleanup Engine
- Recycle Bin integration via SHFileOperationW (FOF_ALLOWUNDO)
- Pre-delete validation pipeline: whitelist check → system path block → runtime lock check
- Cancellation token support for aborting mid-cleanup
- Progress events during batch cleanup
- Restore from Recycle Bin ($I info file parsing)
- Confirmation modal with itemized preview

#### Real-Time Monitoring
- Polling-based file system watcher with configurable interval/debounce
- Aggregated change batches with added/removed/modified detection
- Live event feed in dashboard sidebar
- System tray integration with quick scan, pause monitoring, exit

#### History & Trends
- SQLite-backed snapshot storage (auto-save on scan)
- ECharts trend line chart (total/used/free over time)
- Snapshot history table with expandable directory details
- Cleanup operation log with expandable per-item results

#### Settings
- General preferences (default drive, auto-scan, auto-monitor, watcher params)
- Risk rules table with search/filter and safe-to-delete toggle
- About page with version info and tech stack

#### Design System
- Aurora dark theme with glass-morphism effects
- CSS custom properties design tokens
- Responsive layout with sidebar navigation
- SVG ring chart for drive usage visualization
- Animated progress bars and transitions

## [0.0.8] — 2026-04-30

### Added
- SQLite database module (snapshots, cleanup_logs tables)
- History page with ECharts trend chart
- Snapshot history table with expandable details
- Cleanup timeline with per-item expansion
- Auto-save on scan and cleanup operations

## [0.0.7] — 2026-04-30

### Added
- Real-time file system watcher (polling-based)
- Live monitoring UI with event feed
- System tray icon with menu (quick scan, pause, exit)
- Chinese README (README_zh-CN.md)

## [0.0.6] — 2026-04-29

### Added
- Safe cleanup engine with Recycle Bin integration
- Cleanup preview with whitelist validation
- Confirmation & progress modals
- Undo/restore from Recycle Bin
- 16 unit tests for cleaner module

## [0.0.5] — 2026-04-29

### Added
- Cleanup report page with risk-grouped layout
- Search, sort, and risk-level filter controls
- HTML and CSV export functionality

## [0.0.4] — 2026-04-28

### Added
- Risk classification engine with 16 default rules
- RiskReport, RiskItem, RiskRule, RiskSummary data structures
- Developer project detection heuristic

## [0.0.3] — 2026-04-28

### Added
- ECharts treemap visualization
- Drill-down navigation with breadcrumb trail
- Color-coded directory categories

## [0.0.2] — 2026-04-28

### Added
- Scan progress callback with current path
- Multi-drive support via GetLogicalDrives
- Unit tests for scanner module

## [0.0.1] — 2026-04-28

### Added
- Initial project scaffold
- Tauri 2 + React 19 + TypeScript 5 architecture
- Disk scanner with Win32 GetDiskFreeSpaceExW
- Aurora design system with CSS custom properties
- SVG ring chart + top-20 directory bar chart
