# XiaomiMiMo-TUI v0.5.3

This release improves first-run onboarding and adds MiMo-V2.5-TTS speech synthesis support.

## Changes

- Added a first-run onboarding step to create or select a workspace folder.
- Users can type or paste a folder path; pressing Enter creates or uses that folder.
- The default workspace is `XiaomiMiMo-Workspace` under the user's home directory.
- After selecting a workspace, the TUI refreshes its workspace, shell default directory, skills directory, and engine session context.
- Fixed the onboarding API Key input order so Ctrl+V paste is handled before normal character input.
- Added `xiaomimimo speech` with `tts` alias for non-streaming speech synthesis.
- Speech supports built-in voices (`mimo-v2.5-tts`), voice design (`mimo-v2.5-tts-voicedesign`), and voice clone (`mimo-v2.5-tts-voiceclone`).
- Voice clone accepts `.mp3` and `.wav` samples and sends them as `data:audio/...;base64,...` voice payloads.

## Suggestions

- Normal users can run `xiaomimimo.exe` or `xiaomimimo-tui.exe` and follow the startup guide.
- Use a dedicated folder per project. Avoid using Downloads, Desktop root, or drive root as the long-term workspace.

## Windows assets

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

Legacy alias assets (`xiaomimimo.exe`, `xiaomimimo-tui.exe`) are intentionally omitted to avoid duplicate release resources.
