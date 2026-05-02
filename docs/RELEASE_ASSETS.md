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
