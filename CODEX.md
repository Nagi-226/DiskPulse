# CODEX.md - DiskPulse Agent Operating Manual

This file is the first-stop memory for Codex when working in this repository. Its goal is to keep project context, safety constraints, and verification habits aligned before any code change.

## Role

- Act as the implementation agent for DiskPulse.
- Prefer concise Chinese responses when the user writes in Chinese.
- Make code changes only after reading the relevant local files; do not rely only on this document.
- Treat safety and data-loss prevention as higher priority than feature speed.

## Startup Protocol

Run these checks mentally, and with commands when needed, at the start of a task:

1. Read `PROGRESS.md` for the latest version/status snapshot.
2. Read `CLAUDE.md` for architecture, IPC, safety rules, and release context.
3. Check actual source-of-truth versions in:
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
4. Inspect `git status --short` before editing. Do not overwrite unrelated user changes.
5. Use `rg --files` and `rg` for project search.
6. Open the exact modules you will modify and match existing style.

If documentation conflicts with source code, trust the code and note the mismatch.

## Current Baseline

- Product: DiskPulse, a Windows 11 desktop app for disk monitoring and safe cleanup.
- Current release baseline: `v0.8.0` local Production-Ready Deep Intelligence complete (fragmentation analysis, anomaly fusion fallback, 6D health, predictive cleanup, file classification; native runner validation pending).
- Next milestone: `v0.9.0` follow-ups / native runner validation.
- Full v0.8.0 roadmap: `docs/v0.8.0-plan.md`.
- Stack: Tauri 2, Rust 1.94+, React 19, TypeScript 5, Tailwind CSS 4, SQLite via rusqlite, burn 0.16 (DL engine).
- Build targets: Windows (MSI/NSIS, SignPath signed), Linux (.deb/.AppImage), macOS (.dmg, Homebrew Cask).
- Current state from project docs: v0.8.0 implemented locally; native runner validation and true platform extent counters remain follow-ups.

### v0.5.0 Implementation Tasks — Integration Excellence

> Priority: Complete all tasks in Phase order before any new feature work.
> Full roadmap: `docs/v0.5.0-plan.md`

#### Phase 1: Cross-Module Integration (🔴 High Priority)

**Task A — Wire aging data into recommendation engine**

Goal: `RecommendationInput.age_days` must carry real data from the aging module instead of `None`.

Files: `src-tauri/src/recommendations/mod.rs`, `src-tauri/src/aging/mod.rs`

Steps:
1. Read `aging/mod.rs` to understand `AgingReport` structure and `analyze_file_aging()` signature.
2. In `get_recommendations()`, call `crate::aging::analyze_file_aging()` to obtain per-file age data.
3. Build a `HashMap<String, u64>` mapping normalized path → age_days from the aging report buckets.
4. Update `input_from_risk_item()` to accept the age map and populate `age_days`.
5. Update `score_recommendation()` — `age_days.unwrap_or(25.0)` already handles `None`; after wiring, real age data flows through.
6. Add 1+ test: recommendation with real-looking age_days scores higher than one without.

Verify: `cargo test` (all existing + new), `cargo clippy -- -D warnings`

---

**Task B — Wire duplicate/zombie data into disk health**

Goal: `get_disk_health()` must use actual duplicate waste bytes and zombie bytes instead of hardcoded zeros.

Files: `src-tauri/src/recommendations/mod.rs`

Steps:
1. Read `duplicates/mod.rs` for `scan_duplicates_with_progress_and_cancel()` signature and `DuplicateGroup` structure.
2. Read `aging/mod.rs` for `AgingReport` and zombie detection fields.
3. In `get_disk_health()`, call duplicate scan and aging analysis.
4. Sum `total_size_wasted` from all `DuplicateGroup` results → `duplicate_waste_bytes`.
5. Extract `zombies_total_size` from `AgingReport` → `zombie_bytes`.
6. Pass real values to `calculate_disk_health()`.
7. **IMPORTANT**: These scans are expensive. Design decision:
   - Option A (recommended): Accept latency; document that `get_disk_health` is a "full health check" command.
   - Option B: Run scans in background thread, return cached results. More complex.
   - Choose Option A for v0.5.0; add `AppSettings.health_check_cache_seconds` if needed later.
8. Add 1+ test: disk health with simulated waste data shows lower score.

Verify: `cargo test`, `cargo clippy -- -D warnings`

---

**Task C — Fix CLI export drive argument**

Goal: CLI `export` subcommand must accept a user-specified drive letter instead of hardcoding `"C"`.

Files: `src-tauri/src/cli/mod.rs`

Steps:
1. Change `CliCommand::Export` variant from `{ format, report_type }` to `{ drive, format, report_type }`.
2. Update `parse_cli_args()` — Export now takes 3 positional args: `<drive> <format> <type>`.
3. Update `execute_cli_command()` — use `drive` from the variant instead of `"C"`.
4. Update existing CLI tests to include drive argument in Export variant.

Verify: `cargo test cli`, `cargo clippy`

---

#### Phase 2: Configuration & UI Completion (🟡 Medium Priority)

**Task D — Configurable scoring weights**

Goal: Scoring weights and `min_size` constants must be user-configurable via Settings.

Files: `src-tauri/src/db/mod.rs`, `src-tauri/src/recommendations/mod.rs`, `src-tauri/src/duplicates/mod.rs`, `src/pages/Settings/index.tsx`, `src/types.ts`

Steps:
1. Add 5 new `f64` fields to `AppSettings` struct in `db/mod.rs`: `scoring_weight_risk`, `scoring_weight_age`, `scoring_weight_duplicate`, `scoring_weight_size`, `scoring_weight_safety`.
2. Add 2 new `u64` fields: `duplicate_min_size_bytes` (default 1MB), `aging_zombie_days` (default 180).
3. Add DB migration: `ALTER TABLE settings ADD COLUMN ...` with defaults.
4. Update `ScoringWeights::default()` to read from settings; fallback to hardcoded defaults if settings missing.
5. Update `scan_duplicates` command default min_size to read from settings.
6. Update `aging` zombie detection threshold to read from settings.
7. Frontend: Add "Recommendations" tab in Settings with 5 weight sliders + 2 numeric inputs.
8. Add `AppSettings` TypeScript type updates in `src/types.ts`.

Verify: `cargo test`, `cargo clippy`, `npm run typecheck`, manual settings UI check

---

**Task E — Complete CleanupWizard 5-step flow**

Goal: Transform the UI shell into a fully functional guided cleanup experience.

Files: `src/components/CleanupWizard.tsx`, `src/hooks/useDriveScan.ts`

