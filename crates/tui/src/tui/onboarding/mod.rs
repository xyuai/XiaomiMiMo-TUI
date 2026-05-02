//! Onboarding flow rendering and helpers.

pub mod api_key;
pub mod trust_directory;
pub mod welcome;
pub mod workspace;

use std::path::{Path, PathBuf};

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

use crate::palette;
use crate::tui::app::{App, OnboardingState};

pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default().style(Style::default().bg(palette::XIAOMIMIMO_INK));
    f.render_widget(block, area);

    let content_width = 76.min(area.width.saturating_sub(4));
    let content_height = 22.min(area.height.saturating_sub(4));
    let content_area = Rect {
        x: (area.width - content_width) / 2,
        y: (area.height - content_height) / 2,
        width: content_width,
        height: content_height,
    };

    let lines = match app.onboarding {
        OnboardingState::Welcome => welcome::lines(),
        OnboardingState::Workspace => workspace::lines(app),
        OnboardingState::ApiKey => api_key::lines(app),
        OnboardingState::TrustDirectory => trust_directory::lines(app),
        OnboardingState::Tips => tips_lines(),
        OnboardingState::None => Vec::new(),
    };

    if !lines.is_empty() {
        let (step, total) = onboarding_step(app);
        let panel = Block::default()
            .title(Line::from(Span::styled(
                " XiaomiMiMo-TUI ",
                Style::default()
                    .fg(palette::XIAOMIMIMO_BLUE)
                    .add_modifier(Modifier::BOLD),
            )))
            .title_bottom(Line::from(Span::styled(
                format!(" 步骤 {step}/{total} "),
                Style::default()
                    .fg(palette::TEXT_MUTED)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette::BORDER_COLOR))
            .style(Style::default().bg(palette::XIAOMIMIMO_SLATE))
            .padding(Padding::new(2, 2, 1, 1));
        let inner = panel.inner(content_area);
        f.render_widget(panel, content_area);
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(paragraph, inner);
    }
}

fn onboarding_step(app: &App) -> (usize, usize) {
    let needs_trust = !app.trust_mode && needs_trust(&app.workspace);
    let workspace_step = if app.onboarding_needs_workspace { 1 } else { 0 };
    let api_key_step = if app.onboarding_needs_api_key { 1 } else { 0 };
    let mut total = 2; // Welcome + Tips
    if workspace_step == 1 {
        total += 1;
    }
    if api_key_step == 1 {
        total += 1;
    }
    if needs_trust {
        total += 1;
    }

    let step = match app.onboarding {
        OnboardingState::Welcome => 1,
        OnboardingState::Workspace => 2,
        OnboardingState::ApiKey => 2 + workspace_step,
        OnboardingState::TrustDirectory => 2 + workspace_step + api_key_step,
        OnboardingState::Tips => total,
        OnboardingState::None => total,
    };

    (step, total)
}

pub fn tips_lines() -> Vec<ratatui::text::Line<'static>> {
    use ratatui::style::Modifier;
    use ratatui::text::{Line, Span};

    vec![
        Line::from(Span::styled(
            "快速开始",
            Style::default()
                .fg(palette::XIAOMIMIMO_SKY)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::raw(
            "直接用自然语言描述任务。需要命令时按 Ctrl+K 或输入 /help。",
        )),
        Line::from(Span::raw(
            "底部输入框支持多行：Enter 发送，Alt+Enter 或 Ctrl+J 换行。",
        )),
        Line::from(Span::raw(
            "模式按任务选择：Plan 先规划，Agent 执行，YOLO 自动批准。",
        )),
        Line::from(Span::raw(
            "Ctrl+R 可恢复历史会话，Esc 可退出当前草稿或弹窗。",
        )),
        Line::from(vec![
            Span::styled("按 ", Style::default().fg(palette::TEXT_MUTED)),
            Span::styled(
                "Enter",
                Style::default()
                    .fg(palette::TEXT_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" 打开工作区", Style::default().fg(palette::TEXT_MUTED)),
        ]),
    ]
}

pub fn default_marker_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".xiaomimimo").join(".onboarded"))
}

pub fn is_onboarded() -> bool {
    default_marker_path().is_some_and(|path| path.exists())
}

pub fn mark_onboarded() -> std::io::Result<PathBuf> {
    let path = default_marker_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, "")?;
    Ok(path)
}

pub fn default_workspace_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join("XiaomiMiMo-Workspace"))
}

pub fn prepare_workspace(raw: &str, fallback: &Path) -> std::io::Result<PathBuf> {
    let raw = raw.trim().trim_matches('"').trim_matches('\'');
    let path = if raw.is_empty() {
        default_workspace_path().unwrap_or_else(|| fallback.to_path_buf())
    } else if let Some(stripped) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        dirs::home_dir()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
            })?
            .join(stripped)
    } else if raw == "~" {
        dirs::home_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Home directory not found")
        })?
    } else {
        PathBuf::from(raw)
    };

    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };

    if path.exists() && !path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Selected path exists but is not a directory",
        ));
    }

    std::fs::create_dir_all(&path)?;
    path.canonicalize()
}

pub fn needs_trust(workspace: &Path) -> bool {
    let markers = [
        workspace.join(".xiaomimimo").join("trusted"),
        workspace.join(".xiaomimimo").join("trust.json"),
    ];
    !markers.iter().any(|path| path.exists())
}

pub fn mark_trusted(workspace: &Path) -> std::io::Result<PathBuf> {
    let dir = workspace.join(".xiaomimimo");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("trusted");
    std::fs::write(&path, "")?;
    Ok(path)
}
