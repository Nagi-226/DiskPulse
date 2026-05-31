# DiskPulse

**Real-time disk space monitor & safe cleanup tool for Windows 11**

> [中文版](README_zh-CN.md)

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/tauri-2.0-6366f1)](https://tauri.app)
[![React](https://img.shields.io/badge/react-19-06b6d4)](https://react.dev)
[![Rust](https://img.shields.io/badge/rust-1.94-orange)](https://www.rust-lang.org)
[![Windows](https://img.shields.io/badge/windows-11-0078D6)](https://www.microsoft.com/windows)

DiskPulse gives you full visibility into your disk space usage and helps you reclaim wasted gigabytes — safely. Built with an Aurora-designed UI, powered by a high-performance Rust backend, and committed to never losing your data.

## ✨ Features

- **Interactive treemap visualization** — see exactly what's eating your disk, drill down to any subdirectory
- **Smart risk classification** — 16 built-in rules categorize every directory as Low / Medium / High risk
- **One-click safe cleanup** — all deletions go to Recycle Bin, never permanent
- **Multi-drive support** — scan any drive with real-time progress feedback
- **Cleanup report** — search, filter, sort, and export (HTML/CSV) classified items
- **Parallel scan engine** — walks 500GB drives in under 5 seconds with rayon

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
- **Dark theme** — designed for the modern Windows 11 aesthetic

## 🚀 Quick Start

### Prerequisites

- **Windows 11** (primary target)
- **Node.js** ≥ 22
- **Rust** ≥ 1.94 (with `stable-x86_64-pc-windows-msvc` toolchain)
- **Microsoft Visual C++ Build Tools** (for windows crate)

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
# Production build (generates .msi installer)
npm run tauri build
```

## 🏗 Architecture

```
Frontend (React/TS)  <-->  Tauri IPC  <-->  Rust Backend
     |                                      |
  ECharts/D3                           walkdir + rayon
  Tailwind CSS                         rusqlite (SQLite)
  lucide-react                         windows-rs (Win32)
```

| Layer | Stack |
|-------|-------|
| Desktop Shell | Tauri 2.x |
| Backend | Rust — scanner, risk engine, cleaner, watcher, database |
| Frontend | React 19 + TypeScript 5 + Tailwind CSS 4 |
| Visualization | ECharts 6 + D3 7 |
| Storage | SQLite (via rusqlite) |
| Win32 APIs | windows crate 0.58 |

## 📦 Project Status

| Version | Feature | Status |
|---------|---------|--------|
| v0.0.1 | Project scaffold + Aurora design | ✅ |
| v0.0.2 | Scanner polish + multi-drive + tests | ✅ |
| v0.0.3 | ECharts treemap + drill-down | ✅ |
| v0.0.4 | Risk classification engine (16 rules) | ✅ |
| v0.0.5 | Cleanup report page | ✅ |
| v0.0.6 | Safe cleanup engine (Recycle Bin + undo) | ✅ |
| v0.0.7 | Real-time FS watcher + system tray | ✅ |
| v0.0.8 | History trends + SQLite snapshots | ✅ |
| v0.0.9 | Settings page (prefs, rules, about) | ✅ |
| v0.1.0 | Production release candidate | ✅ |
| v0.2.0 | Performance & UX optimization | ✅ |
| v0.2.5 | Intelligent insights — alerts & prediction | ✅ |

## ⌨️ IPC Commands

```rust
// Scanner
scan_drive(drive: String) -> DriveInfo
scan_drive_meta(drive: String) -> DriveMeta
scan_drive_dirs(drive: String) -> Vec<DirInfo>
cancel_scan() -> ()
list_drives() -> Vec<String>
scan_directory(path: String) -> Vec<DirInfo>

// Risk
classify_risks(scan: DriveInfo) -> RiskReport

// Cleaner
preview_cleanup(items: Vec<CleanItem>) -> CleanPreview
clean_items(items: Vec<CleanItem>) -> CleanResult
undo_cleanup(original_paths: Vec<String>) -> RestoreResult

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

Check [CLAUDE.md](CLAUDE.md) for detailed development context, and [PROGRESS.md](PROGRESS.md) for current progress.

## 📄 License

MIT © 2026 [Nagi_226](https://github.com/Nagi-226)
