# DiskPulse Progress Snapshot

> **Last updated**: 2026-05-06
> **Purpose**: Fast context sync for resuming DiskPulse development.

## Current Baseline

- **Current version**: `v0.1.0` — Production Release Candidate
- **Status**: Production Release Candidate — code complete, build verified, installer generated
- **Last verified**: typecheck (0 errors), clippy (0 warnings), cargo test (36/36 passed), tauri build (successful, MSI + NSIS generated)

## What Works Right Now

| Component | File(s) | Verified |
|-----------|---------|----------|
| Tauri 2 scaffold | `src-tauri/tauri.conf.json` | ✅ |
| Scanner (parallel walkdir + rayon) | `src-tauri/src/scanner/mod.rs` | ✅ 3 tests |
| Risk engine (16 rules) | `src-tauri/src/risk/mod.rs` | ✅ 6 tests |
| Cleanup engine (Recycle Bin) | `src-tauri/src/cleaner/mod.rs` | ✅ 14 tests |
| FS watcher (polling) | `src-tauri/src/watcher/mod.rs` | ✅ 5 tests |
| SQLite database | `src-tauri/src/db/mod.rs` | ✅ 8 tests |
| Tauri IPC (16 commands) | `src-tauri/src/lib.rs` | ✅ registered |
| System tray | `src-tauri/src/lib.rs` | ✅ |
| React dashboard + treemap | `src/App.tsx`, `src/components/Treemap.tsx` | ✅ |
| Cleanup report page | `src/pages/Cleanup/` | ✅ |
| Cleanup preview panel | `src/components/CleanupPreview.tsx` | ✅ |
| History & trends page | `src/pages/History/` | ✅ |
| Settings page | `src/pages/Settings/` | ✅ |
| FS events hook | `src/hooks/useFsEvents.ts` | ✅ |
| TypeScript types | `src/types.ts` | ✅ |
| Aurora design system | `src/index.css` | ✅ |

## Safety Baseline

- All cleanup goes to **Recycle Bin** via SHFileOperationW (FOF_ALLOWUNDO).
- High-risk / system-protected paths are **blocked** at validation.
- `safety_check` runs **rule + runtime (existence + file lock)** checks before each delete.
- **Cancellation token** supported for aborting mid-cleanup.
- **Undo/restore** via Recycle Bin $I info file parsing.

## Version Status

| Version | Name | Status | Notes |
|---------|------|--------|-------|
| v0.0.1 | Scaffold | ✅ | Tauri/React/Rust scaffold |
| v0.0.2 | Scanner Polish | ✅ | Progress events + multi-drive |
| v0.0.3 | Dashboard Treemap | ✅ | ECharts treemap + drill-down |
| v0.0.4 | Risk Engine | ✅ | 16 rules, 3-tier classification |
| v0.0.5 | Cleanup Report | ✅ | Risk-grouped layout + filtering |
| v0.0.6 | Safe Cleanup | ✅ | Recycle Bin, undo, progress events |
| v0.0.7 | FS Watcher + Tray | ✅ | Polling watcher + tray menu |
| v0.0.8 | History & Trends | ✅ | SQLite + ECharts trends + timeline |
| v0.0.9 | Settings | ✅ | Preferences, rules config, about |
| v0.1.0 | Release Candidate | ✅ | Code complete, build verified, MSI + NSIS generated |
| v0.2.0 | Performance & UX | 📋 | Planning — see `docs/v0.2.0-plan.md` |

## v0.1.0 Release Checklist

- [x] Version bump to 0.1.0 (Cargo.toml, package.json, tauri.conf.json)
- [x] MIT LICENSE file
- [x] Plugin permissions (dialog, notification, opener)
- [x] CSP security policy
- [x] clippy 0 warnings
- [x] .gitignore Rust entries
- [x] CLAUDE.md updated
- [x] CHANGELOG.md created
- [x] Frontend-backend command alignment
- [x] All 36 tests passing
- [x] TypeScript typecheck passing
- [x] `npm run tauri build` verified
- [x] MSI installer tested

