# Changelog

All notable changes to DiskPulse will be documented in this file.

## [0.9.1] - 2026-06-06

> M3 Relay Server local-ready foundation. This version adds a self-hosted relay server binary, relay client status model, read-only envelope guard, local WebSocket register handshake, IPC, and verification gate. Public relay deployment, DNS/TLS, and real cross-WAN hardware validation remain external CI/ops gates.

### Relay Server

- Added `relay` module with `RelayRuntime`, `RelayStatus`, `CloudDevice`, `RelayEnvelope`, and local-ready connect/disconnect/status state.
- Added local WebSocket relay server handshake for device registration plus ping/route message shapes.
- Added read-only relay envelope validation by reusing the Hub remote-command allowlist; cleanup/write commands are refused without local confirmation.
- Added standalone `diskpulse-relay` Rust binary for self-hosted local relay smoke runs.
- Added Tauri IPC commands: `connect_relay`, `disconnect_relay`, `get_relay_status`, and `list_cloud_devices`.
- Added frontend TypeScript relay/cloud device models and `npm run verify:m3-relay`.
- Bumped app/package versions to `0.9.1`.

### Verification (v0.9.1)

- `cargo test --manifest-path src-tauri\Cargo.toml`: 147/147 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed.
- `npm run verify:m3-relay`: passed.
- `npm run verify:m2-intelligence`: passed.
- `npm run verify:m1-release`: passed.
- `npm run verify:signing`: passed.
- `npm run verify:linux-ci`: passed.

---

## [0.9.0] - 2026-06-06

> M2 Full Intelligence local completion. v0.8.4-v0.8.8 are implemented locally: AE foundation, Stage 3 classifier, external storage detection, 5-language i18n, and AI Model management UI.

### Full Intelligence Completion

- Completed v0.8.7 i18n expansion with Korean (`ko`) and Spanish (`es`) locale bundles, language selector labels, and system-language auto resolution.
- Completed v0.8.8 AI Model management with model status IPC, 60-snapshot fine-tune gate, reset action, AUC/accuracy metrics, and a Settings → AI Model panel.
- Preserved v0.8.6 external storage IPC/events and v0.8.4-v0.8.5 AE/classifier foundations as the v0.9.0 M2 baseline.
- Bumped app/package versions to `0.9.0`.

### Verification (v0.9.0)

- `cargo test --manifest-path src-tauri\Cargo.toml`: 142/142 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed.
- `npm run verify:m2-intelligence`: passed.
- `npm run verify:m1-release`: passed.
- `npm run verify:signing`: passed.
- `npm run verify:linux-ci`: passed.

---

## [1.0.0] — Planned (target: mid-July 2026)

> Full plan: `docs/v1.0.0-plan.md`. 4 milestones (M1–M4), 14 feature versions, 180+ tests target.

### M1: Production Verification (v0.8.1–v0.8.3)
- SignPath Foundation OSS approval + Windows EV signing
- Linux ubuntu-latest native CI: cargo test + .deb/.AppImage packaging
- macOS macos-latest native CI: FSEvents activation + .dmg packaging

#### Local readiness update (2026-06-05)
- Added `npm run verify:m1-release` and `docs/m1-release-readiness.md` to keep the v0.8.1-v0.8.2 gate explicit and repeatable.
- Hardened Windows release artifacts: unsigned and signed installer uploads now fail if missing, signed output is checked for MSI/EXE files, and artifacts use 14-day retention.
- Fixed Linux/macOS bundle verification split: Linux now validates both `.deb` and `.AppImage`; macOS validates `.dmg` only.
- External gates remain pending: SignPath Foundation approval/secrets and a real `ubuntu-latest` GitHub Actions run.

### M2: Full Intelligence → v0.9.0 (v0.8.4–v0.8.8)
- burn Autoencoder anomaly detection (6-dim→4-dim→6-dim, ~50KB model)
- burn file classifier Stage 3 (12-class softmax, ~80KB model)
- External storage auto-detection (WM_DEVICECHANGE / udev / IOKit)
- Korean + Spanish locales (5 languages total)
- Model fine-tune UI (Settings → AI Model panel)

