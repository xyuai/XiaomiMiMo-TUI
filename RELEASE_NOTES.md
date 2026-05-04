# XiaomiMiMo-TUI v0.6.2

## 更新内容

- 增强 TUI 终端恢复、键盘增强协议回退、粘贴快捷键识别和 Ctrl+H 退格兼容。
- 改进 Markdown 渲染：支持表格、水平分隔线、粗体/斜体标记处理，并修复未闭合 inline marker 的潜在循环问题。
- 加强 `exec_shell` 工作目录校验：`cwd` / `working_dir` 会解析到工作区内，拒绝越界路径，并修复前台 shell 执行忽略 cwd 的问题。
- 新增大工具输出 spillover：超过阈值的成功工具输出会保存到 `~/.xiaomimimo/tool_outputs/`，模型和 UI 只显示有界预览及完整文件路径。
- 工具详情页支持查看 spillover 完整输出，并在启动时清理 7 天前的旧工具输出文件。
- 新增 panic 恢复钩子，崩溃时尽量恢复 raw mode、alt screen、mouse capture、bracketed paste 和 Kitty keyboard flags，并写入 crash log。

## Windows 资源

本版本在线发布以下 Windows x64 资源：

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

## 验证

- `cargo test -p xiaomimimo-tui spillover -- --nocapture`
- `cargo test -p xiaomimimo-tui markdown_render -- --nocapture`
- `cargo test -p xiaomimimo-tui cwd -- --nocapture`
- `cargo test -p xiaomimimo-tui api_key_paste_shortcut_is_not_plain_text_input -- --nocapture`
- `cargo test -p xiaomimimo-tui ctrl_h_is_treated_as_terminal_backspace -- --nocapture`
- `cargo check -p xiaomimimo-tui`
