//! Core commands: help, clear, exit, model

use std::fmt::Write;

use crate::config::{COMMON_XIAOMIMIMO_MODELS, normalize_model_name};
use crate::tui::app::{App, AppAction, AppMode};
use crate::tui::views::{HelpView, ModalKind, SubAgentsView};

use super::CommandResult;

/// Show help information
pub fn help(app: &mut App, topic: Option<&str>) -> CommandResult {
    if let Some(topic) = topic {
        // Show help for specific command
        if let Some(cmd) = super::get_command_info(topic) {
            let mut help = format!(
                "{}\n\n  {}\n\n  用法：{}",
                cmd.name, cmd.description, cmd.usage
            );
            if !cmd.aliases.is_empty() {
                let _ = write!(help, "\n  别名：{}", cmd.aliases.join(", "));
            }
            return CommandResult::message(help);
        }
        return CommandResult::error(format!("未知指令：{topic}"));
    }

    // Show help overlay
    if app.view_stack.top_kind() != Some(ModalKind::Help) {
        app.view_stack.push(HelpView::new_for_locale(app.ui_locale));
    }
    CommandResult::ok()
}

/// Clear conversation history
pub fn clear(app: &mut App) -> CommandResult {
    app.clear_history();
    app.mark_history_updated();
    app.api_messages.clear();
    app.system_prompt = None;
    app.transcript_selection.clear();
    app.queued_messages.clear();
    app.queued_draft = None;
    app.total_conversation_tokens = 0;
    let todos_cleared = app.clear_todos();
    app.tool_log.clear();
    app.tool_cells.clear();
    app.tool_details_by_cell.clear();
    app.exploring_entries.clear();
    app.ignored_tool_calls.clear();
    app.pending_tool_uses.clear();
    app.last_exec_wait_command = None;
    app.last_prompt_tokens = None;
    app.last_completion_tokens = None;
    app.current_session_id = None;
    let message = if todos_cleared {
        "对话已清空".to_string()
    } else {
        "对话已清空（计划状态正忙；如有需要请再次运行 /clear）".to_string()
    };
    CommandResult::with_message_and_action(
        message,
        AppAction::SyncSession {
            messages: Vec::new(),
            system_prompt: None,
            model: app.model.clone(),
            workspace: app.workspace.clone(),
        },
    )
}

/// Exit the application
pub fn exit() -> CommandResult {
    CommandResult::action(AppAction::Quit)
}

/// Switch or view current model. With no argument, open the two-pane
/// picker (Pro/Flash + thinking effort) per #39 — gives users a discoverable
/// way to flip both knobs without memorising the docs.
pub fn model(app: &mut App, model_name: Option<&str>) -> CommandResult {
    if let Some(name) = model_name {
        let Some(model_id) = normalize_model_name(name) else {
            return CommandResult::error(format!(
                "无效模型 '{name}'。请输入 XiaomiMiMo 模型 ID。常用模型：{}",
                COMMON_XIAOMIMIMO_MODELS.join(", ")
            ));
        };
        let old_model = app.model.clone();
        app.model = model_id.clone();
        app.update_model_compaction_budget();
        app.last_prompt_tokens = None;
        app.last_completion_tokens = None;
        CommandResult::with_message_and_action(
            format!("模型已切换：{old_model} → {model_id}"),
            AppAction::UpdateCompaction(app.compaction_config()),
        )
    } else {
        CommandResult::action(AppAction::OpenModelPicker)
    }
}

/// Fetch and list available models from the configured API endpoint.
pub fn models(_app: &mut App) -> CommandResult {
    CommandResult::action(AppAction::FetchModels)
}

/// List sub-agent status from the engine
pub fn subagents(app: &mut App) -> CommandResult {
    if app.view_stack.top_kind() != Some(ModalKind::SubAgents) {
        app.view_stack
            .push(SubAgentsView::new(app.subagent_cache.clone()));
    }
    app.status_message = Some("正在获取子代理状态...".to_string());
    CommandResult::action(AppAction::ListSubAgents)
}

