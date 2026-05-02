# xiaomimimo-tui

`xiaomimimo-tui` 是 **XiaomiMiMo-TUI** 的 npm 安装包装器，用于从 GitHub Release 下载并运行 Windows x64 预构建二进制文件。

项目仓库：<https://github.com/xyuai/XiaomiMiMo-TUI>

## 安装

```bash
npm install -g xiaomimimo-tui
```

## 使用

```bash
xiaomimimo login --api-key "YOUR_XIAOMIMIMO_API_KEY"
xiaomimimo doctor
xiaomimimo
```

该包装器会暴露两个命令：

- `xiaomimimo`
- `xiaomimimo-tui`

配置文件默认位于 `~/.xiaomimimo/config.toml`。

## 支持平台

当前预构建 Release 资产仅提供：

- Windows x64

Only the platform-qualified `xiaomimimo-windows-x64.exe` and `xiaomimimo-tui-windows-x64.exe` release assets are used. Legacy unqualified `.exe` aliases are not published because they duplicate those binaries.

## 配置

- 默认二进制版本来自 `package.json` 中的 `xiaomimimoBinaryVersion`。
- 可用 `XIAOMIMIMO_TUI_VERSION` 或 `XIAOMIMIMO_VERSION` 覆盖下载版本。
- 可用 `XIAOMIMIMO_TUI_GITHUB_REPO` 或 `XIAOMIMIMO_GITHUB_REPO` 覆盖下载仓库，默认 `xyuai/XiaomiMiMo-TUI`。
- 设置 `XIAOMIMIMO_TUI_FORCE_DOWNLOAD=1` 可强制重新下载。
- 设置 `XIAOMIMIMO_TUI_DISABLE_INSTALL=1` 可跳过安装时下载。

## English

npm wrapper for **XiaomiMiMo-TUI**. It installs and runs the `xiaomimimo` and `xiaomimimo-tui` Windows x64 binaries from GitHub release artifacts.
