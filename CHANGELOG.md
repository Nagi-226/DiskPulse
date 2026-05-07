# Changelog

All notable changes to DiskPulse will be documented in this file.

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
