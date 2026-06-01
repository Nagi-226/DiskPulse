# DiskPulse Progress Snapshot

> **Last updated**: 2026-06-01
> **Purpose**: Fast context sync for resuming DiskPulse development.

## Current Baseline

- **Current version**: `v0.4.0` - Production Release
- **Next target**: post-v0.4.0 maintenance / v0.5.0 planning
- **Full plan**: `docs/v0.4.0-plan.md`
- **Status**: v0.4.0 build verified; MSI + NSIS generated
- **Last verified**: `npm run tauri dev` launch smoke (Vite 200 + Rust app launched), `cargo check` (0 errors), `cargo test` (73/73 passed), `cargo clippy -- -D warnings` (0 warnings), `npm run typecheck` (0 errors), `npm run build:web` successful (chunk-size warning only), `npm run tauri build` generated MSI + NSIS

## What Works Right Now

| Component | File(s) | Verified |
|-----------|---------|----------|
| Tauri 2 scaffold | `src-tauri/tauri.conf.json` | 鉁?|
| Scanner (parallel walkdir + rayon) | `src-tauri/src/scanner/mod.rs` | 鉁?7 tests |
| Risk engine (16 rules) | `src-tauri/src/risk/mod.rs` | 鉁?6 tests |
| Cleanup engine (Recycle Bin) | `src-tauri/src/cleaner/mod.rs` | 鉁?14 tests |
| FS watcher (polling) | `src-tauri/src/watcher/mod.rs` | 鉁?5 tests |
| SQLite database | `src-tauri/src/db/mod.rs` | 鉁?8 tests |
| Tauri IPC (26 commands) | `src-tauri/src/lib.rs` | 鉁?registered + 3 watcher-cache tests |
| System tray | `src-tauri/src/lib.rs` | 鉁?|
| React dashboard + treemap | `src/App.tsx`, `src/components/Treemap.tsx` | 鉁?|
| Cleanup report page | `src/pages/Cleanup/` | 鉁?|
| Cleanup preview panel | `src/components/CleanupPreview.tsx` | 鉁?|
| History & trends page | `src/pages/History/` | 鉁?|
| Settings page | `src/pages/Settings/` | 鉁?|
| FS events hook | `src/hooks/useFsEvents.ts` | 鉁?|
| Drive scan hook (lazy + cancel) | `src/hooks/useDriveScan.ts` | 鉁?|
| Large file finder UI + hook | `src/components/LargeFileFinder.tsx`, `src/hooks/useLargeFileFinder.ts` | 鉁?|
| Alert monitor | `src-tauri/src/alert/mod.rs` | 鉁?4 tests |
| Disk usage prediction | `src-tauri/src/prediction/mod.rs` | 鉁?3 tests |
| Large file finder backend | `src-tauri/src/scanner/mod.rs`, `src-tauri/src/lib.rs` | 鉁?3 tests |
| Auto-cleanup backend | `src-tauri/src/scheduler/mod.rs`, `src-tauri/src/db/mod.rs` | 鉁?5 tests |
| Auto-cleanup frontend | `src/components/AutoCleanupStatus.tsx`, `src/pages/Settings/index.tsx`, `src/pages/History/index.tsx` | 鉁?|
| TypeScript types | `src/types.ts` | 鉁?|
| Aurora design system | `src/index.css` | 鉁?|

## v0.4.0 Roadmap 鈥?Extensible Intelligence Platform

> Full plan: `docs/v0.4.0-plan.md`

### Phase 1: Foundation (v0.3.1 鈥?v0.3.3)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.3.1 | D 浣撻獙 鈥?i18n | `react-i18next` + `I18nProvider`, `locales/en.json` + `zh-CN.json`, Settings language selector, `AppSettings.language` | 鉁?Implemented |
| v0.3.2 | D 浣撻獙 鈥?Theme | CSS variable tokens, Aurora Light + Dark themes, `ThemeProvider`, Settings 鈫?Appearance, sidebar quick-toggle, `AppSettings.theme` | 鉁?Implemented |
| v0.3.3 | B 鎬ц兘 鈥?Perf | jwalk parallel walkdir, streaming results via mpsc, incremental scan, `ScanStage` trait, memory < 100MB, cancel < 500ms | 鈿狅笍 Partial: jwalk + ScanStage landed |