/// Show `XiaomiMiMo` dashboard and docs links
pub fn xiaomimimo_links() -> CommandResult {
    CommandResult::message(
        "XiaomiMiMo 链接：\n\
─────────────────────────────\n\
控制台： https://platform.xiaomimimo.com\n\
文档：   https://platform.xiaomimimo.com/docs\n\n\
提示：API key 可在控制台中获取。",
    )
}

/// Show home dashboard with stats and quick actions
pub fn home_dashboard(app: &mut App) -> CommandResult {
    let mut stats = String::new();

    // Basic info
    let _ = writeln!(stats, "XiaomiMiMo TUI 首页");
    let _ = writeln!(stats, "============================================");

    // Model & mode
    let _ = writeln!(stats, "模型：      {}", app.model);
    let _ = writeln!(stats, "模式：      {}", app.mode.label());
    let _ = writeln!(stats, "工作区：    {}", app.workspace.display());

    // Session stats
    let history_count = app.history.len();
    let total_tokens = app.total_conversation_tokens;
    let queued_messages = app.queued_messages.len();
    let _ = writeln!(stats, "历史：      {} 条消息", history_count);
    let _ = writeln!(stats, "Tokens：    {}（会话）", total_tokens);
    if queued_messages > 0 {
        let _ = writeln!(stats, "队列：      {} 条消息", queued_messages);
    }

    // Sub-agents
    let subagent_count = app.subagent_cache.len();
    if subagent_count > 0 {
        let _ = writeln!(stats, "子代理：    {} 个活跃", subagent_count);
    }

    // Active skill
    if let Some(skill) = &app.active_skill {
        let _ = writeln!(stats, "技能：      {}（已激活）", skill);
    }

    // Quick actions section
    let _ = writeln!(stats, "\n快捷操作");
    let _ = writeln!(stats, "--------------------------------------------");
    let _ = writeln!(stats, "/links      - 控制台和 API 链接");
    let _ = writeln!(stats, "/skills      - 列出可用技能");
    let _ = writeln!(stats, "/config      - 打开交互式配置编辑器");
    let _ = writeln!(stats, "/settings    - 显示持久化设置");
    let _ = writeln!(stats, "/model       - 切换或查看模型");
    let _ = writeln!(stats, "/subagents   - 查看子代理状态");
    let _ = writeln!(stats, "/task list   - 显示后台任务队列");
    let _ = writeln!(stats, "/help        - 显示帮助");

    // Mode-specific tips
    let _ = writeln!(stats, "\n模式提示");
    let _ = writeln!(stats, "--------------------------------------------");
    match app.mode {
        AppMode::Agent => {
            let _ = writeln!(stats, "Agent 模式 - 使用工具执行自主任务");
            let _ = writeln!(stats, "  使用 Ctrl+X 可在执行前进入 Plan 模式审查");
            let _ = writeln!(stats, "  输入 /yolo 可启用完整工具访问");
        }
        AppMode::Yolo => {
            let _ = writeln!(stats, "YOLO 模式 - 完整工具访问，无需批准");
            let _ = writeln!(stats, "  请谨慎执行破坏性操作！");
        }
        AppMode::Plan => {
            let _ = writeln!(stats, "Plan 模式 - 先设计再实现");
            let _ = writeln!(stats, "  使用 /plan 创建结构化检查清单");
        }
    }

    CommandResult::message(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::models::Message;
    use crate::tui::app::{App, AppMode, TuiOptions};
    use crate::tui::history::HistoryCell;
    use std::path::PathBuf;

    fn create_test_app() -> App {
        let options = TuiOptions {
            model: "mimo-v2.5-pro".to_string(),
            workspace: PathBuf::from("/tmp/test-workspace"),
            workspace_explicit: true,
            allow_shell: false,
            use_alt_screen: true,
            use_mouse_capture: false,
            use_bracketed_paste: true,
            max_subagents: 1,
            skills_dir: PathBuf::from("/tmp/test-skills"),
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
    fn test_help_unknown_command() {
        let mut app = create_test_app();
        let result = help(&mut app, Some("nonexistent"));
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("未知指令"));
        assert!(result.action.is_none());
    }

    #[test]
    fn test_help_known_command() {
        let mut app = create_test_app();
        let result = help(&mut app, Some("clear"));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("clear"));
        assert!(msg.contains("清空对话历史"));
        assert!(msg.contains("用法：/clear"));
    }

    #[test]
    fn test_help_config_topic_uses_interactive_editor_text() {
        let mut app = create_test_app();
        let result = help(&mut app, Some("config"));
        let msg = result.message.expect("help topic should return message");
        assert!(msg.contains("config"));
        assert!(msg.contains("交互式配置编辑器"));
        assert!(msg.contains("用法：/config"));
    }

    #[test]
    fn test_help_links_topic_shows_aliases() {
        let mut app = create_test_app();
        let result = help(&mut app, Some("links"));
        let msg = result.message.expect("help topic should return message");
        assert!(msg.contains("links"));
        assert!(msg.contains("XiaomiMiMo 控制台和文档链接"));
        assert!(msg.contains("用法：/links"));
        assert!(msg.contains("别名：dashboard, api"));
    }

    #[test]
    fn test_help_pushes_overlay() {
        let mut app = create_test_app();
        assert_ne!(app.view_stack.top_kind(), Some(ModalKind::Help));
        let result = help(&mut app, None);
        assert_eq!(result.message, None);
        assert_eq!(result.action, None);
        assert_eq!(app.view_stack.top_kind(), Some(ModalKind::Help));
    }

    #[test]
    fn test_help_does_not_duplicate_overlay() {
        let mut app = create_test_app();
        help(&mut app, None);
        let initial_kind = app.view_stack.top_kind();
        help(&mut app, None);
        assert_eq!(app.view_stack.top_kind(), initial_kind);
    }

    #[test]
    fn test_clear_resets_all_state() {
        let mut app = create_test_app();
        // Set up some state
        app.history.push(HistoryCell::User {
            content: "test".to_string(),
        });
        app.api_messages.push(Message {
            role: "user".to_string(),
            content: vec![],
        });
        app.total_conversation_tokens = 100;
        app.tool_log.push("test".to_string());
        app.current_session_id = Some("existing-session".to_string());

        let result = clear(&mut app);
        assert!(result.message.is_some());
        assert!(app.history.is_empty());
        assert!(app.api_messages.is_empty());
        assert_eq!(app.total_conversation_tokens, 0);
        assert!(app.tool_log.is_empty());
        assert!(app.tool_cells.is_empty());
        assert!(app.tool_details_by_cell.is_empty());
        assert!(app.current_session_id.is_none());
        assert!(matches!(result.action, Some(AppAction::SyncSession { .. })));
    }

    #[test]
    fn test_exit_returns_quit_action() {
        let result = exit();
        assert!(result.message.is_none());
        assert!(matches!(result.action, Some(AppAction::Quit)));
    }

    #[test]
    fn test_model_change_updates_state() {
        let mut app = create_test_app();
        let old_model = app.model.clone();
        let result = model(&mut app, Some("mimo-v2-flash"));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains(&old_model));
        assert!(msg.contains("mimo-v2-flash"));
        assert!(matches!(
            result.action,
            Some(AppAction::UpdateCompaction(_))
        ));
        assert_eq!(app.model, "mimo-v2-flash");
        assert_eq!(app.last_prompt_tokens, None);
        assert_eq!(app.last_completion_tokens, None);
    }

    #[test]
    fn test_model_change_accepts_future_xiaomimimo_model() {
        let mut app = create_test_app();
        let result = model(&mut app, Some("mimo-v2.5-pro"));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("mimo-v2.5-pro"));
        assert_eq!(app.model, "mimo-v2.5-pro");
        assert!(matches!(
            result.action,
            Some(AppAction::UpdateCompaction(_))
        ));
    }

    #[test]
    fn test_model_change_rejects_invalid_model() {
        let mut app = create_test_app();
        let result = model(&mut app, Some("gpt-4"));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("无效模型"));
        assert!(msg.contains("XiaomiMiMo 模型 ID"));
        assert!(msg.contains("mimo-v2.5-pro"));
        assert!(msg.contains("mimo-v2-flash"));
        assert!(result.action.is_none());
    }

    #[test]
    fn test_model_without_args_opens_picker() {
        let mut app = create_test_app();
        let result = model(&mut app, None);
        assert_eq!(result.message, None);
        assert_eq!(result.action, Some(AppAction::OpenModelPicker));
    }

    #[test]
    fn test_models_triggers_fetch_action() {
        let mut app = create_test_app();
        let result = models(&mut app);
        assert!(result.message.is_none());
        assert!(matches!(result.action, Some(AppAction::FetchModels)));
    }

    #[test]
    fn test_subagents_pushes_view_and_sets_status() {
        let mut app = create_test_app();
        let result = subagents(&mut app);
        assert!(result.message.is_none());
        assert!(matches!(result.action, Some(AppAction::ListSubAgents)));
        assert_eq!(app.view_stack.top_kind(), Some(ModalKind::SubAgents));
        assert_eq!(
            app.status_message,
            Some("正在获取子代理状态...".to_string())
        );
    }

    #[test]
    fn test_xiaomimimo_links() {
        let result = xiaomimimo_links();
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("XiaomiMiMo 链接"));
        assert!(msg.contains("https://platform.xiaomimimo.com"));
        assert!(result.action.is_none());
    }

    #[test]
    fn test_home_dashboard_includes_all_sections() {
        let mut app = create_test_app();
        app.total_conversation_tokens = 1234;
        let result = home_dashboard(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("XiaomiMiMo TUI 首页"));
        assert!(msg.contains("模型："));
        assert!(msg.contains("模式："));
        assert!(msg.contains("工作区："));
        assert!(msg.contains("历史："));
        assert!(msg.contains("Tokens："));
        assert!(msg.contains("快捷操作"));
        assert!(msg.contains("模式提示"));
        assert!(result.action.is_none());
    }

    #[test]
    fn test_home_dashboard_shows_queued_when_present() {
        let mut app = create_test_app();
        app.queued_messages
            .push_back(crate::tui::app::QueuedMessage::new(
                "test".to_string(),
                None,
            ));
        let result = home_dashboard(&mut app);
        let msg = result.message.unwrap();
        assert!(msg.contains("队列："));
    }

    #[test]
    fn test_home_dashboard_mode_tips_for_each_mode() {
        let modes = [AppMode::Agent, AppMode::Yolo, AppMode::Plan];
        for mode in modes {
            let mut app = create_test_app();
            app.mode = mode;
            let result = home_dashboard(&mut app);
            let msg = result.message.unwrap();
            assert!(msg.contains("模式提示"), "Missing tips for mode {mode:?}");
        }
    }

    #[test]
    fn test_home_dashboard_quick_actions_reflect_links_and_config_and_hide_removed_commands() {
        let mut app = create_test_app();
        let result = home_dashboard(&mut app);
        let msg = result
            .message
            .expect("home dashboard should return message");
        assert!(msg.contains("/links      - 控制台和 API 链接"));
        assert!(msg.contains("/config      - 打开交互式配置编辑器"));
        assert!(
            !msg.lines()
                .any(|line| line.trim_start().starts_with("/set "))
        );
        assert!(!msg.contains("/xiaomimimo"));
    }
}