Steps:
1. **Step 1 (Select Drive)**: Already works — keep existing drive selector + "Scan" button.
2. **Step 2 (Scanning)**: Integrate `useDriveScan` hook — show real-time progress bar with phase indicator (Walking/Measuring/Complete). Auto-advance to step 3 on completion.
3. **Step 3 (Review Results)**: Call `classify_risks` + `get_recommendations`. Show risk-grouped summary with space breakdown, top recommendations, and disk health gauge. Reuse existing `RecommendationCard` component patterns.
4. **Step 4 (Confirm Cleanup)**: Build checkbox list of LOW-risk safe candidates. Show estimated space to free. Mirror `CleanupPreview` selection logic — all safe items pre-checked. User can uncheck.
5. **Step 5 (Execution)**: Call `clean_items` with selected items. Show real-time progress via `clean-progress` event. Display result summary (files cleaned, space freed, errors).
6. Add "Back" and "Next" navigation between steps. Preserve state across step transitions.
7. All states handled: loading, empty (no cleanup candidates), error, partial failure.

Verify: `npm run typecheck`, `npm run build:web`, manual wizard walkthrough

---

**Task F — Complete NotificationCenter real-time polling**

Goal: Notification panel must auto-refresh and badge unread count in real-time.

Files: `src/components/NotificationCenter.tsx`, `src-tauri/src/lib.rs`

Steps:
1. Add notification persistence: In every place that emits an alert/cleanup/auto-cleanup event, also call `db::save_notification()` to persist to SQLite. Check current event emission sites in `lib.rs`.
2. Update `NotificationCenter.tsx`: Add `setInterval` polling (every 30s default). Show unread count badge on the bell icon.
3. Add notification types for: `disk-space-alert`, `cleanup-complete`, `auto-cleanup-complete`, `auto-cleanup-scheduled`.
4. Add "Clear all" and per-item dismiss functionality.
5. Add a `mark_notification_read(id)` command for per-item read state.

Verify: `cargo test`, `npm run typecheck`, `npm run build:web`

---

**Task G — Enable CLI CleanLow execution**

Goal: `diskpulse --cli clean <drive>` must execute safe LOW-risk cleanup.

Files: `src-tauri/src/cli/mod.rs`

Steps:
1. Replace the "disabled" error in `CliCommand::CleanLow` with actual execution logic.
2. Call `crate::scanner::scan_drive(drive)` → `crate::risk::classify_risks()` → filter LOW + safe_to_delete items → `crate::cleaner::clean_items_with_progress()`.
3. Add `--dry-run` flag support: preview what would be cleaned without executing.
4. Output: JSON mode prints structured result; default mode prints human-readable summary.
5. Exit codes: 0=success, 1=partial failure, 2=scan error.
6. Add 1+ test: CLI clean parsing (not execution, to avoid actual file deletion in test).

Verify: `cargo test cli`, `cargo clippy`, manual CLI smoke: `cargo run -- --cli clean C: --dry-run`

---

#### Phase 3: Verification & Documentation (🔵 Polish)

**Task H — Performance benchmarks**

Goal: Measure and document v0.5.0 performance against v0.3.3 targets.

Files: `src-tauri/Cargo.toml` (add `criterion` dev-dependency if not present)

Steps:
1. Add benchmark for full drive scan (target: < 5s for 500GB).
2. Add benchmark for duplicate detection (target: < 60s for 500GB).
3. Add benchmark for cancel response time (target: < 500ms).
4. Record results in `docs/v0.5.0-plan.md` benchmarks section.
5. If targets missed, document gap and defer optimization to v0.5.x.

Verify: `cargo bench` (or manual timing if criterion not integrated)

---

**Task I — Integration tests & docs sync**

Goal: End-to-end verification and documentation update.

Steps:
1. Write 1 integration test: scan → classify → recommend → (dry-run) clean pipeline.
2. Update `PROGRESS.md` to v0.5.0 status.
3. Update `CLAUDE.md` with any new commands/settings added during Phase 1-2.
4. Update `CHANGELOG.md` with v0.5.0 entries.
5. Update `README.md` / `README_zh-CN.md` if user-facing features changed.
6. Final verification: `cargo test && cargo clippy -- -D warnings && npm run typecheck && npm run build:web`.
7. Run `npm run tauri build` for release artifacts.

Verify: All checks green, installers generated.

---

### Task Dependency Graph

```
Phase 1:  A ──┬── B ──┐
              │        │
              C ───────┤
                       │
Phase 2:  D ── E ── F ── G
              │
              │
Phase 3:  H ── I
```

- A and C are independent (parallelizable)
- B depends on understanding from A
- D should be done before E (Wizard needs configurable weights)
- E, F, G are independent
- H and I are final phase, sequential

### Verification Matrix (per task)

| Task | cargo check | cargo test | cargo clippy | npm typecheck | npm build:web | manual |
|------|:--:|:--:|:--:|:--:|:--:|:--:|
| A | ✅ | ✅ | ✅ | — | — | — |
| B | ✅ | ✅ | ✅ | — | — | — |
| C | ✅ | ✅ | ✅ | — | — | CLI smoke |
| D | ✅ | ✅ | ✅ | ✅ | ✅ | Settings UI |
| E | — | — | — | ✅ | ✅ | Wizard walkthrough |
| F | ✅ | ✅ | ✅ | ✅ | ✅ | Notification UI |
| G | ✅ | ✅ | ✅ | — | — | CLI smoke |
| H | ✅ | ✅ | ✅ | — | — | — |
| I | ✅ | ✅ | ✅ | ✅ | ✅ | Full app smoke |

---

### v0.6.0 Implementation Tasks — Cross-Platform Performance Foundation

> Priority: Complete all tasks in Phase order. Each trait impl is independently testable.
> Full roadmap: `docs/v0.6.0-plan.md`

#### Phase 1: Platform Trait Foundation (v0.5.1 — v0.5.3)

**Task J — Define 6 platform traits + common types**

Goal: Create the trait definitions that all platform backends will implement. No logic changes — just interface definitions.

Files: `src-tauri/src/platform/mod.rs` (rewrite), `src-tauri/src/platform/common.rs` (new)

Steps:
1. Rewrite `platform/mod.rs`: define all 6 traits with full method signatures (see v0.6.0-plan.md § Architecture).
2. Create `platform/common.rs`: `WatcherGuard`, `TrashResult`, `RestoreResult`, `FileIdentity`, `PlatformProviders` struct.
3. Add `ScanStrategy` enum with `Auto`, `Jwalk`, and `Mft { admin: bool }` variants (MFT gated behind `#[cfg(feature = "mft-scanner")]`).
4. Re-export all types from `platform/mod.rs`.
5. Add 2+ compile-only tests: `PlatformProviders` struct construction compiles.