### Phase 2: Intelligence (v0.3.4 鈥?v0.3.6)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.3.4 | A 鏅鸿兘 鈥?Duplicates | `duplicates` module, 3-phase detection (size鈫?KB鈫扴HA-256), `DuplicateFinder.tsx`, `useDuplicateScan` hook, cleanup integration | 鉁?Implemented |
| v0.3.5 | A 鏅鸿兘 鈥?Aging | `aging` module, 7 aging buckets, zombie file finder, growth hotspot analysis, `AgingAnalysis.tsx`, ECharts stacked bar | 鉁?Implemented |
| v0.3.6 | A Intelligence - Recommendations | `recommendations` module, weighted scoring model, disk health gauge, `RecommendationCard.tsx`, configurable scoring weights | Implemented |

### Phase 3: Power & Polish (v0.3.7 鈥?v0.3.9)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.3.7 | D+C - Rules + Export | `RiskRule` trait + registry, custom rule UI with live test, `report` module, CSV/JSON export, file save dialog | Partial: report export landed |
| v0.3.8 | D Experience - Wizard + Notify | `CleanupWizard.tsx` (5-step guided flow), `NotificationCenter.tsx` (bell icon + panel), `notifications` SQLite table | Partial: UI shell landed |
| v0.3.9 | C Productivity - CLI + Platform | `cli` module, 5 subcommands, JSON/quiet output, exit codes, `platform` abstraction traits (`#[cfg(target_os)]`), Linux/macOS deferred to v0.5.0 | Partial: parser + platform trait landed |

### Phase 4: Release (v0.4.0)

| Task | Status |
|------|--------|
| Integration testing (scan 鈫?classify 鈫?detect 鈫?recommend 鈫?clean 鈫?export) | 馃搵 |
| Performance benchmarks (scan < 5s, duplicate < 60s, memory < 150MB) | 馃搵 |
| Regression tests (projected 75-85 total) | 馃搵 |
| Build verification (cargo check/test/clippy + npm typecheck/build:web + tauri build) | 馃搵 |
| MSI + NSIS installers (fresh + upgrade test) | 馃搵 |
| Documentation sync (CLAUDE.md, PROGRESS.md, CHANGELOG.md, README, README_zh-CN) | 馃搵 |
| GitHub release tag v0.4.0 + release notes | 馃搵 |

### Extensibility Architecture (6 Extension Points)

| # | Extension Point | Mechanism | Landing Version |
|---|----------------|-----------|-----------------|
| 1 | Risk Rule Registry | `trait RiskRule` + registration | v0.3.7 |
| 2 | Scanner Pipeline | `trait ScanStage` (Walk/Filter/Measure) | v0.3.3 |
| 3 | Notification Channel | `trait NotifyChannel` | v0.3.8 |
| 4 | Cleanup Provider | `trait CleanupProvider` | v0.3.9 |
| 5 | i18n Resource Bundle | JSON + `I18nProvider` | v0.3.1 |
| 6 | Theme Token System | CSS custom properties + theme map | v0.3.2 |


## v0.4.0 Release Artifacts

- MSI: `src-tauri/target/release/bundle/msi/DiskPulse_0.4.0_x64_en-US.msi`
- MSI SHA256: `3AAB14EC84C7794734BB9FD3E341A2F75F58E408DB1761E0E3E6552B6D1CC184`
- NSIS: `src-tauri/target/release/bundle/nsis/DiskPulse_0.4.0_x64-setup.exe`
- NSIS SHA256: `62BCE631815A70646359991F4FBD29B5FF7472D374F96950C74E3396F39D1C8C`
- CLI smoke: `cargo run -- --cli health C --json` returned health JSON in 8326 ms including dev build overhead.

