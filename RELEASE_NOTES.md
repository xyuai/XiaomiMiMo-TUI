# XiaomiMiMo-TUI v0.6.6

## 更新内容

- 增强 MCP HTTP 兼容性：默认发送 JSON/SSE Accept，支持 Streamable HTTP、SSE fallback、会话 ID 持久化与 GET 预检。
- MCP discovery 支持 `nextCursor` 分页，单个异常条目不再丢弃整批结果，并保持工具、资源、提示排序稳定。
- stdio MCP 捕获 stderr 尾部，连接或读取失败时提供更清晰的诊断信息。
- HTTP MCP 支持自定义 headers，并继承 `HTTP_PROXY`、`HTTPS_PROXY`、`NO_PROXY` 代理环境配置。
- MCP 配置支持按文件变更 lazy reload，减少修改配置后必须重启的问题。

## 验证

- `cargo check --workspace`
- `cargo test -p xiaomimimo-tui mcp -- --nocapture`

---

# XiaomiMiMo-TUI v0.6.5

## 更新内容

- 强化 `fetch_url` 重定向安全：每一跳目标都会在请求前重新校验 scheme、host、IP 与网络策略。
- 持久任务默认审批边界更严格：省略审批字段不再自动授予 shell 或自动审批权限。
- 子代理保留父会话审批边界，并阻止交互式终端接管。
- shell 与 MCP 子进程启动时使用环境变量白名单，减少父进程敏感变量泄漏。

## 验证

- `rustfmt --edition 2024 --check`（v0.6.5 变更文件）
- `cargo check --workspace`
- `cargo test -p xiaomimimo-tui fetch_url -- --nocapture`
- `cargo test -p xiaomimimo-tui task_manager -- --nocapture`
- `cargo test -p xiaomimimo-tui subagent -- --nocapture`
- `cargo test -p xiaomimimo-tui child_env -- --nocapture`

---

# XiaomiMiMo-TUI v0.6.4

## 更新内容

- 提升工具调用稳定性：自动修复常见工具参数 JSON 问题，包括尾逗号、未闭合括号和字符串内控制字符。
- 清理工具输入 schema：自动处理 nullable union、空 object schema 和失效 required 字段，减少严格工具模式下的请求失败。
- 增强工具名兼容：可将常见非规范工具名自动解析到已注册工具，并保持工具列表稳定排序与缓存。
- 增加工具循环保护：对重复同参工具调用和连续失败工具调用进行提示或阻断，避免会话卡在工具重试循环。
- 强化工作区会话隔离：`--continue`、`resume --last`、`fork --last` 按当前工作区查找最近会话，checkpoint 仅在同工作区恢复。
- 改进工作区快照安全：跳过用户主目录和磁盘根目录等高风险位置，降低误扫大目录的概率。
- 改善 Windows 终端体验：启动时设置 UTF-8 控制台代码页，Windows 默认关闭鼠标捕获，并保留配置/参数手动开启能力。
- 改善粘贴和剪贴板：支持 legacy Ctrl+V 粘贴事件，剪贴板写入增加 Windows `Set-Clipboard` 与 OSC52 fallback。

## Windows 资源

本版本在线发布以下 Windows x64 资源：

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

## 验证

- `cargo fmt`
- `cargo check -p xiaomimimo-tui`
- `cargo test -p xiaomimimo-tui tools::arg_repair -- --nocapture`
- `cargo test -p xiaomimimo-tui tools::schema_sanitize -- --nocapture`
- `cargo test -p xiaomimimo-tui core::engine::loop_guard -- --nocapture`
- `cargo test -p xiaomimimo-tui mouse_capture -- --nocapture`