Verify: `cargo check`, `cargo test`

---

**Task K — Wire business logic through traits**

Goal: Replace all direct OS calls with trait method calls via `platform::providers()` dispatch.

Files: `src-tauri/src/lib.rs`, `src-tauri/src/scanner/mod.rs`, `src-tauri/src/watcher/mod.rs`, `src-tauri/src/cleaner/mod.rs`, `src-tauri/src/db/mod.rs`

Steps:
1. Add `platform::providers()` function — returns `PlatformProviders` with compile-time `#[cfg(target_os)]` dispatch.
2. Update `lib.rs` `start_fs_watcher`: call `providers.fs_watcher.start()` instead of `watcher::start_watching()`.
3. Update `scanner/mod.rs` `get_drive_space`: call `providers.disk_info.free_bytes()` instead of direct `GetDiskFreeSpaceExW`.
4. Update `scanner/mod.rs` `scan_top_level_dirs`: call `providers.dir_scanner.execute()` instead of hardcoded jwalk.
5. Update `cleaner/mod.rs` `clean_items_with_progress`: call `providers.cleanup.move_to_trash()`.
6. Update `db/mod.rs` `set_db_path`: call `providers.system_info.app_data_dir()`.
7. All existing 81 tests must pass — traits are additive, no behavior change.

Verify: `cargo test` (81/81), `cargo clippy -- -D warnings`

---

**Task L — Extract Windows implementations into trait framework**

Goal: Move current Windows-specific code into `impl` blocks. Current polling watcher remains as `WindowsPollWatcher`.

Files: `src-tauri/src/platform/windows.rs` (new), `src-tauri/src/platform/mod.rs`

Steps:
1. Create `platform/windows.rs` with 5 impl blocks:
   - `impl DiskInfoProvider for WindowsDiskInfoProvider` — move `get_drive_space` logic
   - `impl FsWatcher for WindowsPollWatcher` — move `start_watching` + snapshot diff
   - `impl ScanStage for JwalkStage` — move `MeasureStage` logic
   - `impl CleanupProvider for WindowsCleanupProvider` — move `SHFileOperationW` logic
   - `impl SystemInfo for WindowsSystemProvider` — basic Windows impl
2. Create `platform/windows_mft.rs` as technical reserve — `MftStage` struct + `ScanStage` impl skeleton (compiled, not wired).
3. Add `#[cfg(target_os = "windows")]` guards on all Windows impls.
4. `providers()` dispatch returns Windows impls when `#[cfg(windows)]`.

Verify: `cargo test` (81/81 must pass — same logic, new location), `cargo clippy`

---

#### Phase 2: Windows Native Performance (v0.5.4 — v0.5.6)

**Task M — ReadDirectoryChangesW native watcher**

Goal: Replace polling with kernel-push FS events on Windows.

Files: `src-tauri/src/platform/windows.rs`, `src-tauri/Cargo.toml`

Steps:
1. Add `Win32_System_IO` feature to `windows` crate in `Cargo.toml`.
2. Implement `WindowsNativeWatcher`:
   - `CreateFileW` on each watched dir with `FILE_FLAG_OVERLAPPED | FILE_FLAG_BACKUP_SEMANTICS`
   - `ReadDirectoryChangesW` with `FILE_NOTIFY_CHANGE_FILE_NAME | SIZE | LAST_WRITE`
   - Dedicated thread with `GetQueuedCompletionStatus` for overlapped I/O
   - Buffer: 64KB per watched directory
   - Debounce: same config as polling (default 1500ms)
   - Cancel: `CancelIo` + `CloseHandle`
3. Emit same `FsChangeBatch` format — frontend unchanged.
4. Update `providers()` to return `WindowsNativeWatcher` instead of `WindowsPollWatcher`.
5. Keep `WindowsPollWatcher` as fallback if `CreateFileW` fails (non-admin, network drive).
6. Add `impl FileMetaAnalyzer for WindowsFileMetaAnalyzer` — basic `GetFileInformationByHandle` wrapper.
7. Add 2+ tests: watcher guard drop stops thread; cancel flag triggers stop.

Verify: `cargo test`, manual: create/delete file, verify event in NotificationCenter < 1s

---

**Task N — Hard link aware duplicate detection**

Goal: Skip hard-linked files (same file on disk) in duplicate detection.

Files: `src-tauri/src/duplicates/mod.rs`, `src-tauri/src/platform/windows.rs`

Steps:
1. In duplicates scan loop, call `FileMetaAnalyzer::file_identity()` before hashing.
2. Build `HashMap<FileIdentity, ()>` — if two files share same identity, skip SHA-256 hashing for the second.
3. Add `hard_link_count` field to duplicate scan result metadata.
4. Report: "N hard links found, X bytes already shared on disk" in progress events.
5. Add 2 tests: hard-linked files are skipped; non-hard-linked files still processed.
6. Add `hard_link_count` to `FileEntry` type (frontend types.ts).

Verify: `cargo test duplicates`, `npm run typecheck`

---

**Task O — Sparse file detection**

Goal: Detect and report sparse files (apparent size ≠ size on disk).

Files: `src-tauri/src/platform/windows.rs`, `src-tauri/src/scanner/mod.rs`, `src/types.ts`

Steps:
1. Add `is_sparse()` and `size_on_disk()` to `FileMetaAnalyzer` Windows impl.
2. Check `FILE_ATTRIBUTE_SPARSE_FILE` via `GetFileInformationByHandle`.
3. For sparse files: call `GetCompressedFileSizeW` to get actual allocated bytes.
4. Add `size_on_disk_bytes: Option<u64>` to `FileEntry`.
5. Surface in `LargeFileFinder`: "12.0 GB apparent / 3.2 GB on disk (sparse)".
6. Add 1 test: create sparse file with `DeviceIoControl(FSCTL_SET_SPARSE)`, verify detection.

Verify: `cargo test scanner`, `npm run typecheck`

---

#### Phase 3: Linux + macOS (v0.5.7 — v0.5.9)

**Task P — Linux platform implementation**

Goal: All 6 traits implemented for Linux.

Files: `src-tauri/src/platform/linux.rs` (new), `src-tauri/Cargo.toml`

