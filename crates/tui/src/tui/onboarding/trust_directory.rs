//! Workspace trust prompt for onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::palette;
use crate::tui::app::App;

pub fn lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "信任工作区",
        Style::default()
            .fg(palette::XIAOMIMIMO_SKY)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "是否允许 XiaomiMiMo-TUI 访问此工作区文件？",
        Style::default().fg(palette::TEXT_PRIMARY),
    )));
    lines.push(Line::from(Span::styled(
        format!("工作区：{}", app.workspace.display()),
        Style::default().fg(palette::TEXT_MUTED),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Y = 信任当前工作区，允许文件、搜索和代理工具处理任务需要的文件。",
        Style::default().fg(palette::TEXT_MUTED),
    )));
    lines.push(Line::from(Span::styled(
        "N = 暂不信任，文件访问将限制在当前工作区并按需确认。",
        Style::default().fg(palette::TEXT_MUTED),
    )));
    if let Some(message) = app.status_message.as_deref() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            message.to_string(),
            Style::default().fg(palette::STATUS_WARNING),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("按 ", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(
            "Y",
            Style::default()
                .fg(palette::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 信任，", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(
            "N",
            Style::default()
                .fg(palette::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" 跳过", Style::default().fg(palette::TEXT_MUTED)),
    ]));
    lines
}
