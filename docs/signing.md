# DiskPulse Signing Guide

This document tracks the v0.7.1 signing and distribution setup.

## Goals

- Windows: use SignPath Foundation for free open-source code signing.
- macOS: prepare a Homebrew Cask for free distribution while Apple notarization remains optional.
- CI: keep pull requests fast and unsigned; submit signing requests only for `v*` tags or `release/**` branches.

## SignPath Foundation

1. Apply for SignPath Foundation with the public repository `Nagi-226/DiskPulse`.
2. Create the SignPath project with:
   - Project slug: `diskpulse`
   - Artifact configuration slug: `windows-installers`
   - Signing policy slug: `release-signing`
3. Mirror the repository config from `.signpath/config.yml` and `.signpath/policies/diskpulse/release-signing.yml`.
4. Enable GitHub Actions as the trusted build system.
5. Require GitHub-hosted runners and disallow reruns for release signing.

## Required GitHub Secrets

Set these secrets in the repository before publishing signed Windows artifacts:

| Secret | Purpose |
|--------|---------|
| `SIGNPATH_API_TOKEN` | API token used by `signpath/github-action-submit-signing-request@v2`. |
| `SIGNPATH_ORGANIZATION_ID` | SignPath organization/customer identifier. |

Unsigned Windows artifacts are still uploaded as `diskpulse-windows`; signed outputs are uploaded as `diskpulse-windows-signed` when signing succeeds.

## CI Flow

1. `npm run tauri build` creates MSI and NSIS installers.
2. Windows release/tag builds upload `diskpulse-windows-unsigned`.
3. The SignPath GitHub action submits a signing request for that artifact.
4. The workflow waits for completion and uploads `diskpulse-windows-signed`.
5. Release notes should attach signed artifacts first, with unsigned artifacts only as fallback.

## Homebrew Cask

The initial formula lives at `packaging/homebrew/diskpulse.rb`.

Before submitting upstream:

1. Build a macOS `.dmg` on `macos-latest`.
2. Attach it to the GitHub release as `DiskPulse_0.7.0_x64.dmg`.
3. Replace `sha256 :no_check` with the real SHA-256 hash.
4. If no Apple Developer ID is available, keep the unsigned build caveat.
5. Submit a PR to Homebrew Cask once the release URL is public.

## Local Verification

Run:

```bash
npm run verify:signing
```

This checks that the SignPath config, trusted-build policy, GitHub Actions wiring, Homebrew Cask template, and this document all stay in sync.
