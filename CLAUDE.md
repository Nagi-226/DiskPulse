# CLAUDE.md — DiskPulse Project Context

> This file prevents context loss during long development sessions.
> Read this FIRST when resuming work on DiskPulse.

## Project Identity

- **Name**: DiskPulse
- **Tagline**: Real-time disk space monitor & safe cleanup tool for Windows 11
- **Type**: Open source desktop application (MIT License)
- **Repository**: E:\Github Project\DiskPulse
- **Current Version**: v0.0.1 (initial scaffold)

## Tech Stack (LOCKED — do not change without explicit user approval)

| Layer | Technology | Version |
|-------|-----------|---------|
| Desktop Framework | Tauri | 2.x |
| Backend Language | Rust | 1.94+ |
| Frontend Framework | React | 19.x |
| Frontend Language | TypeScript | 5.x |
| Visualization | ECharts 5 + D3 7 | — |
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
- `watcher/` — ReadDirectoryChangesW real-time FS monitoring
- `risk/` — Risk classification engine (rule-based)
- `cleaner/` — Safe cleanup orchestration (Recycle Bin integration)
- `db/` — SQLite storage (snapshots, cleanup logs, config)

### Frontend Structure (src/)
- `pages/Dashboard` — Treemap visualization + drive overview
- `pages/Cleanup` — Risk-grouped cleanup report + one-click clean
- `pages/History` — Cleanup history + trend charts
- `pages/Settings` — Preferences, risk rules config, about
- `components/` — Shared UI components
- `hooks/` — Custom React hooks (useTauriCommand, useFsEvents)

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
- **Rust style**: `rustfmt` + `clippy` must pass, no `unwrap()` in production code
- **TypeScript style**: Strict mode, no `any` types, ESLint + Prettier
- **Testing**: Rust unit tests in each module, React Testing Library for UI
- **Performance target**: Scan 500GB drive in < 5 seconds

## Key Tauri Commands (IPC API)

```rust
#[tauri::command]
fn scan_drive(drive: String) -> Result<DriveInfo, String>

#[tauri::command]
fn classify_risks(scan: DriveInfo) -> Result<RiskReport, String>

#[tauri::command]
fn clean_items(items: Vec<CleanItem>) -> Result<CleanResult, String>

#[tauri::command]
fn get_snapshot_history(days: u32) -> Result<Vec<Snapshot>, String>

#[tauri::command]
fn get_cleanup_history() -> Result<Vec<CleanupLog>, String>
```

## Current Development State

- **Phase**: v0.0.1 — Project Scaffold (~70% complete)
- **Last Updated**: 2026-04-28 19:58
- **Overall Progress**: v0.0.1 at ~70% | Cross-version total at ~8%

### What EXISTS and WORKS

#### Rust Backend (src-tauri/)
| File | Status | Description |
|------|--------|-------------|
| `src/main.rs` | ✅ Working | App entry point |
| `src/lib.rs` | ✅ Working | Registers `scan_drive` + `app_version` Tauri commands; inits dialog/notification plugins |
| `src/scanner/mod.rs` | ✅ Working | Core scanner: `DriveInfo`/`DirInfo` structs, `scan_drive()` using `GetDiskFreeSpaceExW` + walkdir+rayon parallel traversal; skips `$Recycle.Bin` and `System Volume Information` |
| `Cargo.toml` | ✅ Compiled | tauri 2, walkdir 2, rusqlite 0.31 (bundled), rayon 1, windows 0.58 (Win32_Storage_FileSystem, Win32_Foundation), anyhow 1, serde/serde_json 1, tauri-plugin-dialog/notification/opener 2 |

#### Frontend (src/)
| File | Status | Description |
|------|--------|-------------|
| `src/App.tsx` | ✅ Written | React dashboard with SVG ring chart for drive usage %, top-20 directory bar chart, scan button, `formatSize()` helper |
| `src/App.css` | ✅ Written | Dark theme matching original HTML report (`--bg: #0f1117`, `--accent: #6c5ce7`) |
| `src/main.tsx` | ✅ Default | Vite React entry point |
| `package.json` | ✅ Installed | v0.0.1; @tauri-apps/api 2.10.1, @tauri-apps/cli 2.10.1, react 19.2.5, typescript 5.8.3, vite 7.3.2 |

#### Compilation
- `cargo check` ✅ passes
- `tsc --noEmit` ✅ passes

### What is MISSING (v0.0.1 blockers)

| # | Task | Priority | Details |
|---|------|----------|---------|
| 1 | Install frontend viz deps | 🟡 Medium | `echarts`, `d3`, `tailwindcss`, `lucide-react` not in package.json yet. Current UI uses raw SVG. Run: `npm install echarts d3 @types/d3 tailwindcss @tailwindcss/vite lucide-react` |
| 2 | `npm run tauri dev` verification | 🔴 High | App has NEVER been launched. Need to verify window opens, frontend renders, `scan_drive` IPC works end-to-end |
| 3 | `npm run tauri build` verification | 🔴 High | Production build never tested. Need to confirm .msi/.exe generation |
| 4 | Git init + first commit | 🔴 High | `.gitignore` exists but `git init` never run. No version control baseline |

### What is ALREADY DONE from v0.0.2 (bonus)

The scanner module (`src/scanner/mod.rs`) already implements v0.0.2's core deliverable:
- ✅ Parallel directory traversal with walkdir + rayon
- ✅ `scan_drive(drive_letter) -> DriveInfo` Tauri command
- ✅ `DriveInfo` struct with total/used/free + top-level dir sizes
- ✅ JSON serialization to frontend

Still missing from v0.0.2:
- [ ] Progress callback for large scans
- [ ] Unit tests for scanner module

### Next Agent: What to do FIRST

1. `cd "E:/Github Project/DiskPulse"`
2. `npm install echarts d3 @types/d3 tailwindcss @tailwindcss/vite lucide-react`
3. Configure Tailwind in `vite.config.ts`
4. Run `npm run tauri dev` — fix any runtime errors
5. Verify scan C: drive works in the running app
6. `git init && git add . && git commit -m "feat: v0.0.1 project scaffold with working scanner"`

### Known Issues & Pitfalls

- **PowerShell `$` variable swallowing**: When writing .ps1 scripts from Bash, `$` gets interpreted. Write .ps1 files instead of inline commands.
- **Chinese characters garbled in PowerShell**: Use English-only output in scripts.
- **DISM needs admin**: `DISM /Online` requires elevation — deferred to v0.0.9.
- **File edit EBUSY on Cargo.toml**: If Edit tool fails, use Write tool to overwrite entire file.
- **`encode_wide()` import**: Requires `use std::os::windows::ffi::OsStrExt` — already in scanner/mod.rs.

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
