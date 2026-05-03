//! Note command: append to persistent notes file

use crate::tui::app::App;
use std::fs;
use std::io::Write;

use super::CommandResult;

/// Append a note to the persistent notes file
pub fn note(app: &mut App, content: Option<&str>) -> CommandResult {
    let note_content = match content {
        Some(c) => c.trim(),
        None => {
            return CommandResult::error("用法：/note <text>");
        }
    };

    if note_content.is_empty() {
        return CommandResult::error("备注内容不能为空");
    }

    // Determine notes path: workspace/.xiaomimimo/notes.md
    let notes_path = app.workspace.join(".xiaomimimo").join("notes.md");

    // Ensure parent directory exists
    if let Some(parent) = notes_path.parent()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return CommandResult::error(format!("创建备注目录失败：{e}"));
    }

    // Append to notes file
    let mut file = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&notes_path)
    {
        Ok(f) => f,
        Err(e) => {
            return CommandResult::error(format!("打开备注文件失败：{e}"));
        }
    };

    // Write separator and note content
    if let Err(e) = writeln!(file, "\n---\n{}", note_content) {
        return CommandResult::error(format!("写入备注失败：{e}"));
    }

    CommandResult::message(format!("备注已追加到 {}", notes_path.display()))
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
    fn test_note_without_content_returns_error() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let result = note(&mut app, None);
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("用法：/note"));
    }

    #[test]
    fn test_note_with_empty_content_returns_error() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let result = note(&mut app, Some("   "));
        assert!(result.message.is_some());
        assert!(result.message.unwrap().contains("不能为空"));
    }

    #[test]
    fn test_note_appends_to_file() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        let result = note(&mut app, Some("Test note content"));
        assert!(result.message.is_some());
        let msg = result.message.unwrap();
        assert!(msg.contains("备注已追加到"));

        let notes_path = tmpdir.path().join(".xiaomimimo").join("notes.md");
        assert!(notes_path.exists());
        let content = std::fs::read_to_string(&notes_path).unwrap();
        assert!(content.contains("Test note content"));
    }

    #[test]
    fn test_note_multiple_appends() {
        let tmpdir = TempDir::new().unwrap();
        let mut app = create_test_app_with_tmpdir(&tmpdir);
        note(&mut app, Some("First note"));
        note(&mut app, Some("Second note"));

        let notes_path = tmpdir.path().join(".xiaomimimo").join("notes.md");
        let content = std::fs::read_to_string(&notes_path).unwrap();
        assert!(content.contains("First note"));
        assert!(content.contains("Second note"));
        // Should have two separators
        assert_eq!(content.matches("---").count(), 2);
    }
}
