#![allow(clippy::items_after_test_module)]

//! Debug commands: tokens, usage, system, context, undo, retry

use super::CommandResult;
use crate::compaction::estimate_input_tokens_conservative;
use crate::models::{SystemPrompt, context_window_for_model};
use crate::tui::app::{App, AppAction};
use crate::tui::history::HistoryCell;

fn token_count(value: Option<u32>) -> String {
    value.map_or_else(|| "未报告".to_string(), |tokens| tokens.to_string())
}

fn active_context_summary(app: &App) -> String {
    let estimated =
        estimate_input_tokens_conservative(&app.api_messages, app.system_prompt.as_ref());
    match context_window_for_model(&app.model) {
        Some(window) => {
            let used = estimated.min(window as usize);
            let percent = (used as f64 / f64::from(window) * 100.0).clamp(0.0, 100.0);
            format!("~{used} / {window} ({percent:.1}%)")
        }
        None => format!("~{estimated} / 未知窗口"),
    }
}

fn cache_summary(app: &App) -> String {
    match (
        app.last_prompt_cache_hit_tokens,
        app.last_prompt_cache_miss_tokens,
    ) {
        (Some(hit), Some(miss)) => format!("{hit} 命中 / {miss} 未命中"),
        (Some(hit), None) => format!("{hit} 命中 / 未命中未报告"),
        (None, Some(miss)) => format!("命中未报告 / {miss} 未命中"),
        (None, None) => "未报告".to_string(),
    }
}

/// Show token usage for session
pub fn tokens(app: &mut App) -> CommandResult {
    let message_count = app.api_messages.len();
    let chat_count = app.history.len();

    CommandResult::message(format!(
        "Token 用量：\n\
         -----------------------------\n\
         当前上下文：        {}\n\
         最近 API 输入：     {}（回合遥测；工具多轮时可能包含重复前缀）\n\
         最近 API 输出：     {}\n\
         缓存命中/未命中：   {}（仅遥测）\n\
         累计 tokens：       {}（会话用量遥测）\n\
         API 消息数：        {}\n\
         聊天消息数：        {}\n\
         模型：              {}",
        active_context_summary(app),
        token_count(app.last_prompt_tokens),
        token_count(app.last_completion_tokens),
        cache_summary(app),
        app.total_tokens,
        message_count,
        chat_count,
        app.model,
    ))
}

/// Show session token usage (legacy /cost alias).
pub fn cost(app: &mut App) -> CommandResult {
    CommandResult::message(format!(
        "会话 Token：\n\
         -----------------------------\n\
         累计 tokens：       {}\n\
         最近 API 输入：     {}\n\
         最近 API 输出：     {}\n\
         当前上下文：        {}\n\n\
         XiaomiMiMo-TUI 这里显示 token 用量，不显示金额估算。",
        app.total_tokens,
        token_count(app.last_prompt_tokens),
        token_count(app.last_completion_tokens),
        active_context_summary(app),
    ))
}

/// Show XiaomiMiMo prompt-cache telemetry for the latest completed turn.
pub fn cache(app: &mut App, arg: Option<&str>) -> CommandResult {
    if arg.is_some_and(|s| !s.trim().is_empty() && s.trim() != "status") {
        return CommandResult::error("用法：/cache [status]");
    }

    let hit = app.last_prompt_cache_hit_tokens;
    let miss = app.last_prompt_cache_miss_tokens.or_else(|| {
        app.last_prompt_tokens
            .zip(hit)
            .map(|(input, hit)| input.saturating_sub(hit))
    });
    let ratio = match (hit, miss) {
        (Some(hit), Some(miss)) if hit + miss > 0 => {
            format!("{:.1}%", 100.0 * f64::from(hit) / f64::from(hit + miss))
        }
        _ => "未报告".to_string(),
    };
    let replay = app
        .last_reasoning_replay_tokens
        .map_or_else(|| "未报告".to_string(), |v| v.to_string());

    CommandResult::message(format!(
        "Prompt 缓存（最近一轮）：\n\
         -----------------------------\n\
         输入 tokens：       {}\n\
         输出 tokens：       {}\n\
         缓存命中 tokens：   {}\n\
         缓存未命中 tokens： {}\n\
         命中率：            {}\n\
         推理重放：          {}",
        token_count(app.last_prompt_tokens),
        token_count(app.last_completion_tokens),
        token_count(hit),
        token_count(miss),
        ratio,
        replay,
    ))
}

