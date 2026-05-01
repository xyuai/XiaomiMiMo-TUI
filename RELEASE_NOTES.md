# XiaomiMiMo-TUI v0.5.1

本版本继续面向 Xiaomi MiMo 优化 TUI 体验，优先提供 Windows x64 构建。

## 更新内容

- 首次启动、API Key 配置和工作区引导改为中文，并提示 Token Plan 套餐专属 API 与 Base URL。
- 默认系统提示要求使用简体中文回答；可见的思考、推理摘要、计划和待办也优先中文。
- 修复 `/yolo` 等权限模式切换后未立即同步到底层引擎的问题，YOLO 使用全权限工具上下文。
- 修复命令面板中 `/model`、`/provider`、`/statusline` 等可选参数指令不能直接打开的问题。
- 底部状态栏从美元估算改为 token 用量显示：千级显示 `k`，百万级显示 `m` 并保留两位小数（如 `1.39m`）。

## Windows 资产

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

## 功能简介

- 在终端内使用 Xiaomi MiMo 进行多轮对话。
- 支持 Token Plan 套餐专属 OpenAI-compatible API。
- 支持会话保存/恢复、命令面板、模型/Provider 切换、工作区工具调用和多种执行模式。

## 来源说明

本项目最初基于 [deepseektui](https://github.com/Hmbown/DeepSeek-TUI) v0.8.2 修改而来，原项目使用 MIT License。
