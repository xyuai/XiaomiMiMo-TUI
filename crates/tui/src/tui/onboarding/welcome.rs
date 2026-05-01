//! Welcome screen content for onboarding.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::palette;

pub fn lines() -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            "XiaomiMiMo-TUI",
            Style::default()
                .fg(palette::XIAOMIMIMO_BLUE)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("版本 {}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "面向 Xiaomi MiMo 的终端工作区。",
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "接下来会填写 API Key、确认工作区信任，然后进入对话。",
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(Span::styled(
            "默认使用 Token Plan 套餐专属 API 与 Base URL：https://token-plan-cn.xiaomimimo.com/v1",
            Style::default().fg(palette::TEXT_MUTED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "按 Enter 继续。",
            Style::default().fg(palette::TEXT_PRIMARY),
        )),
        Line::from(Span::styled(
            "随时按 Ctrl+C 退出。",
            Style::default().fg(palette::TEXT_MUTED),
        )),
    ]
}
