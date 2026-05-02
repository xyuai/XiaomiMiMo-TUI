# Codex Task Log

- Started: 2026-05-02 15:06:37 +08:00
- Repo: E:\codex2\XiaomiMiMo-TUI-github
- Task: Fix duplicate release assets, prevent future duplicates, enumerate/test slash commands, push updates.
- Resume hint: continue from this file and current git status.

## Initial state

```
## main...origin/main

1f5e1c9 (HEAD -> main, tag: v0.5.3, origin/main) Fix interactive TuiOptions workspace flag

```
2026-05-02 15:16:03 +08:00 - Audited GitHub releases: v0.5.2 and v0.5.3 contain duplicate alias assets (xiaomimimo.exe and xiaomimimo-tui.exe) with identical hashes/sizes to canonical *-windows-x64.exe assets. v0.5.0 and v0.5.1 do not contain alias duplicates.
2026-05-02 15:27:18 +08:00 - Implemented release workflow change to only publish platform-qualified Windows assets and added release-asset verification for absence of legacy duplicates.
2026-05-02 15:27:18 +08:00 - Added slash-command registry smoke test coverage and documented command audit/fixes.
2026-05-02 16:48:20 +08:00 - Historical release cleanup completed: removed legacy duplicate assets xiaomimimo.exe and xiaomimimo-tui.exe from v0.5.2 and v0.5.3, removed stale checksum manifests, and uploaded canonical-only manifests. Verified v0.5.0-v0.5.3 now expose only canonical Windows assets plus manifest.
2026-05-02 16:48:20 +08:00 - Local fixes retained: release workflow guard prevents future legacy alias uploads; npm verifier rejects legacy aliases, duplicate asset names, and duplicate checksums; slash-command registry duplicate /context removed; smoke-test coverage added for every registered / command; /save and /export now resolve relative/default paths inside workspace; /attach home expansion uses platform home resolution; restore test options updated.
2026-05-02 16:48:20 +08:00 - Verification checkpoint before final full rerun: cargo test -p xiaomimimo-tui commands:: passed; node release verifier passed for v0.5.3 and XIAOMIMIMO_TUI_VERSION=0.5.2 after cleanup (GitHub API fallback handles transient asset download timeouts). Full workspace test rerun pending after config redaction expectation fix.
2026-05-02 17:03:47 +08:00 - Full verification completed: cargo test --workspace --locked passed; node npm/xiaomimimo-tui/scripts/verify-release-assets.js passed for v0.5.3; XIAOMIMIMO_TUI_VERSION=0.5.2 verifier passed. Additional unrelated failing tests found during full run were fixed: CLI provider parser now accepts xiaomimimo alias, capacity prior normalization checks flash aliases before pro aliases, file-mention completion handles directory-part partials, .env.example removed an unwired Anthropic-only key, and config redaction test expectation now matches implementation.
