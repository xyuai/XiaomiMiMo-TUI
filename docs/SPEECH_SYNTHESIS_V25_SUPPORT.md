# MiMo-V2.5 语音合成支持记录

记录时间：2026-05-02 21:35（Asia/Shanghai）

对照文档：<https://platform.xiaomimimo.com/docs/zh-CN/usage-guide/speech-synthesis-v2.5>

## 已接入

- Token Plan 专属 OpenAI-compatible Base URL：`https://token-plan-cn.xiaomimimo.com/v1`。
- 套餐模型提示与模型白名单：
  - `mimo-v2.5-pro`
  - `mimo-v2.5`
  - `mimo-v2.5-tts-voiceclone`
  - `mimo-v2.5-tts-voicedesign`
  - `mimo-v2.5-tts`
  - `mimo-v2-pro`
  - `mimo-v2-omni`
  - `mimo-v2-tts`
- TUI 模型可直接调用 `speech` / `tts` 工具，不再需要先让用户手动执行 CLI。
- 合成文本按文档要求放在 assistant 消息中；`instruction` / `voice_prompt` 用作 user 消息中的自然语言控制。
- 支持在 `text` 中保留官方音频/风格标签，因为 `text` 会作为 assistant 内容发送。
- 支持内置音色：默认 `mimo_default`，并在提示词/文档中列出 `冰糖`、`茉莉`、`苏打`、`白桦`、`Mia`、`Chloe`、`Milo`、`Dean`。
- 支持声音设计：`voice_prompt` 或 `mimo-v2.5-tts-voicedesign`。
- 支持声音克隆：`.mp3` / `.wav` 样本文件，或 `data:audio/...;base64,...` URI；base64 后最大 10 MB。
- 支持输出格式：`wav`、`mp3`、`pcm16`（`pcm` 作为 `pcm16` 别名）。

## 暂未作为 TUI 直连工具开放

- 低延迟 `stream=true`：当前 `speech` / `tts` 工具写完整音频文件；如果用户显式传入 `stream=true`，工具会返回清晰错误并提示改用普通文件输出。
- 多轮自定义消息数组：当前直连工具将“最终朗读文本 + 一条自然语言控制指令”封装为文档要求的消息形态；如后续需要完全暴露任意 `messages[]`，再单独扩展。