### M3: Ecosystem → v0.10.0 (v0.9.1–v0.9.3)
- Self-hosted relay server (Rust binary, systemd/Docker, E2E encryption)
- Cloud Sync Bridge (WAN device pairing, relay-based)
- Web Dashboard (embedded HTTP server, shared React dual-build, interactive mode)

### M4: Public Release → v1.0.0
- 5 integration test pipelines, 10 real-hardware benchmarks
- 3-platform signed artifacts (SignPath Windows, Homebrew macOS, Snap Linux)
- 180+ Rust tests, all docs synced

---

## [0.8.6] - 2026-06-06

> M2 external storage detection foundation. Local implementation adds the cross-platform storage abstraction, Windows `WM_DEVICECHANGE` event model, Linux/macOS fallback providers, IPC, and tests.

### External Storage Detection

- Added `storage` module with `ExternalStorageProvider`, `ExternalStorageInfo`, attach/detach event payloads, and a polling monitor guard.
- Added Windows external storage provider using logical drive enumeration plus a `WM_DEVICECHANGE`/`DBT_DEVICEARRIVAL`/`DBT_DEVICEREMOVECOMPLETE` event model for volume unit masks.
- Added Linux fallback provider based on `/proc/mounts` external mount heuristics (`/media`, `/mnt`, `/run/media`) and macOS fallback provider based on `/Volumes`.
- Added Tauri IPC commands: `list_external_storage`, `get_storage_info`, `start_storage_monitor`, and `stop_storage_monitor`.
- Added frontend TypeScript models for external storage info and storage attach/detach events.
- Bumped app/package versions to `0.8.6`.

### Verification (v0.8.6)

- `cargo test --manifest-path src-tauri\Cargo.toml`: 140/140 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed.
- `npm run verify:m2-intelligence`: passed.
- `npm run verify:m1-release`: passed.
- `npm run verify:signing`: passed.
- `npm run verify:linux-ci`: passed.

---

## [0.8.5] - 2026-06-06

> M2 P0 intelligence foundation. Local implementation advances through v0.8.5 while SignPath approval/secrets and native CI runs remain external gates.

### Full Intelligence

- Added v0.8.4 AE foundation modules: 6D snapshot feature extraction, deterministic 6→4→6 autoencoder inference, and 5,400 synthetic training samples.
- Added v0.8.5 file classifier Stage 3: 8D file feature extraction, extended magic signatures, 12-class softmax-style classifier output, model version metadata, and stage-aware classification results.
- Enhanced cleanup risk rules with optional `file_category` conditions plus built-in Stage 3 rules for `dev_cache`, `build`, and `dependency` categories.
- Updated Settings rule details/search to surface `file_category`-based rules.
- Added the `ml-engine` feature gate placeholder and bumped app/package versions to `0.8.5`.

### Release Coordination

- M1 local readiness remains intact for Windows SignPath and Linux bundle verification; real SignPath approval/signing and GitHub-hosted native CI are still expected to be triggered externally.

### Verification (v0.8.5)

- `cargo test --manifest-path src-tauri\Cargo.toml`: 138/138 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed.
- `npm run verify:m2-intelligence`: passed.
- `npm run verify:m1-release`: passed.
- `npm run verify:signing`: passed.
- `npm run verify:linux-ci`: passed.

---

## [0.8.0] - 2026-06-05

> Production-Ready Deep Intelligence. Local v0.7.6-v0.8.0 implementation is complete with 129 Rust tests. Native Linux/macOS runner validation and true platform extent APIs remain external validation/follow-up items.

### Deep Intelligence