/// Show current system prompt
pub fn system_prompt(app: &mut App) -> CommandResult {
    let prompt_text = match &app.system_prompt {
        Some(SystemPrompt::Text(text)) => text.clone(),
        Some(SystemPrompt::Blocks(blocks)) => blocks
            .iter()
            .map(|b| b.text.clone())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n"),
        None => "（未设置系统提示词）".to_string(),
    };

    // Truncate if too long
    let display = if prompt_text.len() > 500 {
        // Find a valid UTF-8 char boundary at or before byte 500
        let truncate_at = prompt_text
            .char_indices()
            .take_while(|(i, _)| *i <= 500)
            .last()
            .map_or(0, |(i, _)| i);
        format!(
            "{}...\n\n（已截断，共 {} 个字符）",
            &prompt_text[..truncate_at],
            prompt_text.len()
        )
    } else {
        prompt_text
    };

    CommandResult::message(format!(
        "系统提示词（{} 模式）：\n─────────────────────────────\n{}",
        app.mode.label(),
        display
    ))
}

/// Show context window usage
pub fn context(_app: &mut App) -> CommandResult {
    CommandResult::action(AppAction::OpenContextInspector)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::models::{ContentBlock, Message, SystemBlock};
    use crate::tui::app::{App, TuiOptions};
    use std::path::PathBuf;

    fn create_test_app() -> App {
        App::new(
            test_options_for_workspace(PathBuf::from("/tmp/test-workspace")),
            &Config::default(),
        )
    }

    fn test_options_for_workspace(workspace: PathBuf) -> TuiOptions {
        TuiOptions {
            model: "mimo-v2.5-pro".to_string(),
            workspace,
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
        }
    }

    #[test]
    fn test_tokens_shows_usage_info() {
        let mut app = create_test_app();
        app.total_tokens = 1234;
        app.session_cost = 0.05;
        app.last_prompt_tokens = Some(100);
        app.last_completion_tokens = Some(25);
        app.last_prompt_cache_hit_tokens = Some(70);
        app.last_prompt_cache_miss_tokens = Some(30);
        app.api_messages.push(Message {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: "test".to_string(),
                cache_control: None,
            }],
        });
        app.history.push(HistoryCell::User {
            content: "test".to_string(),
        });

        let result = tokens(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("Token 用量"));
        assert!(msg.contains("当前上下文"));
        assert!(msg.contains("最近 API 输入"));
        assert!(msg.contains("最近 API 输出"));
        assert!(msg.contains("缓存命中/未命中"));
        assert!(msg.contains("70 命中 / 30 未命中"));
        assert!(msg.contains("累计 tokens"));
        assert!(!msg.contains("Approx session cost:"));
        assert!(msg.contains("API 消息数"));
        assert!(msg.contains("聊天消息数"));
        assert!(msg.contains("模型"));
    }

    #[test]
    fn test_cost_shows_token_info() {
        let mut app = create_test_app();
        app.session_cost = 0.1234;
        let result = cost(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("会话 Token"));
        assert!(msg.contains("累计 tokens"));
        assert!(msg.contains("token 用量"));
        assert!(!msg.contains("$"));
    }

    #[test]
    fn test_cache_shows_latest_turn_telemetry() {
        let mut app = create_test_app();
        app.last_prompt_tokens = Some(100);
        app.last_completion_tokens = Some(20);
        app.last_prompt_cache_hit_tokens = Some(70);
        app.last_prompt_cache_miss_tokens = Some(30);
        app.last_reasoning_replay_tokens = Some(5);

        let result = cache(&mut app, None);
        let msg = result.message.expect("cache message");
        assert!(msg.contains("Prompt 缓存"));
        assert!(msg.contains("70"));
        assert!(msg.contains("30"));
        assert!(msg.contains("70.0%"));
    }

    #[test]
    fn test_system_prompt_displays_text() {
        let mut app = create_test_app();
        app.system_prompt = Some(SystemPrompt::Text("Test system prompt".to_string()));
        let result = system_prompt(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("系统提示词"));
        assert!(msg.contains("Test system prompt"));
    }

    #[test]
    fn test_system_prompt_displays_blocks() {
        let mut app = create_test_app();
        app.system_prompt = Some(SystemPrompt::Blocks(vec![
            SystemBlock {
                block_type: "text".to_string(),
                text: "Block 1".to_string(),
                cache_control: None,
            },
            SystemBlock {
                block_type: "text".to_string(),
                text: "Block 2".to_string(),
                cache_control: None,
            },
        ]));
        let result = system_prompt(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("系统提示词"));
        assert!(msg.contains("Block 1"));
        assert!(msg.contains("Block 2"));
    }

    #[test]
    fn test_system_prompt_none() {
        let mut app = create_test_app();
        app.system_prompt = None;
        let result = system_prompt(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("未设置系统提示词"));
    }

    #[test]
    fn test_system_prompt_truncates_long_text() {
        let mut app = create_test_app();
        let long_text = "x".repeat(600);
        app.system_prompt = Some(SystemPrompt::Text(long_text));
        let result = system_prompt(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("..."));
        assert!(msg.contains("个字符"));
    }

    #[test]
    fn test_context_shows_usage_stats() {
        let mut app = create_test_app();
        app.api_messages.push(Message {
            role: "user".to_string(),
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
                cache_control: None,
            }],
        });
        app.history.push(HistoryCell::User {
            content: "Hello".to_string(),
        });

        let result = context(&mut app);
        assert!(matches!(
            result.action,
            Some(AppAction::OpenContextInspector)
        ));
        assert!(result.message.is_none());
    }

    #[test]
    fn test_undo_removes_last_exchange() {
        let mut app = create_test_app();
        app.history.push(HistoryCell::User {
            content: "Hello".to_string(),
        });
        app.history.push(HistoryCell::Assistant {
            content: "Hi".to_string(),
            streaming: false,
        });
        app.api_messages.push(Message {
            role: "user".to_string(),
            content: vec![],
        });
        app.api_messages.push(Message {
            role: "assistant".to_string(),
            content: vec![],
        });

        let initial_history_len = app.history.len();
        let initial_api_len = app.api_messages.len();
        let result = undo_conversation(&mut app);

        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("已移除"));
        assert!(app.history.len() < initial_history_len);
        assert!(app.api_messages.len() < initial_api_len);
    }

    #[test]
    fn test_undo_nothing_to_undo() {
        let mut app = create_test_app();
        // Clear any default history
        app.history.clear();
        app.api_messages.clear();
        let result = undo_conversation(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("没有可撤销") || msg.contains("已移除"));
    }

    #[test]
    fn patch_undo_restores_latest_tool_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        let workspace = tmp.path();
        std::process::Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(workspace)
            .status()
            .expect("git init");

        let mut app = App::new(
            test_options_for_workspace(workspace.to_path_buf()),
            &Config::default(),
        );
        let file = workspace.join("file.txt");
        std::fs::write(&file, "old").unwrap();
        crate::core::turn::pre_tool_snapshot(workspace, "test-call").expect("snapshot");
        std::fs::write(&file, "new").unwrap();

        let result = patch_undo(&mut app);
        let msg = result.message.expect("undo message");
        assert!(msg.contains("已恢复快照"));
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "old");
    }

    #[test]
    fn test_retry_with_previous_message() {
        let mut app = create_test_app();
        app.history.push(HistoryCell::User {
            content: "Test message".to_string(),
        });
        app.history.push(HistoryCell::Assistant {
            content: "Response".to_string(),
            streaming: false,
        });

        let result = retry(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("正在重试"));
        assert!(msg.contains("Test message"));
        assert!(matches!(result.action, Some(AppAction::SendMessage(_))));
    }

    #[test]
    fn test_retry_no_previous_message() {
        let mut app = create_test_app();
        let result = retry(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("没有可重试"));
        assert!(result.action.is_none());
    }

    #[test]
    fn test_retry_truncates_long_input() {
        let mut app = create_test_app();
        let long_input = "x".repeat(100);
        app.history.push(HistoryCell::User {
            content: long_input.clone(),
        });
        app.history.push(HistoryCell::Assistant {
            content: "Response".to_string(),
            streaming: false,
        });

        let result = retry(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("正在重试"));
        assert!(msg.contains("..."));
    }
}

