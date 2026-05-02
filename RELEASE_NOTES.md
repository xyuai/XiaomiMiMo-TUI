# XiaomiMiMo-TUI v0.5.5

本次更新新增 CI 质量检查，优化语音合成默认输出路径，并补充 Windows 中文编码说明。

## 更新内容

- Release 只发布 `xiaomimimo-windows-x64.exe`、`xiaomimimo-tui-windows-x64.exe` 和校验文件。
- 不再发布 `xiaomimimo.exe`、`xiaomimimo-tui.exe`，避免重复资源。
- 已清理 `v0.5.2` 和 `v0.5.3` 的历史重复资源。
- 已新增 `/` 指令自动测试，检查重复注册、别名解析和调度。
- 已修复 `/context` 重复注册，保留 `/ctx` 别名。
- 已修复 `/save` 和 `/export` 的 workspace 路径问题。
- 已修复 `/attach` 在 Windows 下的 `~/` 路径解析。
- 已修复 CLI provider 的 `xiaomimimo` 解析。
- 已修复 flash 模型归类和文件提及补全问题。
- 已新增 CI 中的 `cargo clippy --workspace --all-targets -- -D warnings` 和 workspace 测试检查。
- 已优化语音合成输出体验：省略 `-o/--output` 时默认文件名会跟随 `--format`，并支持 `[speech].output_dir` / `XIAOMIMIMO_SPEECH_OUTPUT_DIR` 默认输出目录。
- 已补充 Windows 中文显示说明：文档使用 UTF-8，旧版 PowerShell 若乱码可使用 PowerShell 7 或 `chcp 65001`。

## Windows 资源

本版本只发布以下资源：

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

旧的无平台后缀别名资源 `xiaomimimo.exe`、`xiaomimimo-tui.exe` 不再发布，因为它们与 Windows x64 正式资源内容重复。

## 验证

- `cargo check --workspace --locked --offline`
- `XIAOMIMIMO_TUI_VERSION=0.5.3 node npm/xiaomimimo-tui/scripts/verify-release-assets.js`
