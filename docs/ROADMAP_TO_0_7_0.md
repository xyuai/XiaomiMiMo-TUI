# XiaomiMiMo-TUI 更新计划（至 v0.7.0）

> 目标：以最小、可回滚、可验证的方式吸收近期上游安全与兼容性修复，保持 XiaomiMiMo-TUI 的品牌、Provider、发布渠道与用户配置语义不变。所有发布说明、资源文件与用户面向文档不得出现外部参考项目名称。

## 总体策略

- 不做整包合并；按版本切片手工迁移或选择性移植。
- 每个版本只解决一组明确问题，完成后单独提交、打 tag、推送远端。
- 安全边界优先于功能扩展；能默认关闭的高风险能力默认关闭。
- 所有新增行为必须保留配置开关或兼容迁移路径。
- 发布前至少执行：`cargo fmt`、`cargo check`，并尽量运行相关单元测试。

## v0.6.5：安全热修

优先级：P0。

范围：

1. `fetch_url` 网络目标校验
   - 禁止自动跟随重定向；
   - 每一次 redirect 都重新校验目标 URL；
   - 加强 IPv4、IPv6、IPv4-mapped IPv6、私网、链路本地、保留地址、CGNAT、benchmark 网段检查；
   - 保持现有 allow_private_networks 配置语义。

2. 任务默认审批边界
   - 新建任务默认不自动批准；
   - 只有显式传入 auto-approve 时才启用；
   - 保持旧数据读取兼容。

3. 子代理权限边界
   - 子代理不再强制自动批准；
   - 子代理继承父会话审批状态；
   - 未自动批准时阻止需要审批的工具；
   - 禁止子代理抢占交互式终端。

4. 子进程环境变量清洗
   - 新增子进程环境 allowlist；
   - 默认移除 token、API key、云凭证等敏感变量；
   - MCP / Node / Python / Windows 工具链保留必要变量。

验收：

- `cargo fmt --all --check`
- `cargo check --workspace`
- 与 `fetch_url`、任务、子代理、MCP 启动相关的测试能通过或给出明确跳过原因。

## v0.6.6：MCP 与网络兼容性

优先级：P1。

范围：

- MCP discovery 支持 `nextCursor` 分页；
- 单个异常 discovery item 不导致整批工具失效；
- MCP 工具排序稳定；
- stdio MCP 保留 stderr 尾部用于错误诊断；
- Streamable HTTP MCP 支持 JSON/SSE Accept、session id 持久化、GET preflight；
- MCP HTTP 支持自定义 header 与代理环境变量；
- MCP 配置 lazy reload。

验收：

- 本地 stdio MCP、HTTP MCP 各至少跑通一个示例；
- 错误 MCP 服务能显示 stderr 尾部；
- 分页 discovery 测试覆盖 tools/resources/prompts。

## v0.6.7：Windows 与 TUI 稳定性

优先级：P1。

范围：

- 统一终端模式恢复逻辑；
- panic、Ctrl+C、早退路径都恢复 raw mode、mouse、paste、alt screen；
- Windows Terminal 可自动启用 mouse capture，legacy 控制台保持保守默认；
- 新增 `composer_arrows_scroll`；
- 自动 low-motion 检测；
- 可选 synchronized output；
- 修复长 CJK、长无空格文本、markdown table、OSC 8 链接、粘贴与 Home/End 行为。

验收：

- Windows Terminal、PowerShell/ConHost、VS Code terminal 至少完成手动冒烟；
- 粘贴、滚轮、方向键滚动、Ctrl+C 取消/复制行为无回归。

## v0.6.8：Provider 与请求兼容性

优先级：P1。

范围：

- Provider-aware 请求构造；
- generic OpenAI-compatible 后端默认剥离非标准字段；
- `reasoning_content`、`strict`、`allowed_callers`、`defer_loading`、`input_examples` 等字段按能力发送；
- 保留用户已选择的 provider/model；
- local/custom endpoint 支持 no-key；
- 修复 base URL override 被默认配置覆盖的问题；
- 思考模式参数按 provider capability 下发。

验收：

- XiaomiMiMo 默认 provider 可正常对话；
- 至少一个 OpenAI-compatible 自定义 endpoint 不因非标准字段 400；
- 切换 provider/model 后重启仍保持选择。

## v0.6.9：工作区命令、Skills 与 Mention

优先级：P2。

范围：

- workspace-local slash commands；
- 优先 `.xiaomimimo/commands` 与 `~/.xiaomimimo/commands`；
- `@` mention 支持项目内 AI 工具目录与显式隐藏路径；
- Skills 搜索路径扩展到 workspace 与 global；
- 加强技能上下文截断，避免关键 workspace skill 被自动压缩丢失。

验收：

- workspace 命令可覆盖/扩展全局命令；
- gitignored 但显式输入的配置目录可补全；
- 技能上下文在长项目中保持稳定。

## v0.7.0：发布、快照与长期维护

优先级：P2。

范围：

- snapshot 总量、单文件、单 entry 上限；
- 初始化 side repo 前估算 workspace 大小；
- 自更新下载 SHA256 校验；
- release mirror 环境变量；
- npm optional postinstall 超时与失败降级；
- build script 监听 `.git/HEAD`、branch ref、commit ref；
- CI 与发布文档同步更新。

验收：

- 大仓库不会无上限初始化快照；
- npm 安装失败能快速降级并给出清晰提示；
- tag 构建产物版本号与 git tag 一致。

## 不纳入本轮计划

- 外部参考项目品牌、Provider、模型定价、官网与发布渠道；
- 第三方云厂商或 IM 平台专用文档；
- 大规模 UI/engine 重构；
- 非必要重型工具一次性引入。
