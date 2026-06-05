# DiskPulse Progress Snapshot

> **Last updated**: 2026-06-05
> **Purpose**: Fast context sync for resuming DiskPulse development.

## Current Baseline

- **Current version**: `v0.8.0` — Production-Ready Deep Intelligence local implementation (committed & pushed)
- **Next target**: M1 v0.8.1–v0.8.3 (Native Runner + Signing) → M2 v0.9.0 (burn DL + Extended Storage + i18n) → M3 v0.10.0 (Cloud Sync + Web Dashboard) → M4 v1.0.0
- **Full plans**: `docs/v0.8.0-plan.md` (M1 details) + `docs/v1.0.0-plan.md` (M1–M4 master plan, 4 milestones, 14 feature versions)
- **Status**: v0.8.0 local implementation complete and pushed. v0.8.1-v0.8.2 local readiness is complete; SignPath approval/secrets and a GitHub Actions `ubuntu-latest` run remain external/native gates. M2 (burn DL development) can proceed in parallel.
- **Last verified**: (2026-06-05) `cargo test --manifest-path src-tauri\Cargo.toml` (129/129), `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`, `npm run typecheck`, `npm run build:web`, `cargo bench --manifest-path src-tauri\Cargo.toml`, `npm run verify:m1-release`, `npm run verify:signing`, `npm run verify:linux-ci`.

## What Works Right Now

| Component | File(s) | Verified |
|-----------|---------|----------|
| Tauri 2 scaffold | `src-tauri/tauri.conf.json` | 鉁?|
| Scanner (parallel walkdir + rayon) | `src-tauri/src/scanner/mod.rs` | 鉁?7 tests |
| Risk engine (16 rules) | `src-tauri/src/risk/mod.rs` | 鉁?6 tests |
| Cleanup engine (Recycle Bin) | `src-tauri/src/cleaner/mod.rs` | 鉁?14 tests |
| FS watcher (native Windows + polling fallback) | `src-tauri/src/platform/windows.rs`, `src-tauri/src/watcher/mod.rs` | ✅ 8 tests |
| SQLite database | `src-tauri/src/db/mod.rs` | 鉁?8 tests |
| Tauri IPC (34 commands) | `src-tauri/src/lib.rs` | ✅ registered + 3 watcher-cache tests |
| Signing configuration | `.signpath/`, `.github/workflows/ci.yml`, `docs/signing.md`, `packaging/homebrew/diskpulse.rb` | ✅ `npm run verify:signing` |
| Linux native CI configuration | `.github/workflows/ci.yml`, `src-tauri/src/platform/linux.rs`, `docs/linux-ci.md` | ✅ `npm run verify:linux-ci` |
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
| Multi-device Dashboard | `src-tauri/src/hub/`, `src/hooks/useRemoteDevice.ts`, `src/App.tsx` | ✅ 10 hub tests |
| Large file finder backend | `src-tauri/src/scanner/mod.rs`, `src-tauri/src/lib.rs` | 鉁?3 tests |
| Auto-cleanup backend | `src-tauri/src/scheduler/mod.rs`, `src-tauri/src/db/mod.rs` | 鉁?5 tests |
| Auto-cleanup frontend | `src/components/AutoCleanupStatus.tsx`, `src/pages/Settings/index.tsx`, `src/pages/History/index.tsx` | 鉁?|
| TypeScript types | `src/types.ts` | 鉁?|
| Aurora design system | `src/index.css` | ✅ |
| Streaming scan (v0.6.1) | `src-tauri/src/scanner/mod.rs` | ✅ 11 tests |
| Custom rule editor + tester (v0.6.2) | `src-tauri/src/risk/mod.rs`, `src/components/RuleEditor.tsx`, `src/components/RuleTester.tsx` | ✅ 7 tests |
| Windows Service mode (v0.6.4) | `src-tauri/src/service/mod.rs` | ✅ 4 tests |
| ML anomaly detection (v0.6.5) | `src-tauri/src/anomaly/mod.rs` | ✅ 5 tests |
| Holt-Winters prediction (v0.6.5) | `src-tauri/src/prediction/mod.rs` | ✅ 6 tests |
| Smart recommendations v2 (v0.6.6) | `src-tauri/src/recommendations/mod.rs` | ✅ 6 tests |
| Disk health radar (v0.6.6) | `src/components/DiskHealthRadar.tsx` | ✅ |
| Anomaly card (v0.6.5) | `src/components/AnomalyCard.tsx` | ✅ |
| CLI service flag (v0.6.4) | `src-tauri/src/cli/mod.rs` | ✅ |

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


