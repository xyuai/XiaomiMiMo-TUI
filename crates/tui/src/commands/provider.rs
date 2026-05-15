//! Provider switching: flip between XiaomiMiMo, hosted providers, and self-hosted
//! OpenAI-compatible Xiaomi MiMo servers at runtime.
//!
//! `/provider` with no args opens the picker modal (#52). `/provider <name>`
//! keeps the v0.6.6 CLI form for muscle-memory + scripted use.

use crate::config::{ApiProvider, normalize_model_name};
use crate::tui::app::{App, AppAction};

use super::CommandResult;

/// Switch or view the current LLM backend.
///
/// With no args, opens the picker modal. With `<provider> [model]`, performs
/// the switch directly (e.g. `/provider nim flash` lands on
/// `mimo-v2-flash`). The optional model accepts shorthand
/// (`flash`, `pro`, legacy `v4-flash`/`v4-pro`) or any normal XiaomiMiMo model ID.
pub fn provider(app: &mut App, args: Option<&str>) -> CommandResult {
    let trimmed = args.map(str::trim).filter(|s| !s.is_empty());
    let Some(args) = trimmed else {
        return CommandResult::action(AppAction::OpenProviderPicker);
    };

    let mut parts = args.split_whitespace();
    let name = parts.next().unwrap_or("");
    let model_arg = parts.next();

    let Some(target) = ApiProvider::parse(name) else {
        return CommandResult::error(format!(
            "未知 provider '{name}'。可用：xiaomimimo、nvidia-nim、openrouter、novita、fireworks 或 sglang。"
        ));
    };

    let model = match model_arg {
        None => None,
        Some(raw) => match normalize_provider_model(target, raw) {
            Some(model) => Some(model),
            None => {
                return CommandResult::error(format!(
                    "invalid model '{raw}'. Try: flash, pro, tts, voiceclone, mimo-v2.5-pro, mimo-v2.5-tts."
                ));
            }
        },
    };

    if target == app.api_provider && model.is_none() {
        return CommandResult::message(format!("当前已是 provider：{}", target.as_str()));
    }

    CommandResult::action(AppAction::SwitchProvider {
        provider: target,
        model,
    })
}

fn expand_model_alias(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "pro" | "v4-pro" => "mimo-v2.5-pro".to_string(),
        "flash" | "v4-flash" => "mimo-v2-flash".to_string(),
        "tts" | "speech" => "mimo-v2.5-tts".to_string(),
        "voicedesign" | "voice-design" => "mimo-v2.5-tts-voicedesign".to_string(),
        "voiceclone" | "voice-clone" => "mimo-v2.5-tts-voiceclone".to_string(),
        other => other.to_string(),
    }
}