/// Remove last message pair (user + assistant).
pub fn undo_conversation(app: &mut App) -> CommandResult {
    // Remove from display history (up to the last user message)
    let mut removed_count = 0;
    while !app.history.is_empty() {
        let last_is_user = matches!(app.history.last(), Some(HistoryCell::User { .. }));
        app.pop_history();
        removed_count += 1;
        if last_is_user {
            break;
        }
    }

    // Remove from API messages
    while let Some(last) = app.api_messages.last() {
        if last.role == "user" {
            app.api_messages.pop();
            break;
        }
        app.api_messages.pop();
    }

    if removed_count > 0 {
        // Keep tool/index mappings consistent after truncation.
        app.tool_cells.clear();
        app.tool_details_by_cell.clear();
        app.exploring_entries.clear();
        app.ignored_tool_calls.clear();
        app.mark_history_updated();
        CommandResult::message(format!("已移除 {removed_count} 条消息"))
    } else {
        CommandResult::message("没有可撤销的内容")
    }
}

/// Revert the most recent file-modifying tool snapshot if available.
pub fn patch_undo(app: &mut App) -> CommandResult {
    let workspace = app.workspace.clone();
    let repo = match crate::snapshot::SnapshotRepo::open_or_init(&workspace) {
        Ok(repo) => repo,
        Err(err) => {
            return CommandResult::error(format!(
                "快照仓库不可用（{}）：{err}",
                workspace.display()
            ));
        }
    };

    let snapshots = match repo.list(20) {
        Ok(snapshots) => snapshots,
        Err(err) => return CommandResult::error(format!("列出快照失败：{err}")),
    };
    if snapshots.is_empty() {
        return CommandResult::message("未找到可撤销快照；无法使用快照回退。");
    }

    let target = snapshots
        .iter()
        .find(|s| s.label.starts_with("tool:"))
        .or_else(|| snapshots.iter().find(|s| s.label.starts_with("pre-turn:")));
    let Some(target) = target else {
        return CommandResult::message("未找到工具或回合前快照可撤销。");
    };

    if let Err(err) = repo.restore(&target.id) {
        return CommandResult::error(format!("恢复失败：{err}"));
    }

    let diff_stat = std::process::Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(&workspace)
        .output()
        .ok()
        .and_then(|out| {
            let stat = String::from_utf8_lossy(&out.stdout).trim().to_string();
            (!stat.is_empty()).then_some(stat)
        });

    let short = &target.id.as_str()[..target.id.as_str().len().min(8)];
    let summary = match diff_stat {
        Some(stat) => format!(
            "已恢复快照 '{}'（{}）。受影响文件：\n{stat}",
            target.label, short
        ),
        None => format!(
            "已恢复快照 '{}'（{}）。未检测到 diff 变化。",
            target.label, short
        ),
    };

    app.push_history_cell(HistoryCell::System {
        content: format!("/undo 已将工作区恢复到快照 '{}'（{short}）", target.label),
    });
    CommandResult::message(summary)
}

