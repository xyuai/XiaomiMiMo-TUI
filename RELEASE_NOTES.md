# XiaomiMiMo-TUI v0.6.3

## 更新内容

- 默认要求可见思考、`reasoning_content` / `Thinking` 内容和最终回答使用简体中文，除非用户明确要求其他语言。
- 在 Chat API 请求中追加独立的高优先级中文语言 system 消息，避免完整英文工具提示词把 thinking 拉回英文。
- 保留工具名、代码、命令、路径、日志、错误信息和 API 字段原文，减少对技术内容的误翻译。
- 增加提示词和 Chat 消息构建单测，覆盖中文 thinking 语言约束。

## Windows 资源

本版本在线发布以下 Windows x64 资源：
- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

## 验证

- `cargo test -p xiaomimimo-tui chat_messages_append_independent_chinese_thinking_language_system_message -- --nocapture`
- `cargo test -p xiaomimimo-tui base_prompt_requires_simplified_chinese_thinking_and_replies -- --nocapture`
- `cargo check -p xiaomimimo-tui`
- 真实 API 复测：`reasoning_content` 返回中文（示例：`用户问的是一个简单的科学问题，不需要工具调用，直接回答即可。`）