- Added sampled fragmentation analysis (`analyze_fragmentation`, `get_file_fragmentation`, `cancel_fragmentation_scan`) with top directory/file summaries and a new `FragmentationView`.
- Added anomaly fusion primitives and fallback weighting (`healthy`, `degraded`, `disabled`) for Holt-Winters + Modified Z-Score + optional Autoencoder signals.
- Upgraded disk health to 6D scoring with Space/Waste/Trend/Age/Frag/Anomaly dimensions plus `health_snapshots` persistence and `get_health_history`.
- Added predictive cleanup APIs (`predict_disk_full`, `simulate_cleanup_gain`, `get_pre_cleanup_candidates`, `execute_pre_cleanup`) and a dashboard `PredictiveCleanupCard`.
- Added Stage 1/2 file classification helpers and `file_category` on large-file, duplicate, and aging file entries.
- Bumped app/package versions to `0.8.0`.

### Verification (v0.8.0)

- `cargo test --manifest-path src-tauri\Cargo.toml`: 129/129 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed; first app chunk gzip is 3.76KB, React vendor gzip is 76.50KB, ECharts remains lazy-loaded.
- `cargo bench --manifest-path src-tauri\Cargo.toml`: 10 synthetic budget checks passed.
- `npm run verify:signing`: passed.
- `npm run verify:linux-ci`: passed.

### Known Follow-ups (deferred to v0.9.0)

- Real Windows `FSCTL_GET_RETRIEVAL_POINTERS`, Linux `FS_IOC_FIEMAP`, and macOS `F_LOG2PHYS` extent counters on native runners.
- Full `burn` model packaging/fine-tuning; current v0.8.0 uses the statistical fallback/fusion interfaces.
- Risk-rule persistence for `file_category` conditions and additional classifier model coverage.
- Cloud Sync Bridge, Mobile Companion App, Web Dashboard, and more languages (ko/es/etc.).

---

## [0.7.5] - 2026-06-05

> Phase 1 production-ready hardening for v0.7.3-v0.7.5. Local implementation and verification are complete; final FSEvents and `.dmg` confidence still requires a real macOS runner.

### macOS Native CI + FSEvents

- Added `macos-latest` `.dmg` bundle verification in CI and hardened macOS artifact upload.
- Replaced the macOS watcher polling path with a native CoreServices FSEvents stream plus polling fallback.
- Changed macOS trash handling to use the native trash provider through `trash-rs`.
- Filtered `/System/Volumes/Data` synthetic macOS volume entries from root scans.

### Code Split + Update Check

- Added React lazy route/component splitting with Suspense skeletons and Vite manual chunks.
- Added a GitHub Releases update check after launch with a persisted Settings toggle.
- Extended the Tauri CSP for the GitHub Releases API and bumped source-of-truth app versions to `0.7.5`.

### Perf Bench + i18n + Edge Fixes

- Expanded the synthetic performance bench to cover 10 v0.7.5 budget scenarios.
- Added Japanese locale support (`ja`) to the i18n framework and Settings language selector.
- Added an empty-drive/no-readable-folders dashboard state instead of showing a blank treemap.

### Verification (v0.7.5)

- `cargo test --manifest-path src-tauri\Cargo.toml`: 120/120 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed; first app chunk gzip is 3.76KB and React vendor gzip is 76.50KB, with ECharts deferred to a lazy vendor chunk.
- `cargo bench --manifest-path src-tauri\Cargo.toml`: 10 synthetic budget checks passed.

---

## [0.7.2] - 2026-06-04

> Phase 1 Linux native CI validation. Local workflow/configuration is complete; the final native confidence gate is a GitHub Actions run on `ubuntu-latest`.

### Linux Native CI

- Updated `.github/workflows/ci.yml` with additional Ubuntu system dependencies: `libssl-dev`, `pkg-config`, and `libfuse2`.
- Added a `Verify Linux bundles` step that fails when `.deb` or `.AppImage` packages are missing after `npm run tauri build`.
- Hardened Linux artifact upload with `if-no-files-found: error` and 14-day retention.
- Added `trash-rs` as a Linux-only dependency and changed Linux cleanup to prefer `trash::delete` with `gio trash` fallback.
- Added a Linux-only inotify parser unit test for multiple records in one kernel buffer.
- Added `docs/linux-ci.md` and `npm run verify:linux-ci` to document and validate the Linux release path.
- Bumped app/package versions to `0.7.2` across npm, Cargo, Cargo.lock, Tauri config, and Homebrew Cask metadata.