Steps:
1. Add `trash = "5"` to `Cargo.toml` (cross-platform, gated by `#[cfg(target_os = "linux")]`).
2. Implement all 6 traits:
   - `LinuxDiskInfoProvider`: `statvfs()` + parse `/proc/mounts`
   - `LinuxFsWatcher`: `inotify` + `epoll` in dedicated thread
   - `LinuxWalkStage`: `walkdir`-backed `ScanStage`
   - `LinuxCleanupProvider`: `trash-rs` crate
   - `LinuxFileMetaAnalyzer`: `statx()` (Linux 4.11+)
   - `LinuxSystemInfo`: `uname()`, `/proc/cpuinfo`, `/proc/meminfo`
3. Add `#[cfg(target_os = "linux")]` on all Linux impls.
4. Update `providers()` with Linux dispatch.
5. Add 3+ tests (Linux-only, with `#[cfg(target_os = "linux")]`).

Verify: `cargo check --target x86_64-unknown-linux-gnu`, `cargo test` (Windows tests unchanged)

---

**Task Q — macOS platform implementation**

Goal: All 6 traits implemented for macOS.

Files: `src-tauri/src/platform/macos.rs` (new), `src-tauri/Cargo.toml`

Steps:
1. Add `objc = "0.2"` and `fsevent-sys = "4"` to `Cargo.toml` (macOS only).
2. Implement all 6 traits:
   - `MacOsDiskInfoProvider`: `statfs()` + `NSFileManager` via `objc`
   - `MacOsFsWatcher`: `FSEvents` API via `fsevent-sys`
   - `MacOsWalkStage`: `walkdir`-backed `ScanStage`
   - `MacOsCleanupProvider`: `NSFileManager.trashItem()` or `osascript`
   - `MacOsFileMetaAnalyzer`: `stat()` → `st_nlink`, `st_ino`, `st_dev`
   - `MacOsSystemInfo`: `uname()`, `sysctl`
3. Add `#[cfg(target_os = "macos")]` on all macOS impls.
4. Update `providers()` with macOS dispatch.
5. Add 3+ tests (macOS-only, with `#[cfg(target_os = "macos")]`).

Verify: `cargo check --target x86_64-apple-darwin`, `cargo test` (Windows tests unchanged)

---

**Task R — CI/CD + cross-platform packaging**

Goal: GitHub Actions matrix build + packaging for all 3 platforms.

Files: `.github/workflows/ci.yml` (new), `src-tauri/tauri.conf.json`

Steps:
1. Create `.github/workflows/ci.yml` with OS matrix: `[windows-latest, ubuntu-latest, macos-latest]`.
2. Each job: checkout → setup Node/Rust → `npm ci` → `cargo test` → `npm run typecheck` → `npm run build:web` → `npm run tauri build`.
3. Windows: upload `.msi` + `.exe` artifacts.
4. Linux: upload `.deb` + `.AppImage` artifacts.
5. macOS: upload `.dmg` artifact.
6. Update `tauri.conf.json` bundle config for Linux and macOS targets.
7. Run CI once to verify all 3 platforms green.

Verify: CI green on push; all artifacts downloadable

---

#### Phase 4: Release (v0.6.0)

**Task S — Integration tests + docs sync + release**

Goal: End-to-end verification across all platforms. Bump version to 0.6.0.

Steps:
1. Cross-platform integration test: scan → classify → duplicate detection → health → export (Windows + Linux + macOS).
2. Verify native watcher: create/delete/modify files, check event latency < 1s.
3. Verify hard link dedup: create hard links, run duplicate scan, confirm skipped.
4. Update all docs: `CLAUDE.md`, `PROGRESS.md`, `CHANGELOG.md`, `CODEX.md`, `README.md`, `README_zh-CN.md`.
5. Version bump to `0.6.0` in `Cargo.toml`, `package.json`, `tauri.conf.json`.
6. `cargo test` — target 95+ tests, all platforms.
7. `npm run tauri build` — all 3 platforms.
8. Generate and record release artifact hashes.

Verify: All checks green, installers generated for all 3 platforms.

---

### v0.6.0 Task Dependency Graph

```
Phase 1:  J → K → L
              │
Phase 2:  M → N → O
              │
Phase 3:  P ──┤── Q → R
              │
Phase 4:  S (depends on all above)
```

- J, K, L: Sequential (definitions → wiring → extract)
- M, N, O: Sequential (watcher → dedup → sparse)
- P, Q: Parallel (independent platform impls — can run simultaneously on different machines)
- R: Depends on P + Q
- S: Depends on entire Phase 1-3

### v0.6.0 Verification Matrix

| Task | cargo check | cargo test | cargo clippy | npm typecheck | npm build:web | cross-compile | manual |
|------|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| J | ✅ | ✅ | ✅ | — | — | — | — |
| K | ✅ | ✅ | ✅ | ✅ | ✅ | — | — |
| L | ✅ | ✅ | ✅ | — | — | — | — |
| M | ✅ | ✅ | ✅ | — | — | — | FS watch smoke |
| N | ✅ | ✅ | ✅ | ✅ | — | — | — |
| O | ✅ | ✅ | ✅ | ✅ | — | — | — |
| P | ✅ | ✅ | ✅ | — | — | `x86_64-unknown-linux-gnu` | — |
| Q | ✅ | ✅ | ✅ | — | — | `x86_64-apple-darwin` | — |
| R | — | — | — | — | ✅ | — | CI green |
| S | ✅ | ✅ | ✅ | ✅ | ✅ | — | Full smoke |

### v0.7.0 Implementation Tasks — Intelligent Operations Platform

> Priority: Execute in Phase order. Each version is independently deliverable.
> Full roadmap: `docs/v0.7.0-plan.md`

#### Phase 1: Foundation Polish (v0.6.1 — v0.6.2)

**Task T — Streaming incremental scan (v0.6.1) ✅ Complete**

Goal: Replace batch scan with streaming pipeline. First result <500ms, memory <50MB.

Files: `src-tauri/src/scanner/mod.rs`, `src-tauri/src/platform/mod.rs`, `src-tauri/src/platform/windows.rs`, `src-tauri/src/lib.rs`, `src/types.ts`, `src/App.tsx`, `src/hooks/useDriveScan.ts`

Steps:
1. Add `ScanBatch` struct: `{ dirs: Vec<DirInfo>, batch_index: u32, is_complete: bool }`.
2. Add `execute_streaming(&self, ctx: &ScanContext) -> Receiver<ScanBatch>` to `ScanStage` trait.
3. Implement `execute_streaming()` in `JwalkStage`: emit one batch per completed top-level directory.
4. Add `Streaming` variant to `ScanPhase` enum; emit `scan-batch` IPC event during streaming.
5. Update `useDriveScan` hook: listen for `scan-batch` events; call `invoke("scan_drive_dirs", ...)` to start streaming.
6. Update `App.tsx`: render Treemap incrementally as `scan-batch` events arrive (not waiting for Complete).
7. Add incremental rescan: when watcher detects changes in a directory, call `scan_directory(path)` and refresh that Treemap node only.
8. Add `cancel_scan` check between each batch.
9. Add 4+ tests: streaming push ordering, first-batch timing, incremental rescan, cancel mid-stream.