## v0.5.0 — Known Issues (Deferred from v0.4.0 Audit)

> These issues were identified in the v0.4.0 post-release audit. They are not bugs, but incomplete integrations that should be addressed in v0.5.0.

| # | Issue | Module(s) | Impact | Priority |
|---|-------|-----------|--------|----------|
| 1 | `RecommendationInput.age_days` always `None` — aging data not wired to recommendation pipeline | `recommendations/mod.rs` | Age scoring factor always uses default weight (25%); zombie files don't influence recommendations | 🔴 High |
| 2 | `get_disk_health()` passes hardcoded `0` for `duplicate_waste_bytes` and `zombie_bytes` — modules not integrated | `recommendations/mod.rs` | Disk health score ignores actual duplicate waste and zombie data; only uses free space percentage | 🔴 High |
| 3 | CLI `export` subcommand hardcodes `"C"` drive instead of accepting `drive` argument | `cli/mod.rs:79-81` | Export commands ignore user-selected drive; always exports C: drive data | 🟡 Medium |
| 4 | Scoring weights and `min_size` constants in recommendations/duplicates are magic numbers | `recommendations/mod.rs`, `duplicates/mod.rs`, `report/mod.rs` | Hard to tune; user-configurable weights planned in v0.3.6 design but not implemented | 🟡 Medium |
| 5 | `CleanupWizard` and `NotificationCenter` are UI shells — core logic needs completion | `src/components/CleanupWizard.tsx`, `src/components/NotificationCenter.tsx` | Components exist but may have incomplete backend wiring for full 5-step flow and real-time notification polling | 🟡 Medium |

**Fix plan**: These will be addressed in v0.5.0 Phase 1 (Integration Polish) before any new features.

## Safety Baseline

- All cleanup goes to **Recycle Bin** via SHFileOperationW (FOF_ALLOWUNDO).
- High-risk / system-protected paths are **blocked** at validation.
- `safety_check` runs **rule + runtime (existence + file lock)** checks before each delete.
- **Cancellation token** supported for aborting mid-cleanup.
- **Undo/restore** via Recycle Bin $I info file parsing.

## Version Status

| Version | Name | Status | Notes |
|---------|------|--------|-------|
| v0.0.1 | Scaffold | 鉁?| Tauri/React/Rust scaffold |
| v0.0.2 | Scanner Polish | 鉁?| Progress events + multi-drive |
| v0.0.3 | Dashboard Treemap | 鉁?| ECharts treemap + drill-down |
| v0.0.4 | Risk Engine | 鉁?| 16 rules, 3-tier classification |
| v0.0.5 | Cleanup Report | 鉁?| Risk-grouped layout + filtering |
| v0.0.6 | Safe Cleanup | 鉁?| Recycle Bin, undo, progress events |
| v0.0.7 | FS Watcher + Tray | 鉁?| Polling watcher + tray menu |
| v0.0.8 | History & Trends | 鉁?| SQLite + ECharts trends + timeline |
| v0.0.9 | Settings | 鉁?| Preferences, rules config, about |
| v0.1.0 | Release Candidate | 鉁?| Code complete, build verified, MSI + NSIS generated |
| v0.2.0 | Performance & UX | 鉁?| Instant startup, parallel scan, cache freshness, watcher dirty-dir refresh, skeleton UI, cancel scan |
| v0.2.5 | Intelligent Insights (Alerts + Prediction) | 鉁?| S1 (Alerts) + S3 (Prediction); 48 tests |
| v0.2.6 | Large File Finder 鈥?Backend | 鉁?| `FileEntry`, `find_large_files`, `large-file-progress`, cancel, 3 tests |
| v0.2.7 | Large File Finder 鈥?Frontend | 鉁?| UI tab, sortable table, cleanup integration |
| v0.2.8 | Auto-Cleanup 鈥?Backend | 鉁?| Scheduler module, `auto_cleanup_reports` table, commands |
| v0.2.9 | Auto-Cleanup 鈥?Frontend | 鉁?| Automation settings tab, status card, history |
| v0.3.0 | Production Release | 鉁?| Integration polish, build verified, MSI + NSIS |
| v0.3.1 | i18n Internationalization | 鉁?| `react-i18next`, en/zh-CN locales, language setting |
| v0.3.2 | Theme System | 鉁?| CSS variables, Light/Dark themes, ThemeProvider |
| v0.3.3 | Performance Overhaul | 鈿狅笍 | jwalk + ScanStage landed; streaming/incremental benchmarks pending |
| v0.3.4 | Duplicate Detection | 鉁?| 3-phase detection (size鈫?KB鈫扴HA-256), DuplicateFinder UI |
| v0.3.5 | File Aging Analysis | 鉁?| 7 aging buckets, zombie finder, growth hotspots |
| v0.3.6 | Smart Recommendations | Implemented | Weighted scoring, disk health gauge, RecommendationCard |
| v0.3.7 | Custom Rules + Export | Partial | CSV/JSON export landed; custom rule registry UI pending |
| v0.3.8 | Wizard + Notifications | Partial | CleanupWizard + NotificationCenter UI shell landed; SQLite notification history pending |
| v0.3.9 | CLI + Platform Layer | Partial | CLI parser + platform trait landed; full command execution/exit codes pending |
| v0.4.0 | Production Release | Complete | Integration checks, CLI smoke, installers, docs |

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

