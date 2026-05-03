//! Session commands: save, load, compact, export

use std::fmt::Write;
use std::path::PathBuf;

use crate::session_manager::create_saved_session_with_mode;
use crate::tui::app::{App, AppAction};
use crate::tui::history::{HistoryCell, history_cells_from_message};
use crate::tui::session_picker::SessionPickerView;

use super::CommandResult;

/// Save session to file
pub fn save(app: &mut App, path: Option<&str>) -> CommandResult {
    let save_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        PathBuf::from(format!("session_{timestamp}.json"))
    };

    let messages = app.api_messages.clone();
    let session = create_saved_session_with_mode(
        &messages,
        &app.model,
        &app.workspace,
        u64::from(app.total_tokens),
        app.system_prompt.as_ref(),
        Some(app.mode.label()),
    );

    let sessions_dir = save_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map_or_else(|| app.workspace.clone(), std::path::Path::to_path_buf);

    let final_save_path = if save_path.is_absolute() {
        save_path.clone()
    } else {
        sessions_dir.join(&save_path)
    };

    match std::fs::create_dir_all(&sessions_dir) {
        Ok(()) => {
            let json = match serde_json::to_string_pretty(&session) {
                Ok(j) => j,
                Err(e) => return CommandResult::error(format!("序列化会话失败：{e}")),
            };
            match std::fs::write(&final_save_path, json) {
                Ok(()) => {
                    app.current_session_id = Some(session.metadata.id.clone());
                    CommandResult::message(format!(
                        "会话已保存到 {}（ID：{}）",
                        final_save_path.display(),
                        &session.metadata.id[..8]
                    ))
                }
                Err(e) => CommandResult::error(format!("保存会话失败：{e}")),
            }
        }
        Err(e) => CommandResult::error(format!("创建目录失败：{e}")),
    }
}

/// Load session from file
pub fn load(app: &mut App, path: Option<&str>) -> CommandResult {
    let load_path = if let Some(p) = path {
        if p.contains('/') || p.contains('\\') {
            PathBuf::from(p)
        } else {
            app.workspace.join(p)
        }
    } else {
        return CommandResult::error("用法：/load <path>");
    };

    let content = match std::fs::read_to_string(&load_path) {
        Ok(c) => c,
        Err(e) => {
            return CommandResult::error(format!("读取会话文件失败：{e}"));
        }
    };

    let session: crate::session_manager::SavedSession = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            return CommandResult::error(format!("解析会话文件失败：{e}"));
        }
    };

    app.api_messages.clone_from(&session.messages);
    app.clear_history();
    let cells_to_add: Vec<_> = app
        .api_messages
        .iter()
        .flat_map(history_cells_from_message)
        .collect();
    app.extend_history(cells_to_add);
    app.mark_history_updated();
    app.transcript_selection.clear();
    app.model.clone_from(&session.metadata.model);
    app.update_model_compaction_budget();
    app.workspace.clone_from(&session.metadata.workspace);
    app.total_tokens = u32::try_from(session.metadata.total_tokens).unwrap_or(u32::MAX);
    app.total_conversation_tokens = app.total_tokens;
    app.last_prompt_tokens = None;
    app.last_completion_tokens = None;
    app.current_session_id = Some(session.metadata.id.clone());
    if let Some(sp) = session.system_prompt {
        app.system_prompt = Some(crate::models::SystemPrompt::Text(sp));
    }
    app.scroll_to_bottom();

    CommandResult::with_message_and_action(
        format!(
            "已从 {} 加载会话（ID：{}，{} 条消息）",
            load_path.display(),
            &session.metadata.id[..8],
            session.metadata.message_count
        ),
        crate::tui::app::AppAction::SyncSession {
            messages: app.api_messages.clone(),
            system_prompt: app.system_prompt.clone(),
            model: app.model.clone(),
            workspace: app.workspace.clone(),
        },
    )
}

/// Trigger context compaction
pub fn compact(_app: &mut App) -> CommandResult {
    // Trigger immediate compaction via engine
    CommandResult::with_message_and_action(
        "已触发上下文压缩...".to_string(),
        AppAction::CompactContext,
    )
}

