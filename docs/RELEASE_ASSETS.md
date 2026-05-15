# Release assets

Windows releases publish only canonical, platform-qualified assets:

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

Do not upload the legacy unqualified aliases `xiaomimimo.exe` or
`xiaomimimo-tui.exe`. They contain the same bytes as the platform-qualified
Windows x64 binaries and create duplicate GitHub Release resources.

## Historical cleanup

The following historical releases were audited on 2026-05-02:

| Release | Duplicate assets found | Expected action |
| --- | --- | --- |
| `v0.5.0` | none | no change |
| `v0.5.1` | none | no change |
| `v0.5.2` | `xiaomimimo.exe`, `xiaomimimo-tui.exe` | deleted duplicate aliases and re-uploaded a canonical-only checksum manifest |
| `v0.5.3` | `xiaomimimo.exe`, `xiaomimimo-tui.exe` | deleted duplicate aliases and re-uploaded a canonical-only checksum manifest |

`npm/xiaomimimo-tui/scripts/verify-release-assets.js` now also checks that
legacy alias assets are absent and that the checksum manifest does not contain
duplicate hashes.

## Install mirrors and verification

The npm installer downloads binaries from the release matching the package
version and verifies every downloaded asset against
`xiaomimimo-artifacts-sha256.txt` before it is promoted into `bin/downloads`.

Set `XIAOMIMIMO_TUI_RELEASE_MIRROR` (or `XIAOMIMIMO_RELEASE_MIRROR`) to point
the installer at a mirror directory that contains the same canonical asset
names and checksum manifest. The older `*_RELEASE_BASE_URL` variables remain
supported for compatibility.

`postinstall` is optional and bounded by `XIAOMIMIMO_TUI_DOWNLOAD_TIMEOUT_MS`
or `XIAOMIMIMO_DOWNLOAD_TIMEOUT_MS` (default: 30000 ms). A timeout or mirror
failure prints a warning and lets npm finish; direct runtime use retries the
same verified download path.

Release CI validates that the git tag (`vX.Y.Z`) matches both the Rust package
version and the npm package version before uploading artifacts.