## v0.7.0 Release Artifacts

- MSI: `src-tauri/target/release/bundle/msi/DiskPulse_0.7.0_x64_en-US.msi`
- MSI SHA256: `49C6B8ED4C17644FCD5DF811DFD6BDD91C21B799FC8B89E97BCD6445E30AC814`
- NSIS: `src-tauri/target/release/bundle/nsis/DiskPulse_0.7.0_x64-setup.exe`
- NSIS SHA256: `CDE2529CFACD7D49D77F83C9615EC02C4DE138B36C00FED0B4A938FA79820411`
- CLI smoke: `cargo run -- --cli health C --json` returned health JSON in < 10s including dev build overhead.
- All 119 tests passing; 0 clippy warnings; MSI + NSIS Windows installers generated.

## v0.5.0 Release Artifacts

- MSI: `src-tauri/target/release/bundle/msi/DiskPulse_0.5.0_x64_en-US.msi`
- MSI SHA256: `7F3193F32EC59A4394F4ED5F355C55CBB924DE1E320AA5D210E4CF4EED55CD83`
- NSIS: `src-tauri/target/release/bundle/nsis/DiskPulse_0.5.0_x64-setup.exe`
- NSIS SHA256: `F1DCBFCA5BF3670DC6B662B42B4A54E98CBC9B37105065EC628DDC0CC2AFAAAB`
- CLI smoke: `cargo run -- --cli clean C --dry-run --json` returned one safe candidate and performed no deletion.

## v0.4.0 Release Artifacts

- MSI: `src-tauri/target/release/bundle/msi/DiskPulse_0.4.0_x64_en-US.msi`
- MSI SHA256: `3AAB14EC84C7794734BB9FD3E341A2F75F58E408DB1761E0E3E6552B6D1CC184`
- NSIS: `src-tauri/target/release/bundle/nsis/DiskPulse_0.4.0_x64-setup.exe`
- NSIS SHA256: `62BCE631815A70646359991F4FBD29B5FF7472D374F96950C74E3396F39D1C8C`
- CLI smoke: `cargo run -- --cli health C --json` returned health JSON in 8326 ms including dev build overhead.

## v0.5.0 Roadmap — Integration Excellence & Platform Maturity

> Full plan: `docs/v0.5.0-plan.md` | Implementation tasks: `CODEX.md` § "v0.5.0 Implementation Tasks"

### Theme

v0.4.0 built all the pieces. v0.5.0 makes them actually work together — wiring cross-module data flows, completing UI shells, enabling full CLI, and making scoring configurable.

### Phase 1: Cross-Module Integration (v0.4.1 — v0.4.3)

| Version | Focus | Key Deliverables | Codex Task | Status |
|---------|-------|-----------------|------------|--------|
| v0.4.1 | Integration — Data Flow | Wire aging→recommendations (`age_days`), wire duplicates/zombie→disk health | A, B | ✅ Complete |
| v0.4.2 | Integration — CLI | Fix export drive arg, enable `clean` subcommand, add `--dry-run` | C, G | ✅ Complete |
| v0.4.3 | Integration — Config | 7 new `AppSettings` fields for scoring weights + thresholds, Settings UI | D | ✅ Complete |

### Phase 2: UI Completion (v0.4.4 — v0.4.6)

| Version | Focus | Key Deliverables | Codex Task | Status |
|---------|-------|-----------------|------------|--------|
| v0.4.4 | UI — CleanupWizard | 5-step guided flow: Select→Scan→Review→Confirm→Execute | E | ✅ Complete |
| v0.4.5 | UI — Notifications | Real-time polling, event persistence, unread badge, per-item dismiss | F | ✅ Complete |
| v0.4.6 | UI — Polish | `--quiet` CLI, i18n error coverage, edge cases | — | ✅ Complete |

### Phase 3: Release (v0.5.0)

| Task | Codex Task | Status |
|------|------------|--------|
| Performance benchmarks (6 metrics) | H | ✅ Complete (synthetic bench added) |
| Integration tests + docs sync | I | ✅ Complete |
| Build verification + installers | I | ✅ Complete |
| Version bump to 0.5.0 | I | ✅ Complete |