## Complete File Inventory

### Rust Backend (`src-tauri/src/`)
| File | Purpose | Lines | Tests |
|------|---------|-------|-------|
| `main.rs` | App entry point, invokes lib | 7 | — |
| `lib.rs` | 16 IPC commands, tray, DB init, auto-startup | ~340 | — |
| `scanner/mod.rs` | Parallel walkdir + rayon, drive/dir scanning | 268 | 3 |
| `risk/mod.rs` | 16 risk rules, classification, override logic | 452 | 6 |
| `cleaner/mod.rs` | Recycle Bin cleanup, undo, safety checks | 835 | 14 |
| `watcher/mod.rs` | Polling FS monitor, snapshot diff, debounce | 314 | 5 |
| `db/mod.rs` | SQLite: snapshots, logs, settings, overrides | 501 | 8 |

### Frontend (`src/`)
| File | Purpose |
|------|---------|
| `main.tsx` | React entry, strict mode |
| `App.tsx` | Main app: sidebar, dashboard, routing, event listeners |
| `types.ts` | Shared TypeScript interfaces (14 types) |
| `index.css` | Aurora design system, Tailwind, animations |
| `components/Treemap.tsx` | D3/ECharts treemap visualization |
| `components/CleanupPreview.tsx` | Cleanup safety check, execution, undo UI |
| `pages/Cleanup/index.tsx` | Cleanup report: risk-grouped, search/filter |
| `pages/History/index.tsx` | Trend chart (ECharts), snapshot table, cleanup timeline |
| `pages/Settings/index.tsx` | General prefs, risk rules config, about |
| `hooks/useFsEvents.ts` | FS watcher hook (start/stop/listen) |
| `utils/format.ts` | Byte formatting utility |

### Configuration
| File | Purpose |
|------|---------|
| `package.json` | v0.1.0, npm scripts, deps |
| `src-tauri/Cargo.toml` | v0.1.0, Rust deps (tauri 2, rusqlite, windows, rayon) |
| `src-tauri/tauri.conf.json` | v0.1.0, CSP, window config |
| `src-tauri/capabilities/default.json` | Permissions: core, opener, dialog, notification |
| `vite.config.ts` | Vite + React + Tailwind plugin |
| `tsconfig.json` | TypeScript strict mode |

### Documentation
| File | Purpose |
|------|---------|
| `CLAUDE.md` | Architecture, conventions, safety rules, IPC API |
| `PROGRESS.md` | Version status, file inventory, release checklist |
| `CHANGELOG.md` | Full changelog v0.0.1 through v0.1.0 |
| `LICENSE` | MIT |
| `README_zh-CN.md` | Chinese README |

## Notes

- Rust backend compiles with `cargo build`, tray-icon feature enabled.
- Frontend requires `npm run dev` for HMR development.
- Build command: `npm run typecheck && npm run build:web` then `cargo tauri build`.
- The watcher uses polling (not ReadDirectoryChangesW) — configurable interval/debounce.
- Produced artifacts: `diskpulse.exe` (11.8MB), MSI (4.5MB), NSIS (3.2MB).

## v0.2.0 Planning

> Full plan at `docs/v0.2.0-plan.md`

### Startup Performance Problem

`scanDrive("C")` on mount triggers full `WalkDir` recursive scan → **10–30s delay** before meaningful data appears.

### Optimization Strategy

1. **Split scan**: `scan_drive_meta` (50ms, GetDiskFreeSpaceExW + SQLite cache) + `scan_drive_dirs` (background)
2. **Parallel scanning**: Rayon-parallel top-level directory traversal
3. **SQLite cache**: Last scan cached per drive, shown instantly on next launch
4. **Incremental results**: Treemap tiles update as each dir completes

### Performance Targets

| Metric | Before | After |
|--------|--------|-------|
| Ring chart visible | 5–30s | < 500ms |
| Treemap (cached) | 5–30s | < 1s |
| Full scan (500GB) | 10–30s | 5–15s |
| Subsequent launch | 10–30s | < 500ms |
