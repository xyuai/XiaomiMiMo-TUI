# XiaomiMiMo-TUI

XiaomiMiMo-TUI is a terminal user interface client adapted for Xiaomi MiMo.

It provides a keyboard-first TUI experience for working with Xiaomi MiMo-compatible OpenAI-style chat APIs, with configuration, session persistence, tool execution, and model/provider settings built around Xiaomi MiMo usage.

> This project was originally based on [deepseektui](https://github.com/Hmbown/DeepSeek-TUI), which is licensed under the MIT License. It has been modified and adapted for Xiaomi MiMo and may not follow upstream deepseektui updates.

## Features

- Terminal UI for Xiaomi MiMo chat workflows
- OpenAI-compatible API endpoint support
- Config file and environment-variable based setup
- Session/history persistence
- Tool and shell workflow support inherited from the original TUI architecture
- Cross-platform npm wrapper for release binaries

## Quick start

### From source

```powershell
$env:XIAOMIMIMO_API_KEY="your-api-key"
$env:XIAOMIMIMO_BASE_URL="https://token-plan-cn.xiaomimimo.com/v1"
cargo run --bin xiaomimimo
```

Default model: `mimo-v2.5-pro`.

### npm wrapper

```bash
npm install -g xiaomimimo-tui
xiaomimimo login --api-key "YOUR_XIAOMIMIMO_API_KEY"
xiaomimimo
```

The npm package installs wrapper commands for `xiaomimimo` and `xiaomimimo-tui` from GitHub release artifacts.

## Configuration

Configuration is read from `~/.xiaomimimo/config.toml`, environment variables, and command-line options.

Common environment variables:

```bash
XIAOMIMIMO_API_KEY=your-api-key
XIAOMIMIMO_BASE_URL=https://token-plan-cn.xiaomimimo.com/v1
XIAOMIMIMO_MODEL=mimo-v2.5-pro
```

A sample config is available in [`config.example.toml`](config.example.toml), and a sample environment file is available in [`.env.example`](.env.example).

## Repository

- GitHub: [xyuai/XiaomiMiMo-TUI](https://github.com/xyuai/XiaomiMiMo-TUI)
- Version: `0.5.0`

## Relationship to deepseektui

This project was initially adapted from [deepseektui](https://github.com/Hmbown/DeepSeek-TUI) v0.8.2.

Original project: [https://github.com/Hmbown/DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)  
Original license: MIT License

Portions of this repository may include code derived from deepseektui. Xiaomi MiMo-specific modifications are maintained here.

## 中文

# XiaomiMiMo-TUI

XiaomiMiMo-TUI 是一个为 Xiaomi MiMo 适配的终端 TUI 客户端。

它面向 Xiaomi MiMo 兼容的 OpenAI 风格 Chat API，提供键盘优先的终端交互体验，并包含配置、会话保存、工具调用、模型/Provider 设置等能力。

本项目最初基于 [deepseektui](https://github.com/Hmbown/DeepSeek-TUI) v0.8.2 修改而来，原项目使用 MIT License。由于 Xiaomi MiMo 与 DeepSeek 模型及接口侧重点不同，本项目后续将主要围绕 Xiaomi MiMo 进行适配，不一定跟随 deepseektui 上游更新。

## 许可证

MIT。请查看 [LICENSE](LICENSE) 和 [NOTICE.md](NOTICE.md)。
