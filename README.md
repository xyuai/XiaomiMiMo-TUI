# mimotui

mimotui is a TUI client adapted for Mimo.

This project was originally based on [deepseektui](https://github.com/Hmbown/DeepSeek-TUI), which is licensed under the MIT License.  
It has been modified and adapted for the Mimo model and may not follow upstream deepseektui updates.

## 中文

# mimotui

mimotui 是一个为 Mimo 适配的 TUI 客户端。

本项目最初基于 [deepseektui](https://github.com/Hmbown/DeepSeek-TUI) 修改而来，原项目使用 MIT License。  
由于 Mimo 与 DeepSeek 模型差异较大，本项目后续将主要围绕 Mimo 进行适配，不一定跟随 deepseektui 上游更新。

## Quick start

```powershell
$env:XIAOMIMIMO_API_KEY="your-api-key"
$env:XIAOMIMIMO_BASE_URL="https://token-plan-cn.xiaomimimo.com/v1"
cargo run --bin xiaomimimo
```

Default model: `mimo-v2.5-pro`.

## License

MIT. See [LICENSE](LICENSE) and [NOTICE.md](NOTICE.md).