Verify: `cargo test` (90+), `npm run typecheck`, manual: scan C: drive, observe Treemap appearing batch-by-batch.

---

**Task U — Custom rule editor UI (v0.6.2) ✅ Complete**

Goal: UI for creating, editing, testing, and deleting custom risk rules.

Files: `src/components/RuleEditor.tsx` (new), `src/components/RuleTester.tsx` (new), `src/pages/Settings/index.tsx`, `src/types.ts`, `src-tauri/src/risk/mod.rs`, `src-tauri/src/lib.rs`

Steps:
1. Add `test_rule_pattern(pattern: String, test_path: String) -> bool` to `risk/mod.rs`.
2. Register `test_rule_pattern` IPC command in `lib.rs`.
3. Create `RuleEditor.tsx`: form with name input, glob pattern input, risk level select (LOW/MEDIUM only), save/cancel buttons.
4. Create `RuleTester.tsx`: test path input + "Test" button → displays match/no-match with color indicator.
5. Add "Custom Rules" sub-tab in Settings page: list existing custom rules, "New Rule" button, per-row edit/delete.
6. Delete button only shown for custom rules (not built-in). Confirmation dialog before delete.
7. Add TypeScript types: `CustomRule { id, name, pattern, risk_level, enabled }`.
8. Add 2+ tests: pattern matching correctness, rule CRUD IPC.

Verify: `cargo test`, `npm run typecheck`, manual: create a rule → test it → see it appear in cleanup report.

---

#### Phase 2: Deep Performance (v0.6.3 — v0.6.4)

**Task V — MFT direct scan activation (v0.6.3) ✅ Complete**

Goal: Activate `MftStage` technical reserve. Use `FSCTL_ENUM_USN_DATA` for fast approximate scanning.

Files: `src-tauri/src/platform/windows_mft.rs`, `src-tauri/src/platform/mod.rs`, `src-tauri/src/scanner/mod.rs`, `src-tauri/Cargo.toml`

Steps:
1. Implement `MftStage::execute_streaming()`: open volume handle via `CreateFileW(r"\\.\C:", ...)`, call `FSCTL_GET_NTFS_VOLUME_DATA`, enumerate `FSCTL_ENUM_USN_DATA`, parse USN records into `DirInfo`, emit via `ScanBatch`.
2. Add admin privilege detection: `check_admin_privilege() -> bool` via `OpenProcessToken` + `TokenElevation`.
3. Update `ScanStrategy::Auto`: if admin → try MftStage; if MFT fails → fallback to JwalkStage.
4. Add `mft-scanner` feature flag in `Cargo.toml` (default off). Guard MftStage code with `#[cfg(feature = "mft-scanner")]`.
5. Mark MFT results as "approximate" in UI; JwalkStage remains "exact" default.
6. Add Settings option: "Scan Mode: Speed (approximate) / Accuracy (exact)".
7. Add 4+ tests: admin privilege detection, MFT fallback, strategy selection, USN record parsing.
8. Add benchmark: `cargo bench --bench performance` comparing MFT vs Jwalk on same drive.

Verify: `cargo test`, admin CMD: run MFT scan → compare results with Jwalk → verify faster.

---

**Task W — Windows Service mode (v0.6.4) ✅ Complete**

Goal: Run DiskPulse as a background Windows Service. Monitor/alert/collect snapshots without GUI.

Files: `src-tauri/src/service/mod.rs` (new), `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src/pages/Settings/index.tsx`, `src/types.ts`

Steps:
1. Create `service/mod.rs`: `install()`, `uninstall()`, `start()`, `stop()`, `status()` using windows crate SCM API (`OpenSCManagerW`, `CreateServiceW`, etc.).
2. Add `--service` CLI flag in `main.rs`: when present, start background engine (alert monitor + scheduler + snapshot collection), no window.
3. Implement Named Pipe server in service mode: `\\.\pipe\DiskPulseService` — JSON messages matching IPC command format.
4. Implement Named Pipe client in GUI mode: auto-connect when service is detected.
5. Register IPC: `install_service`, `uninstall_service`, `get_service_status`.
6. Settings UI: "Service" tab with install/uninstall button, status indicator (running/stopped/not installed), auto-start checkbox.
7. Safety: Service runs as LOCAL SERVICE account. Service NEVER executes cleanup operations.
8. Add 3+ tests: service IPC, status transitions, pipe message round-trip.

Verify: `cargo test`, manual: install service → start → check GUI detects it → check alerts work → stop → uninstall.

---

#### Phase 3: Intelligence (v0.6.5 — v0.6.6)

**Task X — ML anomaly detection (v0.6.5) ✅ Complete**

Goal: Holt-Winters seasonal forecasting + Modified Z-Score anomaly detection. Pure Rust, zero ML deps.

Files: `src-tauri/src/anomaly/mod.rs` (new), `src-tauri/src/prediction/mod.rs`, `src-tauri/src/lib.rs`, `src/components/AnomalyCard.tsx` (new), `src/pages/History/index.tsx`, `src/types.ts`

Steps:
1. Create `anomaly/mod.rs` with `AnomalyDetector` struct:
   - `HoltWinters { alpha: f64, beta: f64, gamma: f64, period: usize }` — triple exponential smoothing.
   - `ModifiedZScore { threshold: f64 }` — MAD-based outlier detection.
   - `detect(history: &[Snapshot]) -> Vec<AnomalyEvent>` — run both models, return events.
2. 4 anomaly types with thresholds: RateAnomaly (|Mi| > 3.5), BurstAnomaly (|Mi| > 10.0), HotspotAnomaly (single-dir growth > 3.5σ), PatternDeviation (seasonal shift).
3. Upgrade `prediction/mod.rs`: replace OLS with Holt-Winters. Keep OLS as fallback when data points < 2*period.
4. Extend `predict_disk_usage` response: add `seasonal_component`, `trend_component`, `dynamic_confidence_interval`.
5. Register `detect_anomalies` IPC command.
6. Create `AnomalyCard.tsx`: dashboard card showing anomaly count + last anomaly description + severity color.
7. Add "Anomaly Detection" tab in History page: ECharts time series with anomaly markers (red dots on abnormal points).
8. Emit `anomaly-detected` event when real-time anomaly detected.
9. Add 5+ tests: HW forecast accuracy (compare with known series), MAD calculation, anomaly threshold, fallback to OLS, empty history handling.