fn normalize_provider_model(provider: ApiProvider, raw: &str) -> Option<String> {
    let expanded = expand_model_alias(raw);
    if matches!(provider, ApiProvider::XiaomiMiMo) {
        return normalize_model_name(&expanded);
    }
    let trimmed = expanded.trim();
    if trimmed.is_empty() {
        None
    } else if let Some(normalized) = normalize_model_name(trimmed) {
        Some(normalized)
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::app::TuiOptions;
    use std::path::PathBuf;

    fn create_test_app() -> App {
        let options = TuiOptions {
            model: "mimo-v2.5-pro".to_string(),
            workspace: PathBuf::from("."),
            workspace_explicit: true,
            allow_shell: false,
            use_alt_screen: true,
            use_mouse_capture: false,
            use_bracketed_paste: true,
            max_subagents: 1,
            skills_dir: PathBuf::from("."),
            memory_path: PathBuf::from("memory.md"),
            notes_path: PathBuf::from("notes.txt"),
            mcp_config_path: PathBuf::from("mcp.json"),
            use_memory: false,
            start_in_agent_mode: false,
            skip_onboarding: true,
            yolo: false,
            resume_session_id: None,
        };
        App::new(options, &Config::default())
    }

    #[test]
    fn no_args_opens_picker_modal() {
        let mut app = create_test_app();
        let result = provider(&mut app, None);
        assert!(result.message.is_none());
        assert_eq!(result.action, Some(AppAction::OpenProviderPicker));
    }

    #[test]
    fn unknown_provider_returns_error() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("anthropic"));
        let msg = result.message.expect("expected error message");
        assert!(msg.contains("未知 provider"));
        assert!(msg.contains("openrouter"));
        assert!(msg.contains("novita"));
        assert!(result.action.is_none());
    }

    #[test]
    fn switch_to_openrouter_emits_action() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("openrouter"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::Openrouter);
                assert_eq!(model, None);
            }
            other => panic!("expected SwitchProvider, got {other:?}"),
        }
    }

    #[test]
    fn switch_to_novita_emits_action() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("novita"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::Novita);
                assert_eq!(model, None);
            }
            other => panic!("expected SwitchProvider, got {other:?}"),
        }
    }

    #[test]
    fn switch_to_fireworks_emits_action() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("fireworks pro"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::Fireworks);
                assert_eq!(model.as_deref(), Some("mimo-v2.5-pro"));
            }
            other => panic!("expected SwitchProvider, got {other:?}"),
        }
    }

    #[test]
    fn switch_to_sglang_flash_emits_action() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("sglang flash"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::Sglang);
                assert_eq!(model.as_deref(), Some("mimo-v2-flash"));
            }
            other => panic!("expected SwitchProvider, got {other:?}"),
        }
    }

    #[test]
    fn switching_to_active_provider_without_model_is_a_noop() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("xiaomimimo"));
        let msg = result.message.expect("expected message");
        assert!(msg.contains("当前已是 provider"));
        assert!(result.action.is_none());
    }

    #[test]
    fn switch_to_nim_emits_action_without_model_override() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("nvidia-nim"));
        assert!(result.message.is_none());
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::NvidiaNim);
                assert_eq!(model, None);
            }
            other => panic!("expected SwitchProvider action, got {other:?}"),
        }
    }

    #[test]
    fn nim_flash_shorthand_emits_action_with_model_override() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("nim flash"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::NvidiaNim);
                assert_eq!(model.as_deref(), Some("mimo-v2-flash"));
            }
            other => panic!("expected SwitchProvider action, got {other:?}"),
        }
    }

    #[test]
    fn nim_pro_shorthand_emits_action_with_model_override() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("nim pro"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::NvidiaNim);
                assert_eq!(model.as_deref(), Some("mimo-v2.5-pro"));
            }
            other => panic!("expected SwitchProvider action, got {other:?}"),
        }
    }

    #[test]
    fn provider_accepts_tts_shorthands() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("xiaomimimo tts"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::XiaomiMiMo);
                assert_eq!(model.as_deref(), Some("mimo-v2.5-tts"));
            }
            other => panic!("expected SwitchProvider action, got {other:?}"),
        }

        let result = provider(&mut app, Some("xiaomimimo voiceclone"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::XiaomiMiMo);
                assert_eq!(model.as_deref(), Some("mimo-v2.5-tts-voiceclone"));
            }
            other => panic!("expected SwitchProvider action, got {other:?}"),
        }
    }

    #[test]
    fn switch_to_active_provider_with_new_model_still_emits_action() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("xiaomimimo flash"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::XiaomiMiMo);
                assert_eq!(model.as_deref(), Some("mimo-v2-flash"));
            }
            other => panic!("expected SwitchProvider action, got {other:?}"),
        }
    }

    #[test]
    fn custom_provider_model_is_preserved() {
        let mut app = create_test_app();
        let result = provider(&mut app, Some("nim gpt-4"));
        match result.action {
            Some(AppAction::SwitchProvider { provider, model }) => {
                assert_eq!(provider, ApiProvider::NvidiaNim);
                assert_eq!(model.as_deref(), Some("gpt-4"));
            }
            other => panic!("expected SwitchProvider action, got {other:?}"),
        }
    }
}
