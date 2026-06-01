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
- Current release baseline: `v0.4.0`.
- Next milestone: post-v0.4.0 maintenance / v0.5.0 planning.
- Full v0.4.0 roadmap: `docs/v0.4.0-plan.md`.
- Stack: Tauri 2, Rust 1.94+, React 19, TypeScript 5, Tailwind CSS 4, SQLite via rusqlite.
- Build target: Windows desktop installers through Tauri bundling.
- Current state from project docs: v0.4.0 production release built; MSI/NSIS generated.

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