Verify: `cargo test` (98+), manual: accumulate 2 weeks of snapshots → check anomaly detection output.

---

**Task Y — Smart recommendations v2 (v0.6.6) ✅ Complete**

Goal: Context-aware scoring with urgency multiplier, user behavior learning, and 4D health radar.

Files: `src-tauri/src/recommendations/mod.rs`, `src-tauri/src/db/mod.rs`, `src/components/RecommendationCard.tsx`, `src/components/DiskHealthGauge.tsx`, `src/types.ts`

Steps:
1. Add `urgency_multiplier` to `score_recommendation()`: query `predict_disk_usage` → `days_until_full` → map to multiplier (1.0 ~ 3.0).
2. Add `pattern_boost`: query `cleanup_logs` table → group by category → boost score per category (1.0 ~ 1.5x).
3. Add `correlation_bonus`: if same path appears in multiple detectors (aging + duplicates + anomaly + large files) → +bonus points.
4. Update `scoring_weights` to include new `urgency_weight` and `pattern_weight` in `AppSettings`.
5. Redesign `get_disk_health()`: return 4 sub-scores (Space/Waste/Trend/Age) instead of single number.
6. Create `DiskHealthRadar.tsx`: ECharts radar chart showing 4 health dimensions.
7. Update `RecommendationCard.tsx`: show urgency badge (🟢 Normal / 🟡 Elevated / 🔴 Critical) based on multiplier.
8. Add 3+ tests: urgency multiplier calculation, pattern learning from history, health dimension split.

Verify: `cargo test`, `npm run typecheck`, manual: fill disk → see urgency multiplier increase recommendations.

---

#### Phase 4: Ecosystem (v0.6.7)

**Task Z — Multi-device dashboard (v0.6.7)**

Goal: WebSocket-based hub for跨设备监控. One machine sees all devices' disk status.

Current status (2026-06-04): complete. Real `tokio-tungstenite` transport, mDNS advertisement/browser, App UI device selector, pairing token controls, and remote scan command routing are implemented.

Files: `src-tauri/src/hub/` (6 new files), `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`, `src/App.tsx`, `src/types.ts`, `src/hooks/useRemoteDevice.ts` (new), `src/components/*`

Steps:
1. DONE: Add `tokio-tungstenite` and `mdns-sd` to `Cargo.toml` (cross-platform).
2. DONE: Create `hub/mod.rs`: Hub start/stop lifecycle state.
3. DONE: Create `hub/server.rs`: `tokio-tungstenite` WS server on configurable localhost port.
4. DONE: Create `hub/registry.rs`: `DeviceRegistry` — `HashMap<String, DeviceInfo>`.
5. DONE: Create `hub/router.rs`: typed hub event envelope + read-only WS request/response routing.
6. DONE: Create `hub/pairing.rs`: generate 6-digit numeric token and verify single-use token.
7. DONE: Create `hub/discovery.rs`: mDNS service advertisement + browser (Bonjour `_diskpulse._tcp`).
8. DONE: Register IPC: `start_hub`, `stop_hub`, `get_connected_devices`, `get_hub_discovery_info`, `discover_devices`, `create_pairing_token`, `pair_device`, `unpair_device`.
9. DONE: Create `useRemoteDevice` hook: hub status, device list, pairing, events, remote WS query helper.
10. DONE: Update `App.tsx`: dashboard device selector, online/offline indicator, Hub controls, pairing token controls.
11. DONE: Update dashboard data flow to render local or remote drive data.
12. DONE: Security: WS binds localhost only. Remote command whitelist allows read-only commands; cleanup remains blocked remotely.
13. DONE: Add 10 hub tests: message routing, device registry CRUD, pairing token validation, mDNS parsing, remote WebSocket query.

Verify: `cargo test` (108+), manual: two devices → pair → view remote disk → receive remote alert.

---

#### Phase 5: Release (v0.7.0)

**Task AA — Integration tests + docs sync + release**

Goal: End-to-end verification across all 5 pipelines. Bump version to 0.7.0.

Steps:
1. Integration test 1 — Scan pipeline: `streaming_scan → MFT_scan → Jwalk_exact → cancel → incremental_rescan`.
2. Integration test 2 — Intelligence pipeline: `scan → classify → anomaly_detect → recommend_v2 → disk_health_radar`.
3. Integration test 3 — Cleanup pipeline: `recommend → preview → clean → undo` (with custom rule matches).
4. Integration test 4 — Service pipeline: `install → start → monitor_alert → snapshot → stop → uninstall`.
5. Integration test 5 — Hub pipeline: `start_hub → mdns_discover → pair → remote_query → event_forward → stop`.
6. DONE: Update all docs: `CLAUDE.md`, `PROGRESS.md`, `CHANGELOG.md`, `CODEX.md`, `README.md`, `README_zh-CN.md`.
7. DONE: Version bump to `0.7.0` in `Cargo.toml`, `package.json`, `tauri.conf.json`.
8. DONE: `cargo test` — target 115+ tests, reached 119 tests locally.
9. DONE: `cargo clippy -- -D warnings`, `npm run typecheck`, `npm run build:web`.
10. `npm run tauri build` — all 3 platforms (Windows MSI/NSIS, Linux .deb/.AppImage, macOS .dmg).
11. Record release artifact hashes. Create GitHub release tag v0.7.0.

Verify: All checks green, installers generated for all 3 platforms.

---

### v0.7.0 Task Dependency Graph

```
Phase 1:  T → U (T and U independent, can parallelize)
              │
Phase 2:  V ──┤── W (V depends on T for streaming interface; W independent)
              │
Phase 3:  X ──┤── Y (X must come before Y, Y uses X's anomaly signals)
              │
Phase 4:  Z (depends on T for streaming data over WS; X+Y for multi-device intelligence)
              │
Phase 5:  AA (depends on all above)
```

### v0.7.0 Verification Matrix

| Task | cargo check | cargo test | cargo clippy | npm typecheck | npm build:web | manual |
|------|:--:|:--:|:--:|:--:|:--:|:--:|
| T | ✅ | ✅ (90+) | ✅ | ✅ | ✅ | Streaming UI |
| U | ✅ | ✅ | ✅ | ✅ | ✅ | Rule editor |
| V | ✅ | ✅ | ✅ | — | — | MFT vs Jwalk |
| W | ✅ | ✅ | ✅ | ✅ | ✅ | Service smoke |
| X | ✅ | ✅ (98+) | ✅ | ✅ | ✅ | Anomaly check |
| Y | ✅ | ✅ | ✅ | ✅ | ✅ | Rec changes |
| Z | ✅ | ✅ (108+) | ✅ | ✅ | ✅ | Multi-device |
| AA | ✅ | ✅ (115+) | ✅ | ✅ | ✅ | Full smoke |