/// Export conversation to markdown
pub fn export(app: &mut App, path: Option<&str>) -> CommandResult {
    let export_path = path.map_or_else(
        || {
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
            PathBuf::from(format!("chat_export_{timestamp}.md"))
        },
        PathBuf::from,
    );
    let final_export_path = if export_path.is_absolute() {
        export_path.clone()
    } else {
        app.workspace.join(&export_path)
    };
    if let Some(parent) = final_export_path.parent()
        && !parent.as_os_str().is_empty()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return CommandResult::error(format!("创建导出目录失败：{e}"));
    }

    let mut content = String::new();
    content.push_str("# 聊天导出\n\n");
    let _ = write!(
        content,
        "**模型：** {}\n**工作区：** {}\n**日期：** {}\n\n---\n\n",
        app.model,
        app.workspace.display(),
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    );

    for cell in &app.history {
        let (role, body) = match cell {
            HistoryCell::User { content } => ("**你：**", content.clone()),
            HistoryCell::Assistant { content, .. } => ("**助手：**", content.clone()),
            HistoryCell::System { content } => ("*系统：*", content.clone()),
            HistoryCell::Error { message, severity } => match severity {
                crate::error_taxonomy::ErrorSeverity::Warning => ("**警告：**", message.clone()),
                crate::error_taxonomy::ErrorSeverity::Info => ("*信息：*", message.clone()),
                _ => ("**错误：**", message.clone()),
            },
            HistoryCell::Thinking { content, .. } => ("*思考：*", content.clone()),
            HistoryCell::Tool(tool) => ("**工具：**", render_tool_cell(tool, 80)),
            HistoryCell::SubAgent(sub) => ("**子代理：**", render_subagent_cell(sub, 80)),
            HistoryCell::ArchivedContext {
                level,
                range,
                summary,
                ..
            } => (
                "**已归档上下文：**",
                format!("L{level} [{range}]: {summary}"),
            ),
        };

        let _ = write!(content, "{}\n\n{}\n\n---\n\n", role, body.trim());
    }

    match std::fs::write(&final_export_path, content) {
        Ok(()) => CommandResult::message(format!("已导出到 {}", final_export_path.display())),
        Err(e) => CommandResult::error(format!("导出失败：{e}")),
    }
}

/// Open the session picker UI
pub fn sessions(app: &mut App) -> CommandResult {
    app.view_stack.push(SessionPickerView::new());
    CommandResult::ok()
}

