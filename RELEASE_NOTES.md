# XiaomiMiMo-TUI v0.6.0

## 更新内容

- 加强 `fetch_url` 安全校验：拦截本地、私网、链路本地、组播、云 metadata 等地址，并固定 DNS 解析结果，降低 DNS rebinding 风险。
- 修复 Skills 展示路径：使用实际 `SKILL.md` 所在目录，支持目录名与 skill name 不一致的情况。
- 配置、设置、会话、checkpoint、offline queue 和 MCP 状态保存改为原子写入，降低异常退出导致文件损坏的概率。
- 增加后台任务监督：engine、任务 worker、自动化 scheduler 等异步任务 panic 时会记录 crash dump。
- 增强 `/config`：支持查看当前配置项，并支持在会话内临时修改配置。
- 新增 `/lsp`、`/cache`、`/profile` 命令：可查看/切换 LSP 开关、查看最近 token/cache 遥测、快速切换配置 profile。
- 增强 `/undo`：优先回滚最近一次文件修改快照，无可用快照时保留原有对话撤销行为。
- 优化 MCP 与 session 状态保存流程，提高运行时稳定性。

## Windows 资源

本版本在线发布以下 Windows x64 资源：

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

## 验证

- `cargo check --workspace --locked --offline`
- `cargo test --workspace --locked --offline patch_undo_restores_latest_tool_snapshot`
- `cargo test --workspace --locked --offline execute_lsp_cache_and_profile_dispatch`
- `cargo test --workspace --locked --offline test_cache_shows_latest_turn_telemetry`