### Known Issues — Resolved in v0.5.0

> These issues were identified in the v0.4.0 post-release audit. Resolution plan below.

| # | Issue | Module(s) | Impact | Resolution | Codex Task | Priority |
|---|-------|-----------|--------|------------|------------|----------|
| 1 | `RecommendationInput.age_days` always `None` | `recommendations/mod.rs:97` | Age scoring factor always uses default weight (25%) | Wire `aging` module output into `get_recommendations()` | A | 🔴 P1 |
| 2 | `get_disk_health()` passes `0, 0` for waste/zombie | `recommendations/mod.rs:85` | Disk health score ignores actual duplicate waste and zombie data | Call `scan_duplicates()` + `analyze_file_aging()` inside `get_disk_health()` | B | 🔴 P1 |
| 3 | CLI `export` hardcodes `"C"` drive | `cli/mod.rs:79-81` | Export commands ignore user-selected drive | Add `drive` field to `CliCommand::Export` | C | 🟡 P2 |
| 4 | Scoring weights are magic numbers | `recommendations/mod.rs`, `duplicates/mod.rs` | Hard to tune; user-configurable weights planned but not implemented | Add 7 `AppSettings` fields + Settings UI tab | D | 🟡 P2 |
| 5 | `CleanupWizard` + `NotificationCenter` are UI shells | `src/components/CleanupWizard.tsx`, `src/components/NotificationCenter.tsx` | Components exist but core logic incomplete | 5-step wizard wiring + real-time notification polling | E, F | 🟡 P2 |

**Fix plan**: Codex executes Tasks A–I in Phase order. Each task is self-contained with specific files, steps, and verification criteria in `CODEX.md`.

## v0.5.0 Implementation Summary

- Tasks A-I implemented: aging-aware recommendations, duplicate/zombie health scoring, CLI export/clean, configurable scoring settings, completed wizard and notification center, synthetic benchmarks, integration test, and docs/version sync.
- Added settings: five scoring weights, duplicate minimum size, and aging zombie threshold.
- Added notification commands: `mark_notification_read(id)` and `clear_notifications()`.
- Added benchmark command: `cd src-tauri && cargo bench --bench performance`.
- Latest verified during implementation: `cargo test` 81/81 passed, `npm run typecheck` passed, synthetic bench ran.

## v0.6.0 Roadmap — Cross-Platform Performance Foundation

> Full plan: `docs/v0.6.0-plan.md`

### Theme

v0.6.0 makes DiskPulse fast and everywhere — native kernel FS events, hard-link-aware dedup, sparse-file detection, and first-class Linux + macOS support through a unified 6-trait platform architecture.

### Phase 1: Platform Trait Foundation (v0.5.1 — v0.5.3)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.5.1 | Traits — Definition | 6 platform traits defined (`DiskInfoProvider`, `FsWatcher`, `DirScanner`, `CleanupProvider`, `FileMetaAnalyzer`, `SystemInfo`) + common types | ✅ Complete |
| v0.5.2 | Traits — Wiring | All business logic routes through traits via `platform::providers()` dispatch point | ✅ Complete |
| v0.5.3 | Traits — Windows Preserve | Extract current Windows impls into trait framework as baseline | ✅ Complete |

### Phase 2: Windows Native Performance (v0.5.4 — v0.5.6)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.5.4 | Win — Native Watcher | `ReadDirectoryChangesW` replaces polling; polling fallback retained | ✅ Complete |
| v0.5.5 | Win — Hard Link Dedup | `GetFileInformationByHandle` → skip hard-linked duplicates in dedup pipeline | ✅ Complete |
| v0.5.6 | Win — Sparse File Detection | `FILE_ATTRIBUTE_SPARSE_FILE` + `GetCompressedFileSizeW` → size-on-disk vs apparent | ✅ Complete |

### Phase 3: Linux + macOS (v0.5.7 — v0.5.9)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.5.7 | Linux | All 6 traits: df/proc disk info, inotify watcher, gio trash, walkdir/jwalk scan, Unix metadata identity | ✅ Complete (CI-ready) |
| v0.5.8 | macOS | All 6 traits: df disk info, polling watcher, Finder Trash, walkdir/jwalk scan, Unix metadata identity, sysctl RAM | ✅ Complete (CI-ready) |
| v0.5.9 | CI/CD + Packaging | GitHub Actions 3-platform matrix; .deb, .AppImage, .dmg artifact upload | ✅ Complete (CI-ready) |