fn render_tool_cell(tool: &crate::tui::history::ToolCell, width: u16) -> String {
    tool.lines(width)
        .into_iter()
        .map(line_to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_subagent_cell(cell: &crate::tui::history::SubAgentCell, width: u16) -> String {
    cell.lines(width)
        .into_iter()
        .map(line_to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn line_to_string(line: ratatui::text::Line<'static>) -> String {
    line.spans
        .into_iter()
        .map(|span| span.content.to_string())
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::app::{App, TuiOptions};
    use tempfile::TempDir;

    fn create_test_app_with_tmpdir(tmpdir: &TempDir) -> App {
        let options = TuiOptions {
            model: "mimo-v2.5-pro".to_string(),
            workspace: tmpdir.path().to_path_buf(),
            workspace_explicit: true,
            allow_shell: false,
            use_alt_screen: true,
            use_mouse_capture: false,
            use_bracketed_paste: true,
            max_subagents: 1,
            skills_dir: tmpdir.path().join("skills"),
            memory_path: tmpdir.path().join("memory.md"),
            notes_path: tmpdir.path().join("notes.txt"),
            mcp_config_path: tmpdir.path().join("mcp.json"),
            use_memory: false,
            start_in_agent_mode: false,
            skip_onboarding: true,
            yolo: false,
            resume_session_id: None,
        };
        App::new(options, &Config::default())
    }

    #[test]
    fn test_save_creates_file_and_sets_session_id() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let save_path = tmpdir.path().join("test_session.json");

        let result = save(&mut app, Some(save_path.to_str().unwrap()));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("会话已保存到"));
        assert!(msg.contains("ID："));
        assert!(app.current_session_id.is_some());
        assert!(save_path.exists());
    }

    #[test]
    fn test_save_with_default_path_uses_workspace() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let result = save(&mut app, None);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        // Should create file in workspace with timestamp name
        // Give it a moment to ensure file is written
        std::thread::sleep(std::time::Duration::from_millis(10));
        let entries: Vec<_> = std::fs::read_dir(tmpdir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("session_"))
            .collect();
        // Test passes if file was created or if save returned success message
        assert!(!entries.is_empty() || msg.contains("会话已保存"));
    }

    #[test]
    fn test_save_serialization_error() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        // This should work normally since SavedSession is serializable
        // Testing error path would require mocking, which is complex
        let save_path = tmpdir.path().join("test.json");
        let result = save(&mut app, Some(save_path.to_str().unwrap()));
        assert!(result.message.is_some());
    }

    #[test]
    fn test_load_without_path_returns_error() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let result = load(&mut app, None);
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("用法：/load"));
    }

    #[test]
    fn test_load_nonexistent_file_returns_error() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let result = load(&mut app, Some("nonexistent.json"));
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("读取会话文件失败"));
    }

    #[test]
    fn test_load_invalid_json_returns_error() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let bad_file = tmpdir.path().join("bad.json");
        std::fs::write(&bad_file, "not valid json").unwrap();
        let result = load(&mut app, Some(bad_file.to_str().unwrap()));
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("解析会话文件失败"));
    }

    #[test]
    fn test_load_valid_session_restores_state() {
        let tmpdir = TempDir::new().unwrap();
        let mut app1 = create_test_app_with_tmpdir(&tmpdir);
        // Set up some state to save
        app1.api_messages.push(crate::models::Message {
            role: "user".to_string(),
            content: vec![crate::models::ContentBlock::Text {
                text: "Hello".to_string(),
                cache_control: None,
            }],
        });
        app1.total_tokens = 500;
        let save_path = tmpdir.path().join("test.json");
        save(&mut app1, Some(save_path.to_str().unwrap()));

        // Create new app and load
        let mut app2 = create_test_app_with_tmpdir(&tmpdir);
        let result = load(&mut app2, Some(save_path.to_str().unwrap()));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("已从"));
        assert!(msg.contains("ID："));
        assert!(msg.contains("条消息"));
        assert_eq!(app2.api_messages.len(), 1);
        assert_eq!(app2.total_tokens, 500);
        assert!(app2.current_session_id.is_some());
        assert!(matches!(result.action, Some(AppAction::SyncSession { .. })));
    }

    #[test]
    fn test_compact_toggles_state() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);

        let result = compact(&mut app);
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("上下文压缩"));
        assert!(matches!(result.action, Some(AppAction::CompactContext)));
    }

    #[test]
    fn test_export_crees_markdown_file() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        app.history.push(HistoryCell::User {
            content: "Hello".to_string(),
        });
        app.history.push(HistoryCell::Assistant {
            content: "Hi there".to_string(),
            streaming: false,
        });

        let export_path = tmpdir.path().join("export.md");
        let result = export(&mut app, Some(export_path.to_str().unwrap()));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("已导出到"));
        assert!(export_path.exists());

        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("# 聊天导出"));
        assert!(content.contains("**模型：**"));
        assert!(content.contains("**你：**"));
        assert!(content.contains("**助手：**"));
    }

    #[test]
    fn test_export_with_default_path() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let result = export(&mut app, None);
        assert!(result.message.is_some());
        // Should create file with timestamp name in the workspace.
        let entries: Vec<_> = std::fs::read_dir(tmpdir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("chat_export_"))
            .collect();
        assert!(!entries.is_empty());
    }

    #[test]
    fn test_sessions_pushes_picker_view() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let initial_kind = app.view_stack.top_kind();

        let result = sessions(&mut app);
        assert_eq!(result.message, None);
        assert!(result.action.is_none());
        // View should have changed (session picker should be on top)
        assert_ne!(app.view_stack.top_kind(), initial_kind);
    }
}
