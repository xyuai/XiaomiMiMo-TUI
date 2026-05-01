# xiaomimimo-tui

npm wrapper for **XiaomiMiMo-TUI**.

This package installs and runs the `xiaomimimo` and `xiaomimimo-tui` binaries from GitHub release artifacts.

Project repository: [xyuai/XiaomiMiMo-TUI](https://github.com/xyuai/XiaomiMiMo-TUI)

## Install

```bash
npm install -g xiaomimimo-tui
# or
pnpm add -g xiaomimimo-tui
```

For project-local usage:

```bash
npm install xiaomimimo-tui
npx xiaomimimo-tui --help
```

`postinstall` downloads platform binaries into `bin/downloads/` and exposes `xiaomimimo` and `xiaomimimo-tui` commands.

## First run

```bash
xiaomimimo login --api-key "YOUR_XIAOMIMIMO_API_KEY"
xiaomimimo doctor
xiaomimimo
```

The `xiaomimimo` facade and `xiaomimimo-tui` binary share `~/.xiaomimimo/config.toml` for Xiaomi MiMo auth and default model settings.

The app talks to Xiaomi MiMo's OpenAI-compatible Chat Completions API. Set `XIAOMIMIMO_BASE_URL` when you need to override the default endpoint.

## Supported platforms

- Linux x64
- macOS x64 / arm64
- Windows x64

Other platform/architecture combinations are not supported and will fail during install.

## Configuration

- Default binary version comes from `xiaomimimoBinaryVersion` in `package.json`.
- Set `XIAOMIMIMO_TUI_VERSION` or `XIAOMIMIMO_VERSION` to override the release version.
- Set `XIAOMIMIMO_TUI_GITHUB_REPO` or `XIAOMIMIMO_GITHUB_REPO` to override the source repo; default: `xyuai/XiaomiMiMo-TUI`.
- Set `XIAOMIMIMO_TUI_FORCE_DOWNLOAD=1` to force download even when the cached binary is already present.
- Set `XIAOMIMIMO_TUI_DISABLE_INSTALL=1` to skip install-time download.

## Release integrity

- `npm publish` runs a release-asset check to ensure all required binary assets exist for the target GitHub release before publishing.
- Install-time downloads are verified against the release checksum manifest before the wrapper marks them executable.
- Set `XIAOMIMIMO_TUI_RELEASE_BASE_URL` to point the installer at a local or staged release-asset directory for smoke tests.

## Notice

XiaomiMiMo-TUI was originally adapted from [deepseektui](https://github.com/Hmbown/DeepSeek-TUI), licensed under the MIT License.