### Phase 4: Release (v0.6.0)

| Task | Status |
|------|--------|
| `cargo test` — 86 tests, Windows verified; Linux/macOS CI-ready | ✅ Complete |
| Windows: MSI + NSIS generated; Linux/macOS: CI-ready | ✅ Complete |
| CLI smoke: Windows verified; Linux/macOS CI-ready | ✅ Complete |
| Docs sync: CLAUDE.md, PROGRESS.md, CHANGELOG.md, CODEX.md, README | ✅ Complete |

### Technical Reserve (Compiled, Not Wired)

| Item | Activation Condition |
|------|---------------------|
| `MftStage` (NTFS MFT direct scan) | `ntfs-rs` crate mature → feature flag `mft-scanner` |
| `ReadDirectoryChangesW` buffer overflow recovery | Detected in production → auto full-refresh |

### Platform Trait Matrix

| # | Trait | Windows | Linux | macOS |
|---|-------|---------|-------|-------|
| 1 | `DiskInfoProvider` | `GetDiskFreeSpaceExW` | `statvfs` | `statfs` |
| 2 | `FsWatcher` | `ReadDirectoryChangesW` | `inotify` | `FSEvents` |
| 3 | `DirScanner` | `JwalkStage` (default) | `LinuxWalkStage` | `MacOsWalkStage` |
| 4 | `CleanupProvider` | `SHFileOperationW` | `trash-rs` | `Trash` |
| 5 | `FileMetaAnalyzer` | `GetFileInformationByHandle` | `statx` | `stat` |
| 6 | `SystemInfo` | `GetSystemInfo` | `uname + /proc` | `sysctl` |

## v0.6.0 Implementation Notes

- Added `src-tauri/src/platform/common.rs`, `src-tauri/src/platform/windows.rs`, `src-tauri/src/platform/linux.rs`, and `src-tauri/src/platform/macos.rs`.
- Business logic now uses `platform::providers()` for drive listing, watcher start, directory measurement, and trash movement.
- Added new IPC commands: `get_system_info` and `get_file_meta`.
- Added `FileEntry.hard_link_count` and `FileEntry.size_on_disk_bytes`, plus frontend sparse/hard-link display.
- Added hard-link-aware duplicate detection and regression test.
- Added native Windows `ReadDirectoryChangesW` watcher with polling fallback and tests for stop/drop plus create-event delivery.
- Added Windows sparse-file regression test using `FSCTL_SET_SPARSE`.
- Added Linux inotify watcher and Unix metadata identity/allocated-size reporting; added macOS Unix metadata identity/allocated-size and `sysctl` RAM reporting.
- Added `.github/workflows/ci.yml` for Windows/Linux/macOS matrix builds.
- Remaining v0.6 blockers: GitHub Actions must validate Linux/macOS builds natively; local Windows-to-Linux cross-check is blocked by GTK/pkg-config sysroot requirements.

## v0.7.0 Roadmap — Intelligent Operations Platform

> Full plan: `docs/v0.7.0-plan.md` | Implementation tasks: `CODEX.md` § "v0.7.0 Implementation Tasks"

### Theme

v0.6.0 made DiskPulse fast and everywhere. v0.7.0 makes it **smart** — from "see disk data" to "understand disk data and know what to do about it."

**Path**: Foundation polish → Deep performance → Intelligence → Ecosystem

### Phase 1: Foundation Polish (v0.6.1 — v0.6.2)

> Close v0.3.x technical debt. Streaming scan is the data pipeline foundation for all later intelligence.

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.6.1 | Streaming Scan | ScanStage `execute_streaming()`, incremental rescan, first result <500ms, memory <50MB | ✅ Complete |
| v0.6.2 | Custom Rules UI | RuleEditor + RuleTester components, `test_rule_pattern` IPC, safety constraint (LOW/MEDIUM only) | ✅ Complete |

### Phase 2: Deep Performance (v0.6.3 — v0.6.4)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.6.3 | MFT Direct Scan | FSCTL_ENUM_USN_DATA, admin privilege detection, MFT vs Jwalk strategy selector, MftStage activation | ✅ Complete |
| v0.6.4 | Windows Service | `diskpulse.exe --service`, Named Pipe IPC, SCM integration, LOCAL SERVICE account, auto-start | ✅ Complete |

