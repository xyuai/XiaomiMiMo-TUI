# XiaomiMiMo-TUI v0.5.2

本版本主要修复 Windows Release 用户从“下载/Downloads”等大目录直接启动后，发送消息长期无回复的问题。

## 修复内容

- 默认关闭工作区 side-git snapshots，避免首次发送消息前对整个 Downloads 目录执行 `git add -A` 导致卡住。
- 新建配置文件会自动写入 `[snapshots] enabled = false`；如需要 `/restore` 快照能力，可在项目目录中手动开启。
- `xiaomimimo-windows-x64.exe` 现在可直接识别同目录的 `xiaomimimo-tui-windows-x64.exe`，不再必须手动改名为 `xiaomimimo-tui.exe`。
- Windows Release 同时附带短文件名 `xiaomimimo.exe` 与 `xiaomimimo-tui.exe`，方便直接运行。

## 使用建议

- 普通聊天：直接运行 `xiaomimimo.exe` 或 `xiaomimimo-tui.exe`。
- 若在大目录中启动，建议保持 snapshots 关闭。
- 若需要项目快照恢复功能，请进入项目文件夹后在配置中开启：

```toml
[snapshots]
enabled = true
```

## Windows 资产

- `xiaomimimo.exe`
- `xiaomimimo-tui.exe`
- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`