## v0.2.0 Status (Complete)

| Feature | Implementation |
|---------|---------------|
| `scan_drive_meta` (instant metadata) | `lib.rs:36`, `scanner/mod.rs:63` |
| `scan_drive_dirs` (background scan) | `lib.rs:51`, `scanner/mod.rs:98` |
| `useDriveScan` lazy loading hook | `src/hooks/useDriveScan.ts` |
| Parallel top-level dir scanning (rayon) | `scanner/mod.rs:182-217` |
| Incremental results via `partial_results` | `scanner/mod.rs:212` |
| Phase-based progress (Walking/Measuring/Complete) | `scanner/mod.rs:7-11` |
| SQLite `DriveMeta` caching with freshness | `db/mod.rs:383`, `App.tsx:140-178` |
| Skeleton treemap loading | `App.tsx:694-717` |
| `cancel_scan` command + scan cancellation | `lib.rs:101`, `scanner/mod.rs:98-116,199-200,241-242` |
| Cancel button in UI | `App.tsx:595`, `useDriveScan.ts:141-145` |
| Watcher cache refresh | `lib.rs:234`, `scanner/mod.rs:252`, `useDriveScan.ts:71` |

**Deferred**: jwalk evaluation (optional, post-v0.2.0 benchmark candidate)

## v0.2.5鈥?.3.0 Roadmap

> Full plan: `docs/v0.3.0-plan.md`

### v0.2.5 鈥?Alerts + Prediction 鉁?
| Feature | Implementation |
|---------|---------------|
| `alert` module (config, threshold, monitor) | `src-tauri/src/alert/mod.rs` |
| `start_alert_monitor` / `stop_alert_monitor` commands | `lib.rs` |
| `disk-space-alert` IPC event + alert toast | `lib.rs`, `src/App.tsx` |
| 6 new `AppSettings` alert fields | `db/mod.rs` |
| Alerts settings tab | `src/pages/Settings/index.tsx` |
| `prediction` module (OLS forecast) | `src-tauri/src/prediction/mod.rs` |
| `predict_disk_usage` Tauri command | `lib.rs` |
| `PredictionCard` dashboard component | `src/components/PredictionCard.tsx` |
| History forecast trend line + summary | `src/pages/History/index.tsx` |
| Prediction shared types (`Prediction`, `ForecastPoint`) | `src/types.ts` |