### v0.8.0 Implementation Tasks — Production-Ready Deep Intelligence

> Priority: Execute in Phase order. Each version is independently deliverable.
> Full roadmap: `docs/v0.8.0-plan.md`
> Status: Planning complete — tasks to be detailed when implementation begins.

#### Phase 1: Production-Ready (v0.7.1 — v0.7.5)

**Task AB — Code Signing (v0.7.1) ✅ Local Complete**
- SignPath Foundation setup + `.signpath/config.yml`
- Homebrew Cask formula creation
- CI integration for signed artifacts
- Remaining external prerequisites: SignPath Foundation approval, `SIGNPATH_API_TOKEN`, `SIGNPATH_ORGANIZATION_ID`, signed artifact smoke on release tag.

**Task AC — Linux Native CI (v0.7.2) ✅ Local Complete**
- GitHub Actions ubuntu-latest build/test/package configuration
- Linux GTK/WebKit/OpenSSL/FUSE dependencies installed in CI
- `.deb` + `.AppImage` bundle verification and strict artifact upload
- Linux cleanup uses trash-rs first with `gio trash` fallback
- Inotify FFI parser has Linux-only multi-record coverage
- Remaining native prerequisite: GitHub Actions run on `ubuntu-latest`

**Task AD — macOS Native CI + FSEvents (v0.7.3)**
- GitHub Actions macos-latest build/test/package
- FSEvents native watcher activation (polling → kernel-push)

**Task AE — Code Split + Auto-Update (v0.7.4)**
- React.lazy route splitting (first screen <300KB gzip)
- `check_for_update` IPC command via GitHub Release API
- Installer upgrade data preservation (user data dir isolation)

**Task AF — Perf Bench + i18n + Edge Fixes (v0.7.5)**
- 10 benchmarks (synthetic fixtures)
- Japanese locale (`ja.json`)
- 6 edge-case fixes (empty drive, large single file, deep nesting, etc.)

#### Phase 2: Deep Intelligence (v0.7.6 — v0.7.10)

**Task AG — Disk Fragmentation Analysis (v0.7.6)**
- 3-platform extent detection (FSCTL/FIEMAP/F_LOG2PHYS)
- `FragmentationView.tsx` UI component
- Integration into recommendation engine

**Task AH — burn DL Anomaly Detection (v0.7.7)**
- burn Autoencoder model (6→4→6 dims, pure Rust)
- 3-way signal fusion (HW + Z-Score + AE)
- Compile-time feature gate (`ml-engine`)
- 8-point runtime fallback matrix

**Task AI — Health Score v2 (v0.7.8)**
- 4D → 6D radar (Space/Waste/Trend/Age/Frag/Anomaly)
- Composite health score + trend tracking + actionable advice
- `health_snapshots` table

**Task AJ — Predictive Cleanup (v0.7.9)**
- `predict_disk_full()` with confidence intervals
- `simulate_cleanup_gain()` quantified benefit estimation
- Time-sensitive pre-cleanup list + user confirmation safety invariant

**Task AK — Smart File Classification (v0.7.10)**
- 3-stage pipeline: extension → magic bytes → burn classifier
- 12 output categories + `file_category` risk rule condition
- `fileclass/` module (4 files)

#### v0.8.0 Task Dependency Graph

```
Phase 1:
AB ──┬── AC ──┬── AE ── AF (P1, depends on P0 CI infrastructure)
     ├── AD ──┤
     └── (AC/AD independent, can parallelize)

Phase 2:
AG ──┬── AI ──┬── AJ (P1)
     ├── AH ──┤
     └── AK (P1, independent of AG/AH/AI)

AG-AH: independent (fragmentation reads disk, AE reads history)
AI: depends on AG (Frag score) + AH (Anomaly score)
AJ: depends on AI (health score → urgency → pre-cleanup)
AK: independent
```

#### v0.8.0 Verification Matrix

| Task | cargo test | cargo clippy | npm typecheck | npm build:web | cross-platform CI |
|------|:--:|:--:|:--:|:--:|:--:|
| AB | — | — | — | — | SignPath smoke |
| AC | ✅ (119+) | ✅ | — | — | ubuntu-latest |
| AD | ✅ (119+) | ✅ | — | — | macos-latest |
| AE | — | — | ✅ | ✅ (split check) | — |
| AF | ✅ | ✅ | ✅ | ✅ | — |
| AG | ✅ (+5) | ✅ | ✅ | ✅ | 3-platform |
| AH | ✅ (+8) | ✅ | — | — | ml-engine gate test |
| AI | ✅ (+7) | ✅ | ✅ | ✅ | — |
| AJ | ✅ (+7) | ✅ | ✅ | ✅ | — |
| AK | ✅ (+5) | ✅ | ✅ | ✅ | — |

### burn Fallback Verification

| Test | Condition | Expected |
|------|-----------|----------|
| `ml-engine` feature OFF | `cargo test --no-default-features` | All 119+ tests pass (statistical-only mode) |
| AE model missing | Delete `diskpulse_ae.burn` | `ae_disabled=true`, no crash |
| AE inference timeout | Inject >10ms delay | Batch skipped, next batch retries |
| AE accuracy degraded | AUC < 0.7 synthetic test | Weights auto-adjust, statistical weight > 0.9 |

## Non-Negotiable Safety Rules

These rules apply to every cleanup, scheduler, risk, and filesystem feature:

- Never implement permanent delete behavior for cleanup paths.
- All cleanup deletions must go through Windows Recycle Bin support (`FOF_ALLOWUNDO`).
- Validate every path before deletion; allowed cleanup is whitelist-based.
- Protect system and high-risk paths, including `C:\Windows`, `System32`, `WinSxS`, installer caches, registry hives, chat history, and `Program Files` unless a reviewed design explicitly allows display-only handling.
- Skip locked or in-use files; never force-delete.
- Auto-cleanup may only clean LOW-risk items marked safe by the existing safety pipeline.
- Keep preview-before-execute behavior intact.

Any change touching `src-tauri/src/cleaner/`, `src-tauri/src/risk/`, `src-tauri/src/scheduler/`, or path validation requires extra tests and careful review.

## Architecture Map

Backend modules live in `src-tauri/src/`:

