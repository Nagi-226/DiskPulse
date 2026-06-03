# DiskPulse

**Real-time disk space monitor & safe cleanup tool — Windows / Linux / macOS**

> [中文版](README_zh-CN.md)

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/tauri-2.0-6366f1)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19-06b6d4)](https://react.dev)
[![Rust](https://img.shields.io/badge/rust-1.94-orange)](https://www.rust-lang.org)
[![Windows](https://img.shields.io/badge/windows-11-0078D6)](https://www.microsoft.com/windows)
[![Linux](https://img.shields.io/badge/linux-FCC624?logo=linux)](https://kernel.org)
[![macOS](https://img.shields.io/badge/macOS-000000?logo=apple)](https://www.apple.com/macos)

DiskPulse gives you full visibility into your disk space usage and helps you reclaim wasted gigabytes — safely. Built with an Aurora-designed UI, powered by a high-performance Rust backend with native kernel-level file monitoring, intelligent anomaly detection, and committed to never losing your data.


## v0.6.6 Intelligent Operations

- **Streaming incremental scan** — first results in <500ms, Treemap updates batch-by-batch. Incremental rescan on FS changes.
- **MFT direct scan** — NTFS `FSCTL_ENUM_USN_DATA` fast approximate scan. Automatic fallback to JwalkStage.
- **Windows Service mode** — `diskpulse.exe --service` runs headless background engine with Named Pipe IPC to GUI.
- **ML anomaly detection** — Holt-Winters seasonal forecasting + Modified Z-Score detector. 4 anomaly types, pure Rust, zero ML deps.
- **Smart recommendations v2** — context-aware urgency multiplier (1.0x–3.0x), user behavior learning, cross-module correlation bonus.
- **4D disk health radar** — Space / Waste / Trend / Age sub-scores replacing single health index.
- **Custom rule editor** — create, edit, test, and delete custom cleanup rules with live pattern tester.
- **6-trait platform abstraction** — `DiskInfoProvider`, `FsWatcher`, `DirScanner`, `CleanupProvider`, `FileMetaAnalyzer`, `SystemInfo` isolate all OS-specific code behind compile-time dispatch.
- **Native Windows watcher** — `ReadDirectoryChangesW` kernel-push events (< 50ms latency, ~0% idle CPU).
- **Linux/macOS support** — inotify / FSEvents-ready, CI matrix for 3-platform builds.
- **CI/CD** — GitHub Actions 3-platform matrix: Windows (MSI + NSIS), Linux (.deb + .AppImage), macOS (.dmg).

## ✨ Features

- **Interactive treemap visualization** — see exactly what's eating your disk, drill down to any subdirectory
- **Smart risk classification** — 16 built-in rules + custom rule editor categorize every directory as Low / Medium / High risk
- **One-click safe cleanup** — all deletions go to Recycle Bin / Trash, never permanent
- **Multi-drive support** — scan any drive with real-time streaming progress
- **Cleanup report** — search, filter, sort classified items; guided 5-step Cleanup Wizard
- **Native FS monitoring** — kernel-level file change events (Windows ReadDirectoryChangesW, Linux inotify)
- **Duplicate detection** — 3-phase pipeline (size → 4KB hash → SHA-256) with hard-link awareness
- **File aging analysis** — 7 time buckets, zombie file finder, growth hotspot detection
- **Smart recommendations v2** — context-aware scoring with urgency multiplier, behavior learning, and correlation bonus
- **4D disk health radar** — Space / Waste / Trend / Age sub-scores + ECharts radar visualization
- **ML anomaly detection** — Holt-Winters seasonal forecasting + Modified Z-Score; 4 anomaly types
- **Parallel scan engine** — jwalk + rayon + streaming; 500GB drives in under 5 seconds
- **Real-time alerts** — low-space thresholds + sudden growth + anomaly detection via native notifications
- **Windows Service mode** — headless background monitoring with system tray integration
- **Auto-cleanup scheduler** — configurable LOW-risk automatic cleanup
- **i18n** — English + Simplified Chinese, auto-detect OS language
- **Dark/Light themes** — Aurora design system with CSS variable tokens

## 🛡 Safety-first Design

DiskPulse was built from the ground up with these principles:

| Rule | Detail |
|------|--------|
| Recycle Bin only | No permanent delete code path exists in the app |
| Whitelist validation | Only deletes paths matching known-safe patterns (temp, cache, downloads, logs) |
| System path protection | `C:\Windows`, `Program Files`, `System32`, `WinSxS` — never touched |
| File lock detection | Files in use are skipped, never force-deleted |
| Pre-delete path check | Every path verified to exist + pass all rules before deletion |
| Preview before execute | See exactly what will be cleaned with full path listing |

## 🎨 Aurora Design System

A custom "Windows 11 Fluent meets data visualization" design language:

- **Deep space palette** — `#06080d` backgrounds with indigo/cyan gradient accents
- **Glass-morphism cards** — frosted glass with backdrop blur
- **Animated ring chart** — drive usage with glowing drop shadow
- **Shimmer progress bars** — beautiful scanning indicators
- **Live monitoring dot** — green pulsing indicator for real-time mode
- **Dark/Light themes** — CSS variable tokens, auto-match system preference

## 🚀 Quick Start

### Prerequisites

- **Windows 11** / **Linux** / **macOS**
- **Node.js** ≥ 22
- **Rust** ≥ 1.94
- **Windows**: Microsoft Visual C++ Build Tools
- **Linux**: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`
- **macOS**: Xcode Command Line Tools

### Development

```bash
# Clone
git clone https://github.com/Nagi-226/DiskPulse.git
cd DiskPulse

# Install frontend dependencies
npm install

# Launch in dev mode (Vite + Tauri)
npm run tauri dev
```

### Build

```bash
# Production build
npm run tauri build

# Platform-specific artifacts:
#   Windows: .msi + .exe (NSIS)
#   Linux:   .deb + .AppImage
#   macOS:   .dmg
```

### CLI Mode

```bash
# Scan a drive
diskpulse --cli scan C

# Full health check
diskpulse --cli health C --json

# Dry-run cleanup preview
diskpulse --cli clean C --dry-run

# Export scan report
diskpulse --cli export C json scan
```

## 🏗 Architecture

```
Frontend (React/TS)  <-->  Tauri IPC  <-->  Rust Backend
     |                                      |
  ECharts/D3                    ┌──────────┴──────────┐
  Tailwind CSS                  │  6-trait platform    │
  lucide-react                  │  abstraction layer   │
  react-i18next                 ├──────────────────────┤
                                │ Win │ Linux │ macOS │
                                └──────────────────────┘
                                walkdir/jwalk + rayon
                                rusqlite (SQLite)
                                windows-rs / inotify / FSEvents
```

| Layer | Stack |
|-------|-------|
| Desktop Shell | Tauri 2.x |
| Backend | Rust — 20 modules, 6 platform traits, 86 tests |
| Frontend | React 19 + TypeScript 5 + Tailwind CSS 4 |
| Visualization | ECharts 6 + D3 7 |
| Storage | SQLite (via rusqlite) |
| Platform APIs | windows crate 0.58 / inotify FFI / FSEvents + sysctl |
| Knowledge Graph | graphify-rs — 995 nodes, 1356 edges |

## 📦 Project Status

| Version | Feature | Status |
|---------|---------|--------|
| v0.0.1–0.0.9 | Core foundation: scanner, risk, cleanup, watcher, history, settings | ✅ |
| v0.1.0 | Production release candidate | ✅ |
| v0.2.0 | Performance & UX optimization | ✅ |
| v0.2.5–0.2.9 | Intelligent insights: alerts, prediction, large files, auto-cleanup | ✅ |
| v0.3.0 | Production release | ✅ |
| v0.4.0 | Extensible intelligence platform (i18n, themes, duplicates, aging, recommendations) | ✅ |
| v0.5.0 | Integration excellence (cross-module wiring, CLI, wizard, notifications) | ✅ |
| **v0.6.0** | **Cross-platform performance foundation (native watcher, 6 traits, Linux, macOS)** | ✅ |
| v0.7.0 | Intelligent operations (planned) | 📋 |

## ⌨️ IPC Commands

```rust
// Scanner
scan_drive(drive: String) -> DriveInfo
scan_drive_meta(drive: String) -> DriveMeta
scan_drive_dirs(drive: String) -> Vec<DirInfo>
cancel_scan() -> ()
find_large_files(drive: String, min_size: u64, limit: usize) -> Vec<FileEntry>
cancel_large_file_scan() -> ()
list_drives() -> Vec<String>
scan_directory(path: String) -> Vec<DirInfo>

// Risk
classify_risks(scan: DriveInfo) -> RiskReport

// Cleaner
preview_cleanup(items: Vec<CleanItem>) -> CleanPreview
clean_items(items: Vec<CleanItem>) -> CleanResult
undo_cleanup(original_paths: Vec<String>) -> RestoreResult
run_auto_cleanup_now() -> CleanResult
get_auto_cleanup_status() -> AutoCleanupStatus
get_auto_cleanup_history() -> Vec<AutoCleanupReport>

// Watcher
start_fs_watcher() -> String
stop_fs_watcher() -> String

// Alert
start_alert_monitor() -> String
stop_alert_monitor() -> String

// History
get_snapshot_history(drive: String, days: u32) -> Vec<Snapshot>
get_cleanup_history() -> Vec<CleanupLog>
predict_disk_usage(drive: String, days: u32) -> Prediction

// Duplicates & Aging
scan_duplicates(drive: String, min_size: u64) -> Vec<DuplicateGroup>
cancel_duplicate_scan() -> ()
analyze_file_aging(drive: String) -> AgingReport
cancel_aging_scan() -> ()

// Recommendations
get_recommendations(drive: String) -> Vec<Recommendation>
get_disk_health(drive: String) -> DiskHealth

// Rules & Export
create_custom_rule(name: String, pattern: String, risk_level: String) -> RiskRule
delete_custom_rule(rule_id: String) -> ()
export_scan_report(drive: String, format: String) -> String
export_cleanup_history(format: String) -> String
export_duplicates(drive: String, format: String) -> String

// Notifications
get_notifications() -> Vec<NotificationRecord>
mark_notifications_read() -> ()
mark_notification_read(id: i64) -> ()
clear_notifications() -> ()

// System (v0.6.0)
get_system_info() -> PlatformSystemInfo
get_file_meta(path: String) -> FileMeta

// Settings
get_settings() -> AppSettings
save_settings(settings: AppSettings) -> ()
get_rules() -> Vec<RiskRule>
save_rule_override(rule_id: String, safe_to_delete: bool) -> ()

// App
app_version() -> String
```

## 🤝 Contributing

Contributions are welcome! Please read the guidelines:

1. **Branch naming**: `feature/v0.0.X-description` or `fix/description`
2. **Commit format**: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`
3. **Rust**: `rustfmt` + `clippy` must pass, no `unwrap()` in production code
4. **TypeScript**: Strict mode, no `any` types
5. **Safety PRs**: Changes to `cleaner/` module require thorough test coverage + review

Check [CLAUDE.md](CLAUDE.md) for detailed development context, [PROGRESS.md](PROGRESS.md) for current progress, and [CODEX.md](CODEX.md) for implementation tasks.

## 📄 License

MIT © 2026 [Nagi_226](https://github.com/Nagi-226)
