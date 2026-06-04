# DiskPulse v0.7.0 Release Notes

Release date: 2026-06-04

## Highlights

- Streaming incremental scans with batch updates and cancellation.
- Custom cleanup rules with live pattern testing.
- NTFS MFT direct-scan reserve path with exact scanner fallback.
- Windows Service mode for headless background monitoring.
- Holt-Winters + Modified Z-Score anomaly detection.
- Smart Recommendations v2 with urgency, behavior learning, and 4D health radar.
- Multi-device Dashboard with local WebSocket Hub, mDNS discovery, 6-digit pairing, and remote read-only scan routing.

## Verification

- `cargo test --manifest-path src-tauri\Cargo.toml`: 119/119 passed.
- `cargo clippy --manifest-path src-tauri\Cargo.toml -- -D warnings`: passed.
- `npm run typecheck`: passed.
- `npm run build:web`: passed with the existing Vite chunk-size warning.
- `npm run tauri build`: passed on Windows and produced MSI + NSIS bundles.

## Windows Artifacts

| Artifact | SHA-256 |
|----------|---------|
| `src-tauri\target\release\bundle\msi\DiskPulse_0.7.0_x64_en-US.msi` | `49C6B8ED4C17644FCD5DF811DFD6BDD91C21B799FC8B89E97BCD6445E30AC814` |
| `src-tauri\target\release\bundle\nsis\DiskPulse_0.7.0_x64-setup.exe` | `CDE2529CFACD7D49D77F83C9615EC02C4DE138B36C00FED0B4A938FA79820411` |

## Known Follow-ups

- Validate Linux `.deb` / `.AppImage` and macOS `.dmg` on native CI runners.
- Add code signing and notarization before public distribution.
- Consider frontend code-splitting to remove the Vite chunk-size warning.