- `main.rs`: app entry point.
- `lib.rs`: Tauri setup, IPC command registration, tray, startup events, shared app orchestration.
- `scanner/mod.rs`: drive scanning, parallel traversal, large-file finder, cancellation, scan progress.
- `risk/mod.rs`: rule-based risk classification and cleanup eligibility.
- `cleaner/mod.rs`: preview, Recycle Bin cleanup, undo, safety checks, progress events.
- `watcher/mod.rs`: polling filesystem watcher and change batching.
- `db/mod.rs`: SQLite snapshots, cleanup logs, settings, rule overrides, auto-cleanup reports, cache.
- `alert/mod.rs`: disk-space thresholds, sudden-growth checks, notifications.
- `prediction/mod.rs`: linear regression forecast from snapshot history.
- `scheduler/mod.rs`: scheduled auto-cleanup, LOW-risk invariant, report persistence.

**v0.4.0 planned modules:**
- `duplicates/mod.rs`: (v0.3.4) duplicate file detection via 3-phase pipeline.
- `aging/mod.rs`: (v0.3.5) file aging analysis, zombie finder, growth hotspots.
- `recommendations/mod.rs`: (v0.3.6) smart recommendation engine with weighted scoring.
- `report/mod.rs`: (v0.3.7) report generation & export (CSV/JSON).
- `cli/mod.rs`: (v0.3.9) CLI mode with 5 subcommands.
- `platform/mod.rs`: (v0.3.9) cross-platform abstraction traits.

Frontend modules live in `src/`:

- `App.tsx`: main shell, navigation, dashboard, event listeners.
- `types.ts`: shared TypeScript interfaces for IPC payloads and app state.
- `index.css`: Aurora visual system and Tailwind integration.
- `components/Treemap.tsx`: disk treemap visualization.
- `components/CleanupPreview.tsx`: preview, execute, undo UI.
- `components/LargeFileFinder.tsx`: large-file scan UI and cleanup handoff.
- `components/PredictionCard.tsx`: disk forecast display.
- `components/AutoCleanupStatus.tsx`: scheduler status and manual run.
- `pages/Cleanup/index.tsx`: risk-grouped cleanup report.
- `pages/History/index.tsx`: trend chart, snapshots, cleanup history, auto-cleanup reports.
- `pages/Settings/index.tsx`: general, rules, alerts, and automation settings.
- `hooks/useDriveScan.ts`: lazy meta/cache/background scan and cancel.
- `hooks/useFsEvents.ts`: watcher lifecycle.
- `hooks/useLargeFileFinder.ts`: large-file scan lifecycle.
- `utils/format.ts`: formatting helpers.

**v0.4.0 planned frontend files:**
- `src/i18n/index.ts`, `locales/*.json`: (v0.3.1) i18n system with I18nProvider.
- `src/hooks/useTheme.ts`, `src/components/ThemeSwitcher.tsx`: (v0.3.2) theme system.
- `src/components/DuplicateFinder.tsx`, `src/hooks/useDuplicateScan.ts`: (v0.3.4) duplicate file UI.
- `src/components/AgingAnalysis.tsx`, `src/hooks/useAgingAnalysis.ts`: (v0.3.5) aging analysis UI.
- `src/components/RecommendationCard.tsx`, `DiskHealthGauge.tsx`: (v0.3.6) recommendations & health UI.
- `src/components/CleanupWizard.tsx`, `NotificationCenter.tsx`: (v0.3.8) wizard & notification center.

## Development Rules

Rust:

- Keep production code free of `unwrap()` and `expect()` unless there is a documented, unavoidable invariant.
- Use `Result<T, String>` for Tauri command boundaries and `anyhow::Result` or module-local error handling internally where already established.
- Add or update unit tests in the same module under `#[cfg(test)]`.
- Register every new Tauri command in `generate_handler![]` and keep frontend command names in sync.
- Define IPC event names as constants where the module pattern already does so.
- Preserve cancellation paths for long-running scans and cleanup operations.

TypeScript/React:

- Keep TypeScript strict; avoid `any`.
- Use `invoke<T>()` with explicit return types for Tauri commands.
- Put shared IPC and domain types in `src/types.ts`.
- Use `listen<T>()` for Tauri events and always clean up listeners.
- Provide loading, empty, and error states for data-fetching UI.
- Preserve the existing Aurora design language unless the user asks for a redesign.

General:

- Make small, targeted edits.
- Do not refactor unrelated code while fixing a bug.
- Do not edit generated build artifacts unless the task is explicitly release/build packaging.
- Do not commit or stage `.claude/settings.local.json` or `production/` artifacts unless the user explicitly instructs otherwise.

## IPC Change Checklist

When adding or changing a Tauri command:

1. Update Rust command function and serialization types.
2. Register the command in `src-tauri/src/lib.rs`.
3. Update `src/types.ts` if the frontend consumes the payload.
4. Update frontend `invoke<T>()` call sites.
5. Add or update backend tests.
6. Run Rust tests and TypeScript typecheck.
7. Update `CLAUDE.md`, `PROGRESS.md`, or release docs if the public API changed.

## Verification Matrix

Use the smallest useful verification first, then expand before declaring completion.

Backend quick checks:

```powershell
cd src-tauri
cargo check
cargo test
cargo clippy -- -D warnings
```

Frontend quick checks from repo root:

```powershell
npm run typecheck
npm run build:web
```

Full app/release checks when relevant:

```powershell
npm run tauri dev
npm run tauri build
```

Manual smoke checks are required for UI, cleanup, watcher, notification, scheduler, and release packaging changes.

## Task Handling Pattern

For each assigned task:

1. Restate the goal briefly if it is ambiguous.
2. Locate the relevant modules with `rg`.
3. Read existing implementation and tests.
4. Make the smallest safe change.
5. Add or update tests for behavior changes.
6. Run targeted verification.
7. Report exactly what changed, what was verified, and any remaining risk.

## Known Project Priorities

- Safety-first cleanup behavior.
- Fast startup and responsive scanning.
- Clear progress/cancel feedback for long operations.
- Beautiful but consistent Aurora-style UI.
- Reliable local history through SQLite.
- Windows-first behavior and installer readiness.

## Documentation Update Policy

Update docs when a change affects architecture, public commands, release status, setup, or user-facing behavior:

- `PROGRESS.md`: current implementation status, test counts, release checklist.
- `CLAUDE.md`: architecture, IPC API, conventions, safety rules.
- `CHANGELOG.md`: user-facing feature/fix history.
- `docs/`: roadmap or sprint-level design details.
- `README.md` / `README_zh-CN.md`: public setup and feature descriptions.

Keep this `CODEX.md` focused on agent operating context, not detailed changelog history.
