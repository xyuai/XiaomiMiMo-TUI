# XiaomiMiMo-TUI v0.7.0

## 更新内容
- 为工作区快照增加预检上限：覆盖 entry 总数、总字节数、单文件字节数与单 entry 路径长度，并在 side repo 初始化前和每次 staging 前执行。
- npm 安装下载增加超时控制；`postinstall` 变为可降级流程，失败时快速提示 warning，运行时仍可重试 verified download。
- Release 下载支持 `XIAOMIMIMO_TUI_RELEASE_MIRROR` / `XIAOMIMIMO_RELEASE_MIRROR`，并继续对下载产物执行 SHA256 校验。
- 新增 TUI build script，监听 `.git/HEAD`、branch ref、packed refs 与 GitHub release ref 环境变化。
- Release CI 在上传产物前校验 Rust 与 npm package version 是否和 `vX.Y.Z` tag 一致。

## 验证
- `cargo check -p xiaomimimo-tui`
- `cargo test -p xiaomimimo-tui snapshot::repo -- --nocapture`
- `node -c npm/xiaomimimo-tui/scripts/install.js`
- `node -e "const i=require('./npm/xiaomimimo-tui/scripts/install'); console.log(i.resolveTimeoutMs(), i.isOptionalInstall(['node','install.js','--optional']))"`
- `node -e "const a=require('./npm/xiaomimimo-tui/scripts/artifacts'); process.env.XIAOMIMIMO_TUI_RELEASE_MIRROR='https://mirror.example/releases/v0.7.0'; console.log(a.releaseBaseUrl('0.7.0'))"`
- `git diff --check`

---

# XiaomiMiMo-TUI v0.6.9

## 更新内容
- 新增工作区与全局用户 slash commands，支持 `.xiaomimimo/commands` 与 `~/.xiaomimimo/commands`；工作区命令优先并可覆盖内置命令。
- Slash 补全现在会同时显示用户命令和内置命令。
- `@` mention 补全支持显式输入的隐藏/配置路径，例如 `.xiaomimimo/...`，即使这些路径被 ignore。
- Skills 发现扩展到工作区与全局位置，并在 TUI、palette、runtime API 与 prompts 中保持工作区优先级。
- Active skill instructions 使用有界且稳定的上下文块，长 skill 文件不会挤掉关键工作区技能说明。

## 验证
- `cargo check -p xiaomimimo-tui`
- `cargo test -p xiaomimimo-tui commands -- --nocapture`
- `cargo test -p xiaomimimo-tui skills -- --nocapture`
- `cargo test -p xiaomimimo-tui working_set -- --nocapture`
- `cargo test -p xiaomimimo-tui prompts -- --nocapture`
- `git diff --check`

---

# XiaomiMiMo-TUI v0.6.8

## 更新内容
- 按 Provider 能力构造请求：`thinking`、`reasoning_content` 与工具 schema 字段会跟随当前 Provider 能力下发。
- Generic OpenAI-compatible 后端默认剥离非标准工具元数据，仅在支持时保留 `strict`。
- Fireworks 改用 OpenAI-compatible 的 `reasoning_effort`，不再发送顶层 `thinking`。
- `XIAOMIMIMO_BASE_URL` 会应用到当前 Provider 配置，避免被默认配置覆盖。
- `/provider` 与 `/model` 选择按 Provider 持久化；自定义 OpenAI-compatible 模型 ID 会按原样保留。

## 验证
- `cargo check --workspace`
- `cargo test -p xiaomimimo-tui client -- --nocapture`
- `cargo test -p xiaomimimo-tui config -- --nocapture`
- `cargo test -p xiaomimimo-tui settings -- --nocapture`
- `cargo test -p xiaomimimo-tui provider -- --nocapture`
- `git diff --check`

---

# XiaomiMiMo-TUI v0.6.7

## 更新内容
- 统一终端状态恢复：panic、Ctrl+C、SIGTERM、早退路径都会尽量恢复 raw mode、鼠标捕获、bracketed paste 与 alt screen。
- 改进 Windows 终端兼容性：Windows Terminal 可自动启用鼠标捕获，旧版控制台保持保守默认。
- 新增 `tui.composer_arrows_scroll`，输入框多行时 Up/Down 可滚动内容，Windows 场景默认更稳。
- 增加 low-motion 自动检测，兼容 `NO_ANIMATIONS`、VS Code、Ghostty、Termius、SSH、Tilix、Terminator 等环境。
- 修复 diff、pager 与 Markdown 渲染中的长 CJK、长无空格文本、表格、OSC 8 链接、粘贴和 Home/End 行为问题。

## 验证
- `cargo check --workspace`
- `cargo test -p xiaomimimo-tui settings -- --nocapture`
- `cargo test -p xiaomimimo-tui tui::diff_render -- --nocapture`
- `cargo test -p xiaomimimo-tui tui::pager -- --nocapture`
- `cargo test -p xiaomimimo-tui tui::markdown_render -- --nocapture`
- `cargo test -p xiaomimimo-tui tui::app::tests::composer_arrow_scroll_default_tracks_platform_and_mouse_capture -- --nocapture`
- `git diff --check`

---

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