### Verification (v0.7.2)

- `npm run verify:linux-ci`: passed.
- `cargo check --manifest-path src-tauri\Cargo.toml`: passed.
- `cargo test --manifest-path src-tauri\Cargo.toml`: 119/119 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run verify:signing`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed (existing Vite chunk-size warning only).
- `npm run tauri build`: passed on Windows, generating MSI and NSIS installers.

---

## [0.7.1] - 2026-06-04

> Phase 1 production-ready signing foundation. Local configuration complete; external SignPath Foundation approval and repository secrets are still required before signed artifacts can be produced.

### Code Signing + Distribution

- Added `.signpath/config.yml` and `.signpath/policies/diskpulse/release-signing.yml` for Windows SignPath Foundation signing.
- Updated `.github/workflows/ci.yml` with release/tag signing flow, unsigned Windows artifact upload, SignPath request submission, and signed artifact upload.
- Added `packaging/homebrew/diskpulse.rb` as the Homebrew Cask starting point for macOS distribution.
- Added `docs/signing.md` with SignPath application steps, required GitHub secrets, CI flow, and Homebrew Cask submission checklist.
- Added `npm run verify:signing` and `scripts/verify-signing-config.mjs` to keep signing configuration testable.
- Bumped app/package versions to `0.7.1` across npm, Cargo, Cargo.lock, and Tauri config.

### Verification (v0.7.1)

- `npm run verify:signing`: passed.
- `cargo check --manifest-path src-tauri\Cargo.toml`: passed.
- `npm run typecheck`: passed.

---
## [0.7.0] - 2026-06-04

> Intelligent Operations Platform 鈥?released. 119 tests. v0.6.8 hardening was folded into the v0.7.0 release pass.

### Release Hardening

- Bumped app/package versions to `0.7.0` across npm, Cargo, Cargo.lock, and Tauri config.
- Added `tokio`, `tokio-tungstenite`, `futures-util`, and `mdns-sd` for multi-device Hub transport/discovery.
- Completed v0.7.0 verification: `cargo test`, `cargo clippy -- -D warnings`, `npm run typecheck`, and `npm run build:web`.
- Produced Windows MSI/NSIS bundles and recorded SHA-256 hashes in `docs/release-notes-v0.7.0.md`.
- Updated all project documentation: PROGRESS.md, README.md, README_zh-CN.md, CLAUDE.md, CODEX.md.

### Verification (v0.7.0)

- `cargo test`: 119/119 passed.
- `cargo clippy -- -D warnings`: 0 warnings.
- `npm run typecheck`: 0 errors.
- `npm run build:web`: passed (Vite chunk-size warning only).
- `npm run tauri build`: passed on Windows, generating MSI and NSIS installers.

### Known Follow-ups (deferred to v0.8.0)

- Validate Linux `.deb` / `.AppImage` and macOS `.dmg` on native CI runners.
- Add code signing and notarization before public distribution.
- Consider frontend code-splitting to remove the Vite chunk-size warning.
- Disk Defrag Analysis, Deep Learning Anomaly Detection, Mobile App, Cloud Sync.

## [0.6.7] - 2026-06-04

> Multi-device Dashboard complete. 119 tests.

### Multi-Device Hub MVP

- Added `src-tauri/src/hub/` module with lifecycle, registry, router, pairing, mDNS discovery, and WebSocket server.
- Added 6-digit single-use pairing tokens and connected-device registry CRUD.
- Registered Tauri IPC commands: `start_hub`, `stop_hub`, `get_connected_devices`, `get_hub_discovery_info`, `discover_devices`, `create_pairing_token`, `pair_device`, `unpair_device`.
- Added `device-connected`, `device-disconnected`, and `remote-alert` event constants/emission paths.
- Added `src/hooks/useRemoteDevice.ts` plus `DeviceInfo`, `PairingToken`, remote alert, discovery, and remote request types.
- Added Dashboard device selector with local/remote device switching, Hub controls, mDNS discovery, and pairing token controls.
- Added read-only remote command whitelist for `ping`, `scan_meta`, `scan_drive`, `get_disk_health`, and `detect_anomalies`; cleanup remains blocked remotely.

### Verification (v0.6.7)

- `cargo test`: 119/119 passed (up from 109 in v0.6.6).
- `cargo clippy -- -D warnings`: 0 warnings.
- `npm run typecheck`: 0 errors.
- `npm run build:web`: passed (Vite chunk-size warning only).

## [0.6.6] - 2026-06-03

> Phase 1-3 complete. 109 tests. v0.6.7 (multi-device) next.

### Streaming Incremental Scan (v0.6.1)

- `ScanStage::execute_streaming()` with `mpsc::Receiver<ScanBatch>` 鈥?first result <500ms.
- `scan-batch` IPC event for incremental frontend updates; Treemap renders batch-by-batch.
- Incremental rescan: watcher-detected changes trigger single-directory refresh.
- Memory target <50MB via streaming release; cancel <200ms between batches.

### Custom Rule Editor UI (v0.6.2)

- `RuleEditor.tsx` + `RuleTester.tsx` components for creating and live-testing custom risk rules.
- `test_rule_pattern` IPC command; custom rules constrained to LOW/MEDIUM risk levels only.
- "Custom Rules" sub-tab in Settings with list, create, edit, and delete operations.

### MFT Direct Scan (v0.6.3)

- `MftStage` activated via `FSCTL_ENUM_USN_DATA` 鈥?direct NTFS MFT enumeration.
- Admin privilege detection + automatic fallback to `JwalkStage` for non-admin users.
- `ScanStrategy::Auto` dispatches MFT (approximate, fast) vs Jwalk (exact, default).
- Feature-gated behind `mft-scanner` Cargo feature flag.

### Windows Service Mode (v0.6.4)

- `service` module: install/start/stop/uninstall via Windows SCM API.
- `diskpulse.exe --service` starts headless background engine (monitor + alerts + snapshots).
- Named Pipe IPC (`\\.\pipe\DiskPulseService`) 鈥?JSON messages reuse existing IPC format.
- Service runs as LOCAL SERVICE account; cleanup operations disabled in service mode.
- Settings UI: Service tab with status indicator, install/uninstall, auto-start toggle.

### ML Anomaly Detection (v0.6.5)

- `anomaly` module: Holt-Winters triple exponential smoothing + Modified Z-Score detector. Pure Rust, zero ML deps.
- 4 anomaly types: RateAnomaly, BurstAnomaly, HotspotAnomaly, PatternDeviation.
- `detect_anomalies` IPC command; `AnomalyCard.tsx` dashboard component.
- Upgraded `predict_disk_usage`: Holt-Winters seasonal components, dynamic confidence intervals, OLS fallback for short history.
- `anomaly-detected` IPC event for real-time alerting.

### Smart Recommendations v2 (v0.6.6)

- Context-aware scoring: urgency multiplier (1.0x鈥?.0x based on days-until-full), user behavior pattern learning from cleanup history, cross-module correlation bonus.
- 4D disk health radar chart (`DiskHealthRadar.tsx`): Space / Waste / Trend / Age sub-scores.
- Correlation bonus rewards paths appearing in multiple detectors (aging + duplicates + anomaly + large files).

### Verification (v0.6.6)

- `cargo test`: 109/109 passed (up from 86 in v0.6.0).
- `cargo clippy -- -D warnings`: 0 warnings (3 fixes applied).
- `npm run typecheck`: 0 errors.
- `npm run build:web`: passed (Vite chunk-size warning only).

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
- `MftStage` in `platform/windows_mft.rs` 鈥?compiled behind `mft-scanner` feature flag, not yet wired.

### Verification

- `cargo test`: 86/86 passed (up from 81).
- `cargo clippy -- -D warnings`: 0 warnings.
- `npm run typecheck`: 0 errors.
- `npm run build:web`: passed (Vite chunk-size warning only).
- `npm run tauri build`: Windows MSI + NSIS generated.
- Linux cross-compilation blocked by GTK sysroot on Windows dev machine 鈥?CI matrix handles native Linux builds.

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

Transform DiskPulse from a monitoring & cleanup tool into an extensible disk intelligence platform 鈥?with plugin-style architecture, multi-dimensional space analysis, and guided optimization.

#### Planned Versions

| Version | Focus | Key Features |
|---------|-------|-------------|
| v0.3.1 | i18n | `react-i18next`, en/zh-CN locales, language setting |
| v0.3.2 | Themes | CSS variable tokens, Light/Dark themes, ThemeProvider |
| v0.3.3 | Performance | jwalk, streaming scan, incremental update, ScanStage trait, memory < 100MB |
| v0.3.4 | Duplicates | 3-phase detection (size鈫?KB鈫扴HA-256), DuplicateFinder, cleanup integration |
| v0.3.5 | Aging | 7 aging buckets, zombie finder, growth hotspots, ECharts stacked bar |
| v0.3.6 | Recommendations | Weighted scoring model, disk health gauge, RecommendationCard |
| v0.3.7 | Rules + Export | RiskRule trait + registry, custom rule editor, CSV/JSON export |
| v0.3.8 | Wizard + Notify | 5-step CleanupWizard, NotificationCenter with SQLite storage |
| v0.3.9 | CLI + Platform | 5 subcommands (scan/duplicates/health/clean/export), cross-platform traits |
| v0.4.0 | Release | Integration tests, benchmarks, MSI + NSIS, docs |

**Extensibility Architecture (6 extension points):**
1. Risk Rule Registry (`trait RiskRule`) 鈥?new rules without touching core
2. Scanner Pipeline (`trait ScanStage`) 鈥?new scan types as plugins
3. Notification Channel (`trait NotifyChannel`) 鈥?Slack, Email, etc.
4. Cleanup Provider (`trait CleanupProvider`) 鈥?per-platform implementations
5. i18n Resource Bundle (JSON) 鈥?new language = new JSON file
6. Theme Token System (CSS variables) 鈥?new theme = new variable set

### Known Issues 鈥?Resolved in v0.5.0

| # | Issue | Resolution | Priority |
|---|-------|------------|----------|
| 1 | `RecommendationInput.age_days` always `None` | 鉁?Wired aging analysis into `get_recommendations()` | 馃敶 鈫?鉁?|
| 2 | `get_disk_health()` passes hardcoded `0` for duplicate/zombie data | 鉁?Full health check now scans duplicates + aging | 馃敶 鈫?鉁?|
| 3 | CLI `export` subcommand hardcodes `"C"` drive | 鉁?Added `drive` field to `CliCommand::Export` | 馃煛 鈫?鉁?|
| 4 | Scoring weights and `min_size` constants are magic numbers | 鉁?7 new `AppSettings` fields + Settings UI | 馃煛 鈫?鉁?|
| 5 | `CleanupWizard` + `NotificationCenter` are UI shells | 鉁?5-step wizard + real-time polling + badge | 馃煛 鈫?鉁?|

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

## [0.3.0] 鈥?2026-05-31

### Production Release

- Bumped app/package versions to 0.3.0 across npm, Cargo, Cargo.lock, and Tauri config.
- Polished auto-cleanup settings integration so scheduler changes are applied immediately after saving, without requiring app restart.
- Added scheduler cancellation/re-apply path to prevent stale auto-cleanup threads after settings changes.
- Verified release smoke: cargo check, cargo test (56/56), cargo clippy, npm typecheck, web build, release exe launch, and Tauri bundle build.
- Generated release artifacts: `DiskPulse_0.3.0_x64_en-US.msi` and `DiskPulse_0.3.0_x64-setup.exe`.

**Artifacts:**
- MSI SHA256: `48F124C83A1FCCCE9C175B6A5778FBCCB1E3433CABCD917134035C85F53208E4`
- NSIS SHA256: `55589EED8D6BAABE393AB29AED081FB185CC07D7E4A46EA02F9826E65DCED094`

## [0.2.9] 鈥?2026-05-31

### Auto-Cleanup 鈥?Frontend

- Added Automation settings tab with enable toggle, frequency, run time, minimum-free-space threshold, LOW-only safety copy, Save Automation, and Run Now actions.
- Added `AutoCleanupStatus` dashboard card backed by `get_auto_cleanup_status`, `run_auto_cleanup_now`, and auto-cleanup scheduler events.
- Added dashboard toast handling for `auto-cleanup-complete` and `auto-cleanup-scheduled` events.
- Added auto-cleanup report timeline to History via `get_auto_cleanup_history`.
- Kept the frontend aligned with the backend safety invariant: automatic cleanup is locked to LOW-risk, whitelisted candidates and still uses Recycle Bin cleanup.
- Verified `cargo check`, `cargo test` (56/56), `cargo clippy -- -D warnings`, `npm run typecheck`, and `npm run build:web` (chunk-size warning only).

**Next:**
- [0.3.0] 鈥?Production release: integration polish, build verified, MSI + NSIS

## [0.2.8] 鈥?2026-05-31

### Auto-Cleanup 鈥?Backend

- Added `scheduler` Rust module with schedule calculation, status model, run-now orchestration, and scheduler thread startup.
- Added `auto_cleanup_reports` SQLite table plus save/query CRUD.
- Added 5 persisted `AppSettings` fields for auto-cleanup configuration.
- Added `run_auto_cleanup_now`, `get_auto_cleanup_status`, and `get_auto_cleanup_history` Tauri commands.
- Added `auto-cleanup-complete` and `auto-cleanup-scheduled` event emission.
- Enforced safety invariant: automatic cleanup only includes LOW-risk safe candidates and still uses the existing Recycle Bin cleanup pipeline.
- Added 5 tests covering schedule calculation, LOW-risk filtering, DB report CRUD, and settings round-trip/defaults.

**Next:**
- [0.2.9] 鈥?Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] 鈥?Production release: integration polish, build verified, MSI + NSIS

## [0.2.7] 鈥?2026-05-31

### Large File Finder 鈥?Frontend

- Added `useLargeFileFinder` hook for `find_large_files`, `large-file-progress`, and cancellation lifecycle.
- Added `LargeFileFinder` UI with drive selector, minimum-size filter, result limit, scan progress, and sortable table.
- Added "Large Files" sidebar navigation entry.
- Added selected-file handoff into `CleanupPreview` via `additionalItems`, keeping the existing whitelist safety pipeline.
- Verified manual C: scan for files over 500MB: 6 files found in 76 seconds.

**Next:**
- [0.2.9] 鈥?Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] 鈥?Production release: integration polish, build verified, MSI + NSIS

## [0.2.6] 鈥?2026-05-31

### Large File Finder 鈥?Backend

- Added `FileEntry` and `LargeFileProgress` shared backend models.
- Added large-file scanner using `walkdir` plus a bounded `BinaryHeap<Reverse<FileEntry>>` top-N selection.
- Added `large-file-progress` IPC event emission during scans.
- Added `find_large_files` and `cancel_large_file_scan` Tauri commands.
- Added frontend TypeScript types for the upcoming v0.2.7 UI hook/component.
- Added 3 scanner tests covering top-N ordering, min-size filtering, and cancellation.

**Next:**
- [0.2.8] 鈥?Auto-Cleanup: Backend scheduler (scheduler module, DB table, commands, tests)
- [0.2.9] 鈥?Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] 鈥?Production release: integration polish, build verified, MSI + NSIS

## [0.2.5] 鈥?2026-05-07

### Intelligent Insights 鈥?Alerts & Prediction

> Full plan: `docs/v0.3.0-plan.md`

**Sprint 1 鈥?Disk Space Alerts:**
- Disk space alert monitor 鈥?background thread with configurable check interval
- Low space threshold notification via tauri-plugin-notification (percentage or absolute GB)
- Sudden growth detection with configurable time window and growth percent
- New `alert` Rust module with `AlertConfig`, threshold checks, 4 unit tests
- Settings UI: new "Alerts" tab 鈥?enable/disable, threshold type/value, growth params
- Dashboard: in-app alert toast banner with auto-dismiss
- 6 new `AppSettings` fields for alert configuration

**Sprint 3 鈥?Disk Usage Prediction:**
- New `prediction` Rust module with simple OLS linear regression over SQLite snapshots
- `predict_disk_usage` IPC command returning forecast status, confidence, growth rate, and projected 95% date
- Dashboard prediction card between drive ring and treemap
- History trend chart extended with dashed forecast line and forecast summary
- 3 unit tests for date parsing, growth projection, and insufficient-history behavior

**Upcoming:**
- [0.2.8] 鈥?Auto-Cleanup: Backend scheduler (scheduler module, DB table, commands, tests)
- [0.2.9] 鈥?Auto-Cleanup: Frontend UI (settings tab, status card, history)
- [0.3.0] 鈥?Production release: integration polish, build verified, MSI + NSIS

## [0.2.0] 鈥?2026-05-07

### Performance & UX Optimization

**Completed**:
- Split scan: `scan_drive_meta` (<50ms) + `scan_drive_dirs` (background) commands
- `useDriveScan` lazy loading hook with request cancellation
- Rayon-parallel top-level directory scanning with incremental `partial_results`
- Phase-based scan progress (Walking 鈫?Measuring 鈫?Complete)
- SQLite `DriveMeta` caching with freshness badges (Live / Cached / Metadata)
- Skeleton treemap placeholder during background scan
- `cancel_scan` command with AtomicBool cancellation token + UI cancel button
- Watcher cache refresh: detect FS changes 鈫?selective dirty top-level directory re-scan 鈫?refreshed treemap cache event

**Deferred**:
- jwalk parallel walkdir evaluation (optional)

## [0.0.9] 鈥?2026-05-05

### Added
- Settings page with General/Rules/About tabs
- General preferences (default drive, auto-scan, auto-monitor, watcher params)
- Risk rules configuration with search, filter, and safe-to-delete toggle
- About page with version info and tech stack grid
- Settings persistence via SQLite (key-value store)

## [0.1.0] 鈥?2026-05-06

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
- Pre-delete validation pipeline: whitelist check 鈫?system path block 鈫?runtime lock check
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

## [0.0.8] 鈥?2026-04-30

### Added
- SQLite database module (snapshots, cleanup_logs tables)
- History page with ECharts trend chart
- Snapshot history table with expandable details
- Cleanup timeline with per-item expansion
- Auto-save on scan and cleanup operations

## [0.0.7] 鈥?2026-04-30

### Added
- Real-time file system watcher (polling-based)
- Live monitoring UI with event feed
- System tray icon with menu (quick scan, pause, exit)
- Chinese README (README_zh-CN.md)

## [0.0.6] 鈥?2026-04-29

### Added
- Safe cleanup engine with Recycle Bin integration
- Cleanup preview with whitelist validation
- Confirmation & progress modals
- Undo/restore from Recycle Bin
- 16 unit tests for cleaner module

## [0.0.5] 鈥?2026-04-29

### Added
- Cleanup report page with risk-grouped layout
- Search, sort, and risk-level filter controls
- HTML and CSV export functionality

## [0.0.4] 鈥?2026-04-28

### Added
- Risk classification engine with 16 default rules
- RiskReport, RiskItem, RiskRule, RiskSummary data structures
- Developer project detection heuristic

## [0.0.3] 鈥?2026-04-28

### Added
- ECharts treemap visualization
- Drill-down navigation with breadcrumb trail
- Color-coded directory categories

## [0.0.2] 鈥?2026-04-28

### Added
- Scan progress callback with current path
- Multi-drive support via GetLogicalDrives
- Unit tests for scanner module

## [0.0.1] 鈥?2026-04-28

### Added
- Initial project scaffold
- Tauri 2 + React 19 + TypeScript 5 architecture
- Disk scanner with Win32 GetDiskFreeSpaceExW
- Aurora design system with CSS custom properties
- SVG ring chart + top-20 directory bar chart
