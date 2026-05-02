//! Workspace selection step for first-run onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::palette;
use crate::tui::app::App;

pub fn lines(app: &App) -> Vec<Line<'static>> {
    let current_workspace = app.workspace.display().to_string();
    let default_workspace = super::default_workspace_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| current_workspace.clone());
    let input = if app.onboarding_workspace_input.is_empty() {
        "（留空则创建/使用默认工作区）".to_string()
    } else {
        render_input_with_cursor(
            &app.onboarding_workspace_input,
            app.onboarding_workspace_cursor,
        )
    };
    let has_status = app.status_message.is_some();
    let status = app
        .status_message
        .clone()
        .unwrap_or_else(|| "输入一个文件夹路径，按 Enter 后会自动创建或使用该文件夹。".to_string());

    vec![
        Line::from(Span::styled(
            "选择工作区文件夹",
            Style::default()
                .fg(palette::XIAOMIMIMO_SKY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::raw(
            "XiaomiMiMo-TUI 会把会话、可信标记和部分工作文件放在工作区内。",
        )),
        Line::from(Span::raw(
            "建议选择一个专门的文件夹，避免直接使用 Downloads、Desktop 或磁盘根目录。",
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("当前启动目录：", Style::default().fg(palette::TEXT_MUTED)),
            Span::raw(current_workspace),
        ]),
        Line::from(vec![
            Span::styled("默认工作区：", Style::default().fg(palette::TEXT_MUTED)),
            Span::raw(default_workspace),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "工作区路径",
            Style::default()
                .fg(palette::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("> {input}"),
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                " 创建/使用该文件夹   ",
                Style::default().fg(palette::TEXT_MUTED),
            ),
            Span::styled("Ctrl+V", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" 粘贴路径", Style::default().fg(palette::TEXT_MUTED)),
        ]),
        Line::from(vec![
            Span::styled("←/→", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" 移动光标   ", Style::default().fg(palette::TEXT_MUTED)),
            Span::styled("Backspace", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" 删除   ", Style::default().fg(palette::TEXT_MUTED)),
            Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" 返回欢迎页", Style::default().fg(palette::TEXT_MUTED)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            status,
            Style::default().fg(if has_status {
                palette::STATUS_WARNING
            } else {
                palette::TEXT_MUTED
            }),
        )),
    ]
}

fn render_input_with_cursor(input: &str, cursor: usize) -> String {
    let char_len = input.chars().count();
    let cursor = cursor.min(char_len);
    let byte_index = input
        .char_indices()
        .nth(cursor)
        .map(|(idx, _)| idx)
        .unwrap_or(input.len());
    let (left, right) = input.split_at(byte_index);
    format!("{left}▌{right}")
}
