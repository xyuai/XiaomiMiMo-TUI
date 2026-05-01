//! API key entry screen for onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::palette;
use crate::tui::app::App;

pub fn lines(app: &App) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            "Token Plan API Key 配置",
            Style::default()
                .fg(palette::XIAOMIMIMO_SKY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "请输入 Token Plan 套餐专属 API Key（XIAOMIMIMO_API_KEY）以继续。",
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "默认专属 Base URL: https://token-plan-cn.xiaomimimo.com/v1",
            Style::default().fg(palette::XIAOMIMIMO_SKY),
        )),
        Line::from(Span::styled(
            "Anthropic 兼容地址: https://token-plan-cn.xiaomimimo.com/anthropic",
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "请完整粘贴 Key，不要包含空格或换行。",
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
    ];

    let masked = mask_key(&app.api_key_input);
    let display = if masked.is_empty() {
        "（在这里粘贴 API Key）"
    } else {
        masked.as_str()
    };
    lines.push(Line::from(vec![
        Span::styled("Key：", Style::default().fg(palette::TEXT_MUTED)),
        Span::styled(
            display.to_string(),
            Style::default()
                .fg(palette::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    if let Some(message) = app.status_message.as_deref() {
        lines.push(Line::from(Span::styled(
            message.to_string(),
            Style::default().fg(palette::STATUS_WARNING),
        )));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "按 Enter 保存，按 Esc 返回。",
        Style::default().fg(palette::TEXT_MUTED),
    )));

    lines
}

fn mask_key(input: &str) -> String {
    let trimmed = input.trim();
    let len = trimmed.chars().count();
    if len == 0 {
        return String::new();
    }
    if len <= 4 {
        return "*".repeat(len);
    }
    let visible: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{}{}", "*".repeat(len - 4), visible)
}
