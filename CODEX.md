# CODEX.md — DiskPulse Implementation Guide

> **Your role**: Implementer — execute assigned development tasks following this project's conventions.
> **Planner**: Claude Code (CLAUDE.md) — owns architecture, roadmap, code review, and release.
> **Sync order**: Read this file → `CLAUDE.md` (for full API reference) → `PROGRESS.md` (for version status).

## Project Identity

- **DiskPulse**: Real-time disk space monitor & safe cleanup tool for Windows 11
- **Stack**: Tauri 2 + Rust 1.94+ backend, React 19 + TypeScript 5 + Tailwind CSS 4 frontend
- **Repo**: `E:\Github Project\DiskPulse`
- **Current version**: v0.3.0 (production release, 56 tests)

## Current Task

> **Status**: ⏳ Awaiting assignment
> **Target version**: TBD
> **Branch**: TBD
> **Deadline**: TBD

<!-- TASK TEMPLATE (fill in when assigning):
### v0.X.Y — Feature Name

**Backend tasks** (Rust):
- [ ] ...

**Frontend tasks** (TypeScript):
- [ ] ...

**Verification**:
- [ ] `cargo test` (all passing)
- [ ] `cargo clippy -- -D warnings` (0 warnings)
- [ ] `npm run typecheck` (0 errors)
- [ ] `npm run build:web` (success)
- [ ] Manual smoke test
-->

## Development Conventions

### Before writing ANY code
1. Read `CLAUDE.md` sections: Tech Stack, Architecture Overview, Critical Safety Rules, Risk Classification System
2. Check `PROGRESS.md` for the current file inventory and test counts
3. Read the existing code in the module you're modifying — match its style, naming, and comment density

### Rust (src-tauri/src/)
```
Module map:
  main.rs          — entry point
  lib.rs           — 26 Tauri IPC commands, tray, auto-startup, event constants
  scanner/mod.rs   — walkdir + rayon parallel scan, large file finder
  risk/mod.rs      — 16 risk rules, classification engine
  cleaner/mod.rs   — Recycle Bin cleanup, undo via $I file parsing
  watcher/mod.rs   — polling FS monitor, snapshot diff
  db/mod.rs        — SQLite (rusqlite): snapshots, cleanup logs, settings, auto-cleanup reports
  alert/mod.rs     — disk space alert monitor, threshold checks, notifications
  prediction/mod.rs — OLS linear regression, forecast computation (zero new deps)
  scheduler/mod.rs — auto-cleanup scheduler, LOW-risk filtering, cancellable sleep
```

**Rules**:
- `rustfmt` + `clippy` must pass, **0 warnings**
- No `unwrap()` in production code — use `?`, `ok_or_else`, or `match`
- New Tauri commands register in `lib.rs` → `generate_handler![]`
- New IPC events: define `pub const` in the appropriate module, emit via `app.emit()`
- Tests: `#[cfg(test)] mod tests { ... }` at the bottom of each module
- Use `anyhow::Result` or `Result<T, String>` for fallible functions

### TypeScript (src/)
```
Module map:
  App.tsx                            — main app: sidebar, routing, dashboard, event listeners
  types.ts                           — all shared TypeScript interfaces
  index.css                          — Aurora design system (CSS custom properties + Tailwind)
  components/Treemap.tsx             — D3/ECharts treemap visualization
  components/CleanupPreview.tsx      — cleanup safety check, execute, undo
  components/PredictionCard.tsx      — disk usage forecast card
  components/LargeFileFinder.tsx     — large file scanner UI + sortable table
  components/AutoCleanupStatus.tsx   — auto-cleanup scheduler status card
  components/Icons.tsx               — shared SVG nav icons
  pages/Cleanup/index.tsx            — risk-grouped cleanup report
  pages/History/index.tsx            — trend chart, snapshot table, cleanup timeline
  pages/Settings/index.tsx           — General / Rules / Alerts / Automation tabs
  hooks/useDriveScan.ts              — lazy scan: meta → cache → background → cancel
  hooks/useFsEvents.ts               — FS watcher lifecycle
  hooks/useLargeFileFinder.ts        — large file scan lifecycle
  utils/format.ts                    — byte formatting
```

**Rules**:
- Strict mode, no `any` types
- Use `invoke<T>()` with explicit type parameter for all Tauri commands
- New types go in `types.ts`
- Event listeners: use `listen<T>()` from `@tauri-apps/api/event`
- Match existing component patterns: glass-card, aurora color tokens, Tailwind utilities
- Loading / empty / error states for every data-fetching component

### Safety (NEVER violate)
1. **All deletes → Recycle Bin** — `SHFileOperationW` with `FOF_ALLOWUNDO`
2. **Whitelist-only cleanup** — only paths matching `is_path_allowed()` patterns
3. **System path protection** — `C:\Windows`, `Program Files`, `System32`, `WinSxS` blocked
4. **File lock detection** — skip locked files, never force-delete
5. **Auto-cleanup invariant** — only LOW risk, `safe_to_delete==true` items

## Verification Checklist (run before marking task done)

```bash
# Rust
cd src-tauri
cargo test          # ALL must pass
cargo clippy -- -D warnings  # ZERO warnings

# TypeScript
npm run typecheck   # ZERO errors
npm run build:web   # must succeed

# Manual (when applicable)
# - Launch app: npm run tauri dev
# - Smoke test the changed feature
```

## Git

- **Branch**: `feature/v0.X.Y-description` from `master`
- **Commit**: `feat:` / `fix:` / `refactor:` / `docs:` / `chore:` prefix
- **Do NOT commit**: `.claude/settings.local.json`, `production/` directory

## Reference

| Doc | What it's for |
|-----|---------------|
| `CLAUDE.md` | Full architecture, IPC API reference, safety rules, environment |
| `PROGRESS.md` | Version history, file inventory, test counts, release checklists |
| `CHANGELOG.md` | Human-readable changelog for each version |
| `docs/v0.3.0-plan.md` | Sprint-by-sprint plan for v0.2.5–v0.3.0 |
| `README.md` | Project overview, quick start, feature list |