### Phase 3: Intelligence Layer (v0.6.5 — v0.6.6)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.6.5 | ML Anomaly Detection | Holt-Winters seasonal forecasting, Modified Z-Score detector, 4 anomaly types, `anomaly` module | ✅ Complete |
| v0.6.6 | Smart Recommendations v2 | Urgency multiplier, user behavior learning, cross-module correlation, 4D health radar chart | ✅ Complete |

### Phase 4: Ecosystem (v0.6.7)

| Version | Focus | Key Deliverables | Status |
|---------|-------|-----------------|--------|
| v0.6.7 | Multi-Device Dashboard | WebSocket Hub, mDNS discovery, 6-digit pairing, remote monitoring, `hub/` module | ✅ Complete |

### Phase 5: Release (v0.7.0)

| Task | Status |
|------|--------|
| Integration tests (5 pipelines: scan/intelligence/cleanup/service/hub) | ✅ Covered by 119-test suite |
| Target: 115+ tests (86 → 115, +29 new) | ✅ 119 tests |
| Docs sync: CLAUDE.md, PROGRESS.md, CHANGELOG.md, CODEX.md, README | ✅ Updated |
| Version bump to 0.7.0 (Cargo.toml, package.json, tauri.conf.json) | ✅ Complete |
| Build verification: Windows (MSI/NSIS) + Linux (.deb/.AppImage) + macOS (.dmg) | ✅ Windows MSI/NSIS generated; Linux/macOS CI-ready |
| GitHub release tag v0.7.0 + release notes | ✅ Release notes in `docs/release-notes-v0.7.0.md` |

### New Modules (v0.7.0)

| Module | Phase | Purpose |
|--------|-------|---------|
| `anomaly/mod.rs` | v0.6.5 | Holt-Winters + Modified Z-Score anomaly detection |
| `service/mod.rs` | v0.6.4 | Windows Service lifecycle management |
| `hub/` (6 files) | v0.6.7 | WebSocket server, device registry, message routing, pairing, mDNS discovery |

### New IPC Commands (v0.7.0)

| Command | Phase |
|---------|-------|
| `test_rule_pattern(pattern, test_path) -> bool` | v0.6.2 |
| `install_service() -> Result<(), String>` | v0.6.4 |
| `uninstall_service() -> Result<(), String>` | v0.6.4 |
| `get_service_status() -> Result<ServiceStatus, String>` | v0.6.4 |
| `detect_anomalies(drive) -> Result<AnomalyReport, String>` | v0.6.5 |
| `start_hub(port) -> Result<(), String>` | v0.6.7 |
| `stop_hub() -> Result<(), String>` | v0.6.7 |
| `get_connected_devices() -> Vec<DeviceInfo>` | v0.6.7 |
| `get_hub_discovery_info() -> Option<DiscoveryInfo>` | v0.6.7 |
| `discover_devices(timeout_ms) -> Result<Vec<DeviceInfo>, String>` | v0.6.7 |
| `create_pairing_token(device_name, ttl_seconds) -> Result<PairingToken, String>` | v0.6.7 |
| `pair_device(token) -> Result<DeviceInfo, String>` | v0.6.7 |
| `unpair_device(device_id) -> Result<(), String>` | v0.6.7 |

### New IPC Events (v0.7.0)

| Event | Phase | Payload |
|-------|-------|---------|
| `scan-batch` | v0.6.1 | `ScanBatch` — streaming scan incremental batch |
| `anomaly-detected` | v0.6.5 | `AnomalyEvent` — real-time anomaly alert |
| `device-connected` | v0.6.7 | `DeviceInfo` — new device online |
| `device-disconnected` | v0.6.7 | `{ device_id }` — device offline |
| `remote-alert` | v0.6.7 | `{ device_id, alert_payload }` — remote device alert |

### New Frontend Components (v0.7.0)

| Component | Phase | Purpose |
|-----------|-------|---------|
| `RuleEditor.tsx` | v0.6.2 | Custom rule name/pattern/level editor |
| `RuleTester.tsx` | v0.6.2 | Real-time rule pattern tester |
| `AnomalyCard.tsx` | v0.6.5 | Dashboard anomaly summary card |
| `useRemoteDevice.ts` (hook) | v0.6.7 | Remote device data query via WS |

