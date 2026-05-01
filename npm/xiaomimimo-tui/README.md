# xiaomimimo-tui

Install and run the `xiaomimimo` and `xiaomimimo-tui` binaries from GitHub release artifacts.

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

`postinstall` downloads platform binaries into `bin/downloads/` and exposes
`xiaomimimo` and `xiaomimimo-tui` commands.

## First run

```bash
xiaomimimo login --api-key "YOUR_XIAOMIMIMO_API_KEY"
xiaomimimo doctor
xiaomimimo
```

The `xiaomimimo` facade and `xiaomimimo-tui` binary share `~/.xiaomimimo/config.toml`
for XiaomiMiMo auth and default model settings. Common TUI commands are available
directly through the facade, including `xiaomimimo doctor`, `xiaomimimo models`,
`xiaomimimo sessions`, and `xiaomimimo resume --last`.

The app talks to XiaomiMiMo's documented OpenAI-compatible Chat Completions API.
Set `XIAOMIMIMO_BASE_URL` only if you need the China endpoint or XiaomiMiMo beta
features such as strict tool mode, chat prefix completion, or FIM completion.

NVIDIA NIM-hosted XiaomiMiMo V4 Pro is also supported:

```bash
xiaomimimo auth set --provider nvidia-nim --api-key "YOUR_NVIDIA_API_KEY"
xiaomimimo --provider nvidia-nim
```

For a single process, set `XIAOMIMIMO_PROVIDER=nvidia-nim` and `NVIDIA_API_KEY`
or `NVIDIA_NIM_API_KEY` (with `XIAOMIMIMO_API_KEY` as a compatibility fallback).
The NIM default model is `mimo-v2.5-pro` and the default base URL
is `https://integrate.api.nvidia.com/v1`. With `--provider nvidia-nim`,
`--model mimo-v2-flash` maps to `mimo-v2-flash`.

## Supported platforms

- Linux x64
- macOS x64 / arm64
- Windows x64

Other platform/architecture combinations are not supported and will fail during install.

## Configuration

- Default binary version comes from `xiaomimimoBinaryVersion` in `package.json`.
- Set `XIAOMIMIMO_TUI_VERSION` or `XIAOMIMIMO_VERSION` to override the release version.
- Set `XIAOMIMIMO_TUI_GITHUB_REPO` or `XIAOMIMIMO_GITHUB_REPO` to override the source repo (defaults to `YOUR_GITHUB_USERNAME/mimotui`).
- Set `XIAOMIMIMO_TUI_FORCE_DOWNLOAD=1` to force download even when the cached binary is already present.
- Set `XIAOMIMIMO_TUI_DISABLE_INSTALL=1` to skip install-time download.

## Release integrity

- `npm publish` runs a release-asset check to ensure all required binary assets
  exist for the target GitHub release before publishing.
- Install-time downloads are verified against the release checksum manifest before
  the wrapper marks them executable.
- Set `XIAOMIMIMO_TUI_RELEASE_BASE_URL` to point the installer at a local or
  staged release-asset directory for smoke tests.
