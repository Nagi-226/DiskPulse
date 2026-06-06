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

DiskPulse gives you full visibility into your disk space usage and helps you reclaim wasted gigabytes — safely. Built with an Aurora-designed UI, powered by a high-performance Rust backend with native kernel-level file monitoring, intelligent anomaly detection, deep-learning-ready intelligence pipeline, and committed to never losing your data.


## v0.9.1 Local-ready Relay Server

- **Self-hosted relay binary** - `diskpulse-relay` starts a local WebSocket relay on `127.0.0.1` for self-hosted smoke runs.
- **Relay client IPC** - `connect_relay`, `disconnect_relay`, `get_relay_status`, and `list_cloud_devices` expose relay state to the desktop app.
- **Read-only route guard** - Relay envelopes reuse the Hub remote-command allowlist and refuse cleanup/write commands without local confirmation.
- **Local-ready scope** - Public relay deployment, DNS/TLS, and real cross-WAN two-device validation are external CI/ops gates.

## v0.9.0 Full Intelligence

- **AE anomaly foundation** - 6D snapshot features, deterministic 6->4->6 autoencoder inference, and synthetic training samples for v0.8.4.
- **File classifier Stage 3** - 8D feature extraction, extended magic signatures, 12-class softmax-style classifier output, and model version metadata for v0.8.5.
- **File-category risk rules** - Built-in cleanup rules can now match `file_category` conditions such as `dev_cache`, `build`, and `dependency`.
- **External storage detection** - Cross-platform storage abstraction, Windows `WM_DEVICECHANGE` event model, Linux/macOS fallback providers, and storage attach/detach IPC events.
- **5-language i18n** - English, Simplified Chinese, Japanese, Korean, and Spanish with auto system-language resolution.
- **AI Model panel** - Settings -> AI Model shows AE/classifier versions, AUC/accuracy metrics, 60-snapshot fine-tune gate, and reset action.

- **Disk fragmentation analysis** — Sampled extent-based fragmentation scoring (FSCTL_GET_RETRIEVAL_POINTERS / FIEMAP / F_LOG2PHYS), top directory/file summaries, `FragmentationView` UI.
- **6D disk health v2** — Space / Waste / Trend / Age / Frag / Anomaly radar with health snapshot history and trend tracking.
- **Predictive cleanup** — Disk-full prediction with confidence intervals, cleanup gain simulation, pre-cleanup candidate ranking with confirmation guard.
- **Smart file classification** — Extension + magic-byte classification pipeline, `file_category` on large-file, duplicate, and aging file entries.
- **Anomaly fusion fallback** — Runtime fusion weighting (healthy/degraded/disabled) for Holt-Winters + Modified Z-Score + optional Autoencoder signals.
- **Code signing foundation** — SignPath config + CI signing hook + Homebrew Cask template; external approval pending.
- **Linux native CI** — ubuntu-latest deps, .deb/.AppImage verification, trash-rs fallback, inotify parser coverage.
- **macOS FSEvents** — Native CoreServices FSEvents watcher replacing polling; .dmg artifact verification.
- **Code split + auto-update** — React.lazy route splitting (first screen <300KB gzip), GitHub Release update checker.
- **i18n + perf** — Japanese locale, 10 synthetic benchmarks, edge-case fixes.
- **Streaming incremental scan** — First results in <500ms, Treemap updates batch-by-batch. Incremental rescan on FS changes.
- **MFT direct scan** — NTFS `FSCTL_ENUM_USN_DATA` fast approximate scan. Automatic fallback to JwalkStage.
- **Windows Service mode** — `diskpulse.exe --service` runs headless background engine with Named Pipe IPC to GUI.
- **ML anomaly detection** — Holt-Winters seasonal forecasting + Modified Z-Score detector. 4 anomaly types, pure Rust, zero ML deps.
- **Smart recommendations v2** — Context-aware urgency multiplier (1.0x–3.0x), user behavior learning, cross-module correlation bonus.
- **Multi-device Dashboard** — Local WebSocket Hub, mDNS discovery, 6-digit pairing tokens, remote read-only scans.
- **Custom rule editor** — Create, edit, test, and delete custom cleanup rules with live pattern tester.
- **6-trait platform abstraction** — `DiskInfoProvider`, `FsWatcher`, `DirScanner`, `CleanupProvider`, `FileMetaAnalyzer`, `SystemInfo` isolate all OS-specific code behind compile-time dispatch.
- **CI/CD** — GitHub Actions 3-platform matrix: Windows (MSI + NSIS), Linux (.deb + .AppImage), macOS (.dmg).

## ✅ Features