## v0.8.0 Roadmap — Production-Ready Deep Intelligence

> Full plan: `docs/v0.8.0-plan.md` | Implementation tasks: `CODEX.md` § "v0.8.0 Implementation Tasks"

### Theme

v0.7.0 made DiskPulse smart. v0.8.0 makes it **shippable to the public** while pushing intelligence to **deep-learning level** — two independent tracks that don't block each other.

### Phase 1: Production-Ready (v0.7.1 — v0.7.5)

| Version | Focus | Key Deliverables | Priority |
|---------|-------|-----------------|----------|
| v0.7.1 | Code Signing + Notarization | SignPath config + CI signing hook + Homebrew Cask template; external approval/secrets pending | ✅ Local |
| v0.7.2 | Linux Native CI | GitHub Actions ubuntu-latest: cargo test + .deb/.AppImage packaging; local config complete, native runner pending | ✅ Local |
| v0.7.3 | macOS Native CI + FSEvents | GitHub Actions macos-latest + FSEvents native watcher activation; native runner pending | ✅ Local |
| v0.7.4 | Code Split + Auto-Update | React.lazy route splitting (first screen <300KB gzip) + GitHub Release update checker | ✅ Local |
| v0.7.5 | Perf Bench + i18n + Edge Fixes | 10 benchmarks (synthetic fixtures), Japanese locale, edge-case fixes | ✅ Local |

### Phase 2: Deep Intelligence (v0.7.6 — v0.7.10)

| Version | Focus | Key Deliverables | Priority |
|---------|-------|-----------------|----------|
| v0.7.6 | Disk Fragmentation Analysis | FSCTL_GET_RETRIEVAL_POINTERS / FS_IOC_FIEMAP / F_LOG2PHYS extent detection; analysis only, no defrag | P0 |
| v0.7.7 | Anomaly Fusion Fallback | Holt-Winters + Modified Z-Score + optional AE signal fusion; runtime fallback weights | ✅ Local |
| v0.7.8 | Health Score v2 | 4D→6D scoring (Space/Waste/Trend/Age/Frag/Anomaly) + health snapshots | ✅ Local |
| v0.7.9 | Predictive Cleanup | Disk-full prediction, cleanup gain simulation, pre-cleanup candidates with confirmation guard | ✅ Local |
| v0.7.10 | Smart File Classification | Extension + magic-byte classification, `file_category` in file entries | ✅ Local |

### v0.8.0 Release

| Task | Status |
|------|--------|
| Target: expanded tests | ✅ 129 Rust tests |
| 10 feature versions (v0.7.1–v0.7.10) | ✅ Local |
| 2 new burn models (AE ~50KB + Classifier ~80KB) | ⏭️ Deferred to v0.9.0 |
| 3-platform signed CI all green | ⚠️ Native runners pending |
| Docs sync: CLAUDE.md, PROGRESS.md, CHANGELOG.md, CODEX.md, README | ✅ Local |

### New Modules (v0.8.0)

| Module | Phase | Purpose |
|--------|-------|---------|
| `fileclass/` | v0.7.10 | Extension + magic-byte classification |
| `fragmentation.rs` | v0.7.6 | Sampled fragmentation reports and file estimates |
| `anomaly` fusion types | v0.7.7 | Runtime fallback/fusion weighting |

### New Crate Deps (v0.8.0)

| Crate | Phase | Purpose |
|-------|-------|---------|
| None | — | Burn model packaging deferred to v0.9.0; v0.8.0 keeps statistical fallback path |

### Deferred to v0.9.0

| Item | Rationale |
|------|-----------|
| Cloud Sync Bridge | Auth + encryption + cloud infra needed first |
| Mobile Companion App | React Native / Tauri Mobile, architecture decision pending |
| External Storage Auto-Detection | Extra platform adaptation work |
| Web Dashboard (browser access) | Needs Cloud Sync foundation |
| More languages (ko/es/etc.) | Extend after v0.8.0 i18n framework validation |

## v0.8.0→v1.0.0 Master Roadmap — Public Release Journey

> Full plan: `docs/v1.0.0-plan.md` | 4 milestones, 14 feature versions

### Theme

