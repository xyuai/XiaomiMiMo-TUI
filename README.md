# XiaomiMiMo-TUI

XiaomiMiMo-TUI 是一个为 Xiaomi MiMo 适配的终端 TUI 客户端。

它面向 Xiaomi MiMo 兼容的 OpenAI 风格 Chat API，提供键盘优先的终端交互体验，适合在命令行里进行连续对话、代码辅助、工具调用和会话管理。

> 本项目最初基于 [deepseektui](https://github.com/Hmbown/DeepSeek-TUI) v0.8.2 修改而来，原项目使用 MIT License。当前项目已围绕 Xiaomi MiMo 进行适配，后续不一定跟随 deepseektui 上游更新。

## 功能简介

- **MiMo 对话 TUI**：在终端中使用 Xiaomi MiMo 进行多轮对话。
- **OpenAI 兼容接口**：默认使用 Xiaomi MiMo Token Plan 套餐专属 OpenAI-compatible API 与 Base URL，也可通过配置覆盖。
- **模型配置**：默认模型为 `mimo-v2.5-pro`，可通过配置文件、环境变量或命令行切换。
- **语音合成**：新增 `speech`/`tts` 命令，支持 MiMo-V2.5-TTS 内置音色、声音设计和声音克隆。
- **会话管理**：支持会话保存、恢复和历史记录。
- **启动引导**：首次启动会引导创建或选择工作区文件夹，避免误把 Downloads 等大目录作为默认工作区。
- **工具工作流**：保留原 TUI 架构中的工具调用、文件/命令辅助等能力。
- **Windows 发布包**：当前 Release 优先提供 Windows x64 可执行文件。

## 快速开始

### Windows Release

在 GitHub Release 下载 Windows x64 资产：

- `xiaomimimo-windows-x64.exe`
- `xiaomimimo-tui-windows-x64.exe`
- `xiaomimimo-artifacts-sha256.txt`

设置 API Key 后运行：

```powershell
$env:XIAOMIMIMO_API_KEY="your-api-key"
.\xiaomimimo.exe
```

> 从 v0.5.2 开始，工作区快照默认关闭，避免在 `Downloads` 等大目录启动后发送消息前扫描整个目录。若你在项目目录中需要 `/restore` 快照功能，可在 `~/.xiaomimimo/config.toml` 中手动开启 `[snapshots] enabled = true`。

> 首次启动时会提示创建或选择工作区文件夹。推荐使用独立目录，例如 `~/XiaomiMiMo-Workspace`，不要直接把下载目录、桌面根目录或磁盘根目录作为长期工作区。

### 语音合成

语音功能使用 MiMo-V2.5-TTS 系列模型，通过 OpenAI 兼容的 Chat Completions 接口生成音频文件：

```powershell
# 内置音色
.\xiaomimimo.exe speech "你好，这是一段 MiMo 语音合成测试。" --voice 冰糖 -o hello.wav

# 可使用别名 tts
.\xiaomimimo.exe tts "欢迎使用 XiaomiMiMo-TUI。" --instruction "温柔、自然、语速稍慢" -o welcome.wav

# 声音设计
.\xiaomimimo.exe speech "今晚的故事现在开始。" --voice-prompt "温暖沉稳的中文男声，像深夜电台主持人" -o radio.wav

# 声音克隆（支持 mp3/wav 样本，base64 后不超过 10 MB）
.\xiaomimimo.exe speech "这是一段克隆音色测试。" --clone-voice .\voice.wav -o clone.wav
```

常用内置音色：`mimo_default`、`冰糖`、`茉莉`、`苏打`、`白桦`、`Mia`、`Chloe`、`Milo`、`Dean`。

### 从源码运行

```powershell
$env:XIAOMIMIMO_API_KEY="your-api-key"
$env:XIAOMIMIMO_BASE_URL="https://token-plan-cn.xiaomimimo.com/v1"
cargo run --bin xiaomimimo
```

### npm wrapper

```bash
npm install -g xiaomimimo-tui
xiaomimimo login --api-key "YOUR_XIAOMIMIMO_API_KEY"
xiaomimimo
```

> 当前 npm wrapper 的预构建二进制优先支持 Windows x64。

## 配置

配置来源包括 `~/.xiaomimimo/config.toml`、环境变量和命令行参数。

常用环境变量：

```bash
XIAOMIMIMO_API_KEY=your-api-key
XIAOMIMIMO_BASE_URL=https://token-plan-cn.xiaomimimo.com/v1
XIAOMIMIMO_MODEL=mimo-v2.5-pro
```

示例配置文件：[`config.example.toml`](config.example.toml)  
示例环境变量文件：[`.env.example`](.env.example)

## 发布版本

当前版本：`0.5.3`

Windows x64 Release 资产：

| 文件 | 说明 |
| --- | --- |
| `xiaomimimo-windows-x64.exe` | 主命令入口 |
| `xiaomimimo-tui-windows-x64.exe` | TUI 可执行文件 |
| `xiaomimimo-artifacts-sha256.txt` | SHA-256 校验文件 |

> Release only publishes platform-qualified canonical assets; the legacy
> `xiaomimimo.exe` and `xiaomimimo-tui.exe` aliases are no longer uploaded,
> so they cannot duplicate the Windows x64 binaries.

## 与 deepseektui 的关系

本项目最初基于 [deepseektui](https://github.com/Hmbown/DeepSeek-TUI) v0.8.2 适配而来。

- 原项目：<https://github.com/Hmbown/DeepSeek-TUI>
- 原许可证：MIT License
- 当前项目：<https://github.com/xyuai/XiaomiMiMo-TUI>

本仓库保留必要的原项目版权和来源说明，Xiaomi MiMo 相关修改由本仓库维护。

## English

XiaomiMiMo-TUI is a terminal user interface client adapted for Xiaomi MiMo. It provides a keyboard-first chat workflow, configuration management, session persistence, and tool-oriented terminal usage for Xiaomi MiMo-compatible OpenAI-style APIs.

This project was originally adapted from [deepseektui](https://github.com/Hmbown/DeepSeek-TUI) v0.8.2 under the MIT License.

## License

MIT. See [LICENSE](LICENSE) and [NOTICE.md](NOTICE.md).
