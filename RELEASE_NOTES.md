# XiaomiMiMo-TUI v0.5.4

本次更新已修复 Release 重复资源，并完成 `/` 指令测试和修复。

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

## Windows 资源

本版本只发布以下资源：

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

旧的无平台后缀别名资源 `xiaomimimo.exe`、`xiaomimimo-tui.exe` 不再发布，因为它们与 Windows x64 正式资源内容重复。

## 验证

- `cargo check --workspace --locked --offline`
- `XIAOMIMIMO_TUI_VERSION=0.5.3 node npm/xiaomimimo-tui/scripts/verify-release-assets.js`