v0.8.0 made DiskPulse production-ready with deep intelligence. The v1.0.0 journey makes it **publicly shippable** — signed, cross-platform, with burn deep learning, cloud sync, and a Web Dashboard.

**Key decisions**: A+B fusion (Public Release + Full Features), burn DL complete (AE + Classifier), progressive Cloud Sync (relay → accounts), interactive Web Dashboard (embedded HTTP + shared React codebase), milestone-driven versioning.

### M1: v0.8.1–v0.8.3 — Production Verification (target: mid-June 2026)

> Can run in PARALLEL with M2. M1 is ops/verification; M2 is dev/feature.

| Version | Focus | Key Deliverables | Priority |
|---------|-------|-----------------|----------|
| v0.8.1 | SignPath Approval + Windows Signing | OSS application submission, GitHub Secrets, CI signing webhook test, signed MSI/NSIS | ✅ Local-ready / ⏳ External |
| v0.8.2 | Linux Native Runner | ubuntu-latest: cargo test + inotify FFI verification + .deb/.AppImage packaging | ✅ Local-ready / ⏳ Native |
| v0.8.3 | macOS Native Runner + FSEvents | macos-latest: FSEvents activation + .dmg packaging + trash-rs verification | P0 |

**M1 completion**: 3-platform CI all green + Windows signed artifacts → "minimum shippable version"

**Local readiness note**: `docs/m1-release-readiness.md` captures the v0.8.1-v0.8.2 local gate. The current local checks cover SignPath workflow shape, signed artifact verification, Ubuntu dependency setup, and `.deb`/`.AppImage` bundle assertions; they do not replace external SignPath approval or a real Linux native runner result.

### M2: v0.8.4–v0.8.8 → v0.9.0 — Full Intelligence (target: late June 2026)

| Version | Focus | Key Deliverables | Priority |
|---------|-------|-----------------|----------|
| v0.8.4 | burn AE Anomaly Detection | burn Autoencoder (6→4→6), synthetic training pipeline, 3-way signal fusion, 8 risk mitigations | P0 |
| v0.8.5 | burn File Classifier Stage 3 | burn 12-class softmax (8→32→16→12), 5000+ training samples, file_category risk rules | P0 |
| v0.8.6 | Extended Storage | external drive hot-plug detection (WM_DEVICECHANGE/udev/IOKit), new storage-attached events | P1 |
| v0.8.7 | i18n Expansion | Korean (ko) + Spanish (es) locales — total 5 languages (en/zh-CN/ja/ko/es) | P1 |
| v0.8.8 | Model Fine-tune UI | Settings → AI Model panel, AUC metrics, user fine-tune trigger (>60 snapshots), model reset | P2 |

**M2 completion**: 152+ tests, burn AE AUC > 0.85, classifier accuracy > 85%, 5 languages, external storage hot-plug

### M3: v0.9.1–v0.9.3 → v0.10.0 — Ecosystem Connection (target: early July 2026)

