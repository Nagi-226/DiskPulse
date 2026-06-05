# DiskPulse Linux Native CI

DiskPulse v0.7.2 promotes Linux from cross-platform compile coverage to a native
GitHub Actions target. The CI matrix now runs on `ubuntu-latest` and exercises
the same core validation path as Windows and macOS before publishing Linux
bundles.

## CI Coverage

The `ubuntu-latest` job runs:

- `cargo test`
- `cargo clippy -- -D warnings`
- `npm run typecheck`
- `npm run build:web`
- `npm run tauri build`

After the Tauri build, the workflow verifies that both Linux distributables were
created:

- `.deb` under `src-tauri/target/release/bundle/deb/`
- `.AppImage` under `src-tauri/target/release/bundle/appimage/`

The Linux artifact upload is configured with `if-no-files-found: error` and a
14-day retention window so missing packages fail CI instead of producing a green
build with no installer.

## Ubuntu Dependencies

The Linux runner installs the native libraries required by Tauri, WebKitGTK,
OpenSSL, and AppImage validation:

- `libwebkit2gtk-4.1-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`
- `patchelf`
- `libssl-dev`
- `pkg-config`
- `libfuse2`

## Platform Notes

- File metadata uses Unix `stat`/`MetadataExt` values (`blocks`, `dev`, `ino`),
  so there is no hard `statx` dependency for v0.7.2.
- Linux cleanup now prefers `trash-rs` and keeps `gio trash` as a fallback for
  desktop environments where the crate path fails.
- The inotify FFI parser has a Linux-only unit test that covers multiple records
  in one kernel buffer, including directory delete flags.

## Release Status

v0.7.2 is locally complete once `npm run verify:linux-ci` passes. Full native
confidence still requires a GitHub Actions run on `ubuntu-latest` because the
local Windows workstation cannot execute the Linux-only inotify test or produce
the real `.deb` and `.AppImage` artifacts.