- **Interactive treemap visualization** — See exactly what's eating your disk, drill down to any subdirectory
- **Smart risk classification** — 19 built-in rules + custom rule editor categorize every directory as Low / Medium / High risk
- **One-click safe cleanup** — All deletions go to Recycle Bin / Trash, never permanent
- **Multi-drive support** — Scan any drive with real-time streaming progress
- **Cleanup report** — Search, filter, sort classified items; guided 5-step Cleanup Wizard
- **Native FS monitoring** — Kernel-level file change events (Windows ReadDirectoryChangesW, Linux inotify, macOS FSEvents)
- **Duplicate detection** — 3-phase pipeline (size → 4KB hash → SHA-256) with hard-link awareness
- **File aging analysis** — 7 time buckets, zombie file finder, growth hotspot detection
- **Smart recommendations v2** — Context-aware scoring with urgency multiplier, behavior learning, and correlation bonus
- **6D disk health radar** — Space / Waste / Trend / Age / Frag / Anomaly sub-scores + ECharts radar visualization
- **Disk fragmentation analysis** — Extent-based fragmentation scoring with top directory/file summaries
- **Predictive cleanup** — Disk-full prediction, cleanup gain simulation, pre-cleanup candidates with confirmation guard
- **Smart file classification** — Extension + magic-byte + Stage 3 pipeline, file category on scan entries
- **ML anomaly detection** — Holt-Winters seasonal forecasting + Modified Z-Score; 4 anomaly types with fusion fallback
- **Parallel scan engine** — jwalk + rayon + streaming; 500GB drives in under 5 seconds
- **Real-time alerts** — Low-space thresholds + sudden growth + anomaly detection via native notifications
- **Windows Service mode** — Headless background monitoring with Named Pipe IPC and system tray integration
- **Multi-device Dashboard** — Discover and monitor paired DiskPulse devices on the LAN
- **Auto-cleanup scheduler** — Configurable LOW-risk automatic cleanup
- **i18n** — 5 languages: English, 简体中文, 日本語, 한국어, Español, auto-detect OS language
- **Dark/Light themes** — Aurora design system with CSS variable tokens

## 🛡️ Safety-first Design

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
- **Glass-morphism cards** — Frosted glass with backdrop blur
- **Animated ring chart** — Drive usage with glowing drop shadow
- **Shimmer progress bars** — Beautiful scanning indicators
- **Live monitoring dot** — Green pulsing indicator for real-time mode
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
  ECharts/D3                    ┌────────────────────┐
  Tailwind CSS                  │ 6-trait platform   │
  lucide-react                  │ abstraction layer  │
  react-i18next                 └────────────────────┘
                                walkdir/jwalk + rayon
                                rusqlite (SQLite)
                                windows-rs / inotify / FSEvents
```

| Layer | Stack |
|-------|-------|
| Desktop Shell | Tauri 2.x |
| Backend | Rust — 40 source files, 6 platform traits, 147 tests |
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
| **v0.7.0** | **Intelligent operations platform (119 tests, multi-device dashboard)** | ✅ |
| **v0.7.1** | **Code signing foundation (SignPath config, Homebrew Cask template, CI signing hook)** | ✅ Local |
| **v0.7.2** | **Linux native CI configuration (ubuntu-latest deps, .deb/.AppImage verification, trash-rs fallback)** | ✅ Local |
| **v0.7.5** | **Production-ready hardening (macOS FSEvents, code splitting, update check, perf bench, Japanese locale)** | ✅ Local |
| **v0.8.0** | **Production-Ready Deep Intelligence (fragmentation, anomaly fusion, 6D health, predictive cleanup, file classification)** | ✅ Local |
| **v0.8.1** | **SignPath Approval + Windows Signing** | ✅ Local-ready / ⏳ External |
| **v0.8.2** | **Linux Native Runner (ubuntu-latest CI)** | ✅ Local-ready / ⏳ Native |
| **v0.8.3** | **macOS Native Runner + FSEvents** | ⏳ Native |
| **v0.9.0** | **Full Intelligence: burn DL (AE + Classifier), Extended Storage, Korean/Spanish** | ✅ Local |
| **v0.9.1** | **Local-ready Relay Server (self-hosted binary, IPC/status, read-only route guard)** | ✅ Local-ready |
| **v0.10.0** | **Ecosystem: Cloud Sync Bridge + Web Dashboard** | ⏳ Planned |
| **v1.0.0** | **Public Release (180+ tests, 3-platform signed, docs synced)** | ⏳ Planned |

> Full master plan: [`docs/v1.0.0-plan.md`](docs/v1.0.0-plan.md)

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

// Recommendations & Health
get_recommendations(drive: String) -> Vec<Recommendation>
get_disk_health(drive: String) -> DiskHealth
get_health_history(drive: String, limit: usize) -> Vec<HealthSnapshot>

// Anomaly & Fragmentation (v0.8.0)
detect_anomalies(drive: String) -> AnomalyReport
analyze_fragmentation(drive: String) -> FragmentationReport
get_file_fragmentation(path: String) -> FileFragmentation
cancel_fragmentation_scan() -> ()

// Predictive Cleanup (v0.8.0)
predict_disk_full(drive: String) -> DiskFullPrediction
simulate_cleanup_gain(items: Vec<CleanItem>) -> CleanupGainEstimate
get_pre_cleanup_candidates(drive: String) -> Vec<CleanItem>
execute_pre_cleanup(items: Vec<CleanItem>) -> CleanResult

// Multi-device Hub
start_hub(port: u16) -> ()
stop_hub() -> ()
get_connected_devices() -> Vec<DeviceInfo>
get_hub_discovery_info() -> Option<DiscoveryInfo>
discover_devices(timeout_ms: u64) -> Vec<DeviceInfo>
create_pairing_token(device_name: String, ttl_seconds: u64) -> PairingToken
pair_device(token: String) -> DeviceInfo
unpair_device(device_id: String) -> ()

// Relay Server (v0.9.1)
connect_relay(url: String) -> RelayStatus
disconnect_relay() -> RelayStatus
get_relay_status() -> RelayStatus
list_cloud_devices() -> Vec<CloudDevice>

// Rules & Export
create_custom_rule(name: String, pattern: String, risk_level: String) -> RiskRule
delete_custom_rule(rule_id: String) -> ()
test_rule_pattern(pattern: String, test_path: String) -> bool
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

// Service (v0.6.4)
install_service() -> ()
uninstall_service() -> ()
get_service_status() -> ServiceStatus

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

## 📫 License

MIT © 2026 [Nagi_226](https://github.com/Nagi-226)