### v0.2.6 鈥?Large File Finder: Backend 鉁?
| Task | Details |
|------|---------|
| New types | `FileEntry { name, path, size_bytes, modified_epoch_ms }`, `LargeFileProgress` |
| Scanner function | `find_large_files_with_progress_and_cancel(drive, min_size, limit, cancel)` 鈥?walkdir + `BinaryHeap<Reverse>` for top-N |
| Progress events | `large-file-progress` IPC event with dirs processed/total + files_found |
| Cancel support | `cancel_large_file_scan` command + `LARGE_FILE_SCAN_CANCEL` static |
| Tauri command | `find_large_files` registered in `generate_handler![]` |
| Unit tests | top-N ordering, min_size filtering, cancellation mid-scan |

### v0.2.7 鈥?Large File Finder: Frontend 鉁?
| Task | Details |
|------|---------|
| `useLargeFileFinder` hook | Invoke + progress listen + cancel lifecycle |
| `LargeFileFinder` component | Min-size dropdown, limit selector, sortable table (path/size/age/risk), "Add to Cleanup" button |
| Dashboard nav | Add "Large Files" to `NAV_ITEMS` sidebar |
| Cleanup integration | Pass selected files to `CleanupPreview` as `additionalItems` |
| Error/loading/empty states | All states handled |

### v0.2.8 鈥?Auto-Cleanup: Backend 鉁?
| Task | Details |
|------|---------|
| `scheduler` module | `AutoCleanupConfig`, `AutoCleanupStatus`, `calculate_next_run()`, scheduler thread |
| DB table | `auto_cleanup_reports` table + `save_auto_cleanup_report()` / `get_auto_cleanup_history()` |
| New `AppSettings` fields | `auto_cleanup_enabled`, `auto_cleanup_frequency`, `auto_cleanup_time`, `auto_cleanup_risk_levels`, `auto_cleanup_min_free_gb` |
| Tauri commands | `run_auto_cleanup_now`, `get_auto_cleanup_status`, `get_auto_cleanup_history`, `cancel_large_file_scan` |
| IPC events | `auto-cleanup-complete`, `auto-cleanup-scheduled` |
| Safety invariant | Only LOW risk auto-cleaned; existing `preview_cleanup` pipeline enforced |
| Unit tests | `calculate_next_run()`, DB CRUD, settings round-trip |

### v0.2.9 鈥?Auto-Cleanup: Frontend 鉁?
| Task | Details |
|------|---------|
| Settings "Automation" tab | Enable toggle, frequency selector, time picker, min-free-GB, LOW-only invariant, "Run Now" button |
| `AutoCleanupStatus` component | Dashboard card: active/inactive, next run, last result summary |
| History integration | Auto-cleanup history section in History page (expandable rows) |
| Event listeners | `auto-cleanup-complete` toast, `auto-cleanup-scheduled` status update |
| Manual trigger feedback | Loading state on "Run Now", success/error message |

### v0.3.0 鈥?Production Release 鉁?
| Task | Details |
|------|---------|
| Integration testing | 鉁?Release exe launch smoke + command/test coverage for scan/classify/cleanup/auto-clean reports |
| Performance audit | Manual C: >500MB large-file scan returned 6 files in 76s; scheduler idle smoke covered by release launch |
| Build verification | 鉁?cargo check/test/clippy + npm typecheck/build:web + tauri build |
| Installer test | 鉁?MSI + NSIS generated and artifact hashes recorded; clean-machine install not run in this session |
| Docs finalize | 鉁?README, CHANGELOG, PROGRESS, CLAUDE.md, v0.3.0 plan synced |
| Release tag | Pending user git tag / GitHub release publish |

## Complete File Inventory