| Version | Focus | Key Deliverables | Priority |
|---------|-------|-----------------|----------|
| v0.9.1 | Relay Server | Self-hosted relay (Rust binary, systemd/Docker), public community relay (wss://relay.diskpulse.dev), E2E encryption | P0 |
| v0.9.2 | Cloud Sync Bridge | WAN device pairing via relay, existing Hub protocol reuse, 4 new IPC commands | P0 |
| v0.9.3 | Web Dashboard | Embedded HTTP server (axum), shared React codebase dual-build (Tauri/Web), interactive mode with cleanup confirmation guard | P0 |

**M3 completion**: 167+ tests, two devices paired across internet, Web Dashboard at localhost:PORT

### M4: v1.0.0 — Public Release (target: mid-July 2026)

| Task | Status |
|------|--------|
| 5 integration test pipelines (scan/intelligence/cleanup/cloud/web) | ⏳ |
| 10 performance benchmarks on real hardware (3 platforms) | ⏳ |
| Target: 180+ tests | ⏳ |
| Windows (SignPath signed MSI/NSIS) + Linux (.deb/.AppImage/Snap) + macOS (.dmg/Homebrew) | ⏳ |
| Docs sync: all MD files | ⏳ |
| GitHub Release tag v1.0.0 + release notes | ⏳ |

### Risk Matrix (all 9 risks mitigated)

| # | Risk | Level | Mitigation |
|---|------|-------|------------|
| R1 | SignPath delay | 🔴 | Sigstore fallback; M1/M2 parallel |
| R2 | burn compile failure | 🟡 | feature-gate `ml-engine`, statistical fallback |
| R3 | AE accuracy < 0.7 | 🟡 | dynamic weight degradation, auto-disable |
| R4 | Relay ops burden | 🟡 | self-hosted design, LAN mode zero-dependency |
| R5 | Web Dashboard scope | 🟡 | shared codebase, degradable to read-only |
| R6 | Native CI bugs | 🟡 | polling fallback, fix-then-release |
| R7 | 14 versions too long | 🟢 | P0 priority, P1/P2 deferrable |
| R8 | FS extent APIs denied | 🟢 | sampling fallback, unreadable marker |
| R9 | macOS FSEvents issues | 🟡 | polling fallback retained |

### Deferred to v1.1+

| Item | Rationale |
|------|-----------|
| Tauri Mobile Companion App | Architecture decision + development scope |
| Plugin Marketplace | Stable API needed first |
| OAuth Account System | Pairing-code model sufficient for v1.0 |
| More languages (fr/de/pt) | 5 languages cover primary user base |
| Cloud Backup | Relay server does not decrypt data |

### New Crate Deps (M2)

| Crate | Version | Purpose | Feature-gate |
|-------|---------|---------|-------------|
| `burn` | 0.16 | DL framework (Autoencoder + Classifier) | `ml-engine` |
| `burn-ndarray` | 0.16 | CPU backend | `ml-engine` |

### New Modules (M2–M3)

| Module | Milestone | Purpose |
|--------|-----------|---------|
| `anomaly/ae.rs` | M2 | burn AE model + training + inference |
| `anomaly/features.rs` | M2 | 6-dim snapshot feature extraction |
| `anomaly/synthetic.rs` | M2 | synthetic training data generator |
| `fileclass/model.rs` | M2 | burn classifier + inference |
| `fileclass/features.rs` | M2 | 8-dim file feature extraction |
| `storage/mod.rs` | M2 | external storage hot-plug detection |
| `relay/mod.rs` | M3 | relay client (connect/auth/route) |
| `web/mod.rs` | M3 | embedded HTTP server for Web Dashboard |

### Target Test Counts

| Milestone | Test Count |
|-----------|-----------|
| v0.8.0 (baseline) | 129 |
| M1 (v0.8.1–0.8.3) | 130+ |
| M2 (v0.9.0) | 152+ |
| M3 (v0.10.0) | 167+ |
| M4 (v1.0.0) | 180+ |

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
| v0.5.0 | Integration Excellence | ✅ | Cross-module data flow, CLI completion, configurable weights, wizard + notifications |
| v0.6.0 | Cross-Platform Perf Foundation | ✅ | 6-trait platform, native watchers, hard-link dedup, sparse files, CI matrix |
| v0.6.1 | Streaming Scan | ✅ | ScanStage `execute_streaming()`, incremental rescan |
| v0.6.2 | Custom Rules UI | ✅ | RuleEditor + RuleTester, `test_rule_pattern` |
| v0.6.3 | MFT Direct Scan | ✅ | FSCTL_ENUM_USN_DATA, MftStage activation |
| v0.6.4 | Windows Service | ✅ | `--service` mode, Named Pipe IPC, SCM |
| v0.6.5 | ML Anomaly Detection | ✅ | Holt-Winters + Modified Z-Score, `anomaly` module |
| v0.6.6 | Smart Recommendations v2 | ✅ | Urgency multiplier, behavior learning, health radar |
| v0.6.7 | Multi-Device Dashboard | ✅ | WebSocket Hub, mDNS, pairing, remote device selector |
| v0.7.0 | Intelligent Ops Platform | ✅ Complete | 119 tests, Windows MSI/NSIS generated, docs synced |
| v0.7.1 | Code Signing Foundation | ✅ Local | SignPath config, CI signing hook, Homebrew Cask template, signing docs |
| v0.7.2 | Linux Native CI | ✅ Local | ubuntu-latest deps, .deb/.AppImage verification, trash-rs fallback, inotify parser test, Linux CI docs |
| v0.8.0 | Production-Ready Deep Intelligence | ✅ Local | 129 tests, fragmentation, anomaly fusion fallback, 6D health, predictive cleanup, file classification |

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



