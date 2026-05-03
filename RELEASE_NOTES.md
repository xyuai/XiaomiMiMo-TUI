# XiaomiMiMo-TUI v0.6.1

## 更新内容

- 将 TUI 内各项指令返回信息改为中文，包括配置、上下文、缓存、token、会话、队列、任务、MCP、Provider、Skills 等常用命令输出。
- 将上下文检查器、右键菜单、状态提示、帮助提示等界面信息改为中文，减少中英文混用。
- 优化命令错误、用法说明、空状态提示和操作成功提示的中文表达，让返回结果更容易直接阅读。
- 保持命令名称、参数、路径、模型名、Provider 名称等技术标识原样，避免影响复制和脚本使用。
- 修复中文化后的右键菜单测试断言，确保 Windows CI 的 clippy 和完整测试通过。

## Windows 资源

本版本在线发布以下 Windows x64 资源：

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

## 验证

- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked --no-fail-fast`