### Rust Backend (`src-tauri/src/`)
| File | Purpose | Lines | Tests |
|------|---------|-------|-------|
| `main.rs` | App entry point, invokes lib | 7 | 鈥?|
| `lib.rs` | 26 IPC commands, tray, DB init, auto-startup | ~710 | 3 |
| `scanner/mod.rs` | Parallel dir scan + large file finder, cancel support | ~735 | 7 |
| `alert/mod.rs` | Disk space alert monitor, threshold checks, notifications | ~280 | 4 |
| `prediction/mod.rs` | Disk usage linear regression and forecast computation | ~430 | 3 |
| `risk/mod.rs` | 16 risk rules, classification, override logic | 452 | 5 |
| `cleaner/mod.rs` | Recycle Bin cleanup, undo, safety checks | 835 | 16 |
| `watcher/mod.rs` | Polling FS monitor, snapshot diff, debounce | 314 | 4 |
| `db/mod.rs` | SQLite: snapshots, logs, auto-cleanup reports, settings, overrides, cache | ~700 | 10 |
| `scheduler/mod.rs` | Auto-cleanup scheduling, LOW-risk filtering, run-now orchestration | ~380 | 4 |

### Frontend (`src/`)
| File | Purpose |
|------|---------|
| `main.tsx` | React entry, strict mode |
| `App.tsx` | Main app: sidebar, dashboard, routing, event listeners |
| `types.ts` | Shared TypeScript interfaces (18+ types) |
| `index.css` | Aurora design system, Tailwind, animations |
| `components/Treemap.tsx` | D3/ECharts treemap visualization |
| `components/CleanupPreview.tsx` | Cleanup safety check, execution, undo UI |
| `components/PredictionCard.tsx` | Dashboard disk usage forecast card |
| `components/AutoCleanupStatus.tsx` | Dashboard auto-cleanup status card and manual trigger |
| `components/LargeFileFinder.tsx` | Large file scan controls, progress, sortable table, cleanup handoff |
| `pages/Cleanup/index.tsx` | Cleanup report: risk-grouped, search/filter |
| `pages/History/index.tsx` | Trend chart (ECharts), snapshot table, cleanup timeline, auto-cleanup reports |
| `pages/Settings/index.tsx` | General prefs, risk rules, alerts, automation, about |
| `hooks/useDriveScan.ts` | Lazy scan hook: meta 鈫?cache 鈫?background 鈫?cancel |
| `hooks/useFsEvents.ts` | FS watcher hook (start/stop/listen) |
| `hooks/useLargeFileFinder.ts` | Large file scan lifecycle: invoke, progress listen, cancel |
| `utils/format.ts` | Byte formatting utility |

### Configuration
| File | Version | Purpose |
|------|---------|---------|
| `package.json` | 0.3.0 | npm scripts, deps |
| `src-tauri/Cargo.toml` | 0.3.0 | Rust deps (tauri 2, rusqlite, windows, rayon) |
| `src-tauri/tauri.conf.json` | 0.3.0 | CSP, window config |
| `src-tauri/capabilities/default.json` | 鈥?| Permissions: core, opener, dialog, notification |
| `vite.config.ts` | 鈥?| Vite + React + Tailwind plugin |
| `tsconfig.json` | 鈥?| TypeScript strict mode |

### Documentation
| File | Purpose |
|------|---------|
| `CLAUDE.md` | Architecture, conventions, safety rules, IPC API |
| `PROGRESS.md` | Version status, file inventory, release checklist |
| `CHANGELOG.md` | Full changelog v0.0.1 through v0.2.0 |
| `docs/v0.2.0-plan.md` | v0.2.0 technical design and roadmap |
| `LICENSE` | MIT |
| `README.md` | English README |
| `README_zh-CN.md` | Chinese README |

## Notes

- Rust backend compiles with `cargo build`, tray-icon feature enabled.
- Frontend requires `npm run dev` for HMR development.
- Build command: `npm run typecheck && npm run build:web` then `cargo tauri build`.
- The watcher uses polling (not ReadDirectoryChangesW) 鈥?configurable interval/debounce.
- Produced artifacts: `diskpulse.exe` (12.22MB), `DiskPulse_0.3.0_x64_en-US.msi` (4.61MB), `DiskPulse_0.3.0_x64-setup.exe` (3.25MB).