/// Prefer patch-aware undo for recent file writes; fall back to conversation
/// undo when no snapshots exist.
pub fn undo(app: &mut App) -> CommandResult {
    let result = patch_undo(app);
    let fallback = result.message.as_deref().is_some_and(|msg| {
        msg.starts_with("未找到可撤销快照") || msg.starts_with("未找到工具或回合前快照")
    });
    if fallback {
        undo_conversation(app)
    } else {
        result
    }
}

/// Retry last request - remove last exchange and re-send the user's message
pub fn retry(app: &mut App) -> CommandResult {
    let last_user_input = app.history.iter().rev().find_map(|cell| match cell {
        HistoryCell::User { content } => Some(content.clone()),
        _ => None,
    });

    match last_user_input {
        Some(input) => {
            undo(app);
            let display_input = if input.len() > 50 {
                let truncate_at = input
                    .char_indices()
                    .take_while(|(i, _)| *i <= 50)
                    .last()
                    .map_or(0, |(i, _)| i);
                format!("{}...", &input[..truncate_at])
            } else {
                input.clone()
            };
            CommandResult::with_message_and_action(
                format!("正在重试：{display_input}"),
                AppAction::SendMessage(input),
            )
        }
        None => CommandResult::error("没有可重试的上一条请求"),
    }
}
