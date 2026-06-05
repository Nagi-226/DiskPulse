# M1 Release Readiness

Status date: 2026-06-05

Scope: v0.8.1 SignPath approval + Windows signing, and v0.8.2 Linux native runner validation.

## Local Readiness

| Track | Local status | Evidence |
|-------|--------------|----------|
| v0.8.1 Windows signing | Ready for external SignPath approval | `.github/workflows/ci.yml` uploads unsigned Windows installers, submits SignPath, verifies signed MSI/EXE output, and uploads signed artifacts with `if-no-files-found: error`. |
| v0.8.2 Linux native runner | Ready for GitHub Actions native runner validation | `.github/workflows/ci.yml` installs Ubuntu GTK/WebKit/FUSE dependencies and verifies both `.deb` and `.AppImage` bundles. |

## External Gates

These items cannot be completed locally:

- SignPath Foundation OSS approval is external and remains pending until the application is approved.
- GitHub repository secrets (`SIGNPATH_API_TOKEN`, `SIGNPATH_ORGANIZATION_ID`) must be configured after approval.
- The release-tag signing webhook must run on GitHub Actions to produce signed Windows installers.
- Linux native runner validation is pending until `ubuntu-latest` runs `cargo test` and `npm run tauri build` on GitHub Actions.

## Local Verification Commands

Run these before tagging or asking for external validation:

```powershell
npm run verify:m1-release
npm run verify:signing
npm run verify:linux-ci
```

## Promotion Criteria

- v0.8.1 can be marked externally complete only after SignPath returns signed MSI/NSIS artifacts and the signed artifact verification step passes.
- v0.8.2 can be marked externally complete only after the `ubuntu-latest` GitHub Actions job passes `cargo test`, builds Tauri, and uploads `.deb` and `.AppImage` artifacts.
- Until both external gates pass, project status is: local readiness complete; external/native validation pending.
