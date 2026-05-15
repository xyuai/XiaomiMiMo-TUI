//! Markdown rendering for TUI transcript lines.
//!
//! ## Width-independent parse vs width-dependent render (CX#6)
//!
//! The previous renderer was a single function `render_markdown(content, width)`
//! that scanned the source, classified each line (heading / list / code-fence /
//! paragraph / link), and word-wrapped to `Line<'static>` in one pass. That meant
//! every terminal resize forced a full re-parse of the source for every visible
//! cell — wasted work on the streaming cell whose content is changing anyway.
//!
//! The codex tui solves this by splitting parse from render. We mirror that:
//!
//! * [`parse`] turns the markdown source into a [`ParsedMarkdown`] AST: a vector
//!   of width-independent [`Block`]s. The block kind already records all the
//!   classification decisions (heading level, list bullet, code block membership)
//!   that don't depend on width.
//! * [`render_parsed`] takes a `ParsedMarkdown` plus a width and a base style and
//!   produces `Vec<Line<'static>>`. It only does word-wrap and span styling.
//!
//! [`render_markdown`] is kept as a thin convenience that does both — useful for
//! callers (Thinking body, message body) that don't want to manage the cache.
//!
//! The transcript cache layer (see `tui/transcript.rs`) caches the parsed AST per
//! cell and re-runs only the render step on width changes. That makes resize a
//! re-flow operation rather than a re-parse + re-flow operation.

#[cfg(test)]
use std::cell::Cell;

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::palette;

// Thread-local counter incremented every time `parse` runs. Used by tests to
// prove that width-only changes hit the cached-AST path and skip parsing.
// Thread-local (not global atomic) so concurrent tests calling `parse()` can't
// pollute each other's counters.
#[cfg(test)]
thread_local! {
    static PARSE_INVOCATIONS: Cell<u64> = const { Cell::new(0) };
}

#[cfg(test)]
#[must_use]
pub fn parse_invocation_count() -> u64 {
    PARSE_INVOCATIONS.with(|c| c.get())
}

#[cfg(test)]
pub fn reset_parse_invocation_count() {
    PARSE_INVOCATIONS.with(|c| c.set(0));
}

/// One classified line of markdown source, width-independent.
///
/// All decisions that depend only on the source text (heading level, bullet
/// kind, whether we're inside a fenced code block, paragraph text) are made at
/// parse time. Width-dependent layout (word-wrap, prefix indent) is deferred to
/// the render step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// `# heading text`. Includes the heading level (1..6).
    Heading { level: usize, text: String },
    /// A horizontal rule emitted under a level-1 heading.
    HeadingRule,
    /// A standalone `---` / `***` / `___` horizontal rule.
    HorizontalRule,
    /// A bullet (`-`/`*`) or ordered (`1.`) list item with its prefix and body.
    ListItem { bullet: String, text: String },
    /// A line inside a fenced code block. Fences themselves are dropped.
    Code { line: String },
    /// A table row: cells split on `|`. Separator rows (`|---|`) are dropped.
    TableRow(Vec<String>),
    /// A non-empty paragraph line that may contain inline links.
    Paragraph { text: String },
    /// An empty source line, preserved so paragraph spacing survives.
    Blank,
}

/// Width-independent parsed-markdown AST for one cell's source.
///
/// Wrapped in `Arc` at the cache layer so the cache can hand the same AST to
/// many render calls without copying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMarkdown {
    blocks: Vec<Block>,
}

/// Parse markdown source into a width-independent block AST.
///
/// This is a small line-oriented parser tuned for the patterns we render:
/// fenced code blocks, ATX headings, dash/star/numbered list items, and plain
/// paragraphs with optional links. It does not attempt to handle every CommonMark
/// edge case — that's intentional. The renderer will treat anything we don't
/// classify as `Block::Paragraph`.
#[must_use]
pub fn parse(content: &str) -> ParsedMarkdown {
    #[cfg(test)]
    PARSE_INVOCATIONS.with(|c| c.set(c.get() + 1));

    let mut blocks = Vec::new();
    let mut in_code_block = false;

    for raw_line in content.lines() {
        let trimmed = raw_line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            blocks.push(Block::Code {
                line: raw_line.to_string(),
            });
            continue;
        }

        if let Some((level, text)) = parse_heading(trimmed) {
            blocks.push(Block::Heading {
                level,
                text: text.to_string(),
            });
            if level == 1 {
                blocks.push(Block::HeadingRule);
            }
            continue;
        }

        if let Some((bullet, text)) = parse_list_item(trimmed) {
            blocks.push(Block::ListItem {
                bullet,
                text: text.to_string(),
            });
            continue;
        }

        if is_horizontal_rule(trimmed) {
            blocks.push(Block::HorizontalRule);
            continue;
        }

        match parse_table_row(trimmed) {
            Some(cells) => {
                blocks.push(Block::TableRow(cells));
                continue;
            }
            None if trimmed.starts_with('|') => continue, // separator row — drop it
            None => {}
        }

        if raw_line.is_empty() {
            blocks.push(Block::Blank);
            continue;
        }

        blocks.push(Block::Paragraph {
            text: trimmed.to_string(),
        });
    }

    ParsedMarkdown { blocks }
}

/// Render a parsed-markdown AST at the given terminal width.
///
/// This is the width-dependent half: word-wrapping, link styling, code-block
/// formatting. The AST is owned by the caller (typically the transcript cache),
/// so width-only changes can call `render_parsed` again with the same AST and
/// skip the parse step entirely.
#[must_use]
pub fn render_parsed(parsed: &ParsedMarkdown, width: u16, base_style: Style) -> Vec<Line<'static>> {
    let width = width.max(1) as usize;
    let mut out: Vec<Line<'static>> = Vec::with_capacity(parsed.blocks.len());

    for block in &parsed.blocks {
        match block {
            Block::Heading { text, .. } => {
                let style = Style::default()
                    .fg(palette::XIAOMIMIMO_SKY)
                    .add_modifier(Modifier::BOLD);
                out.extend(render_wrapped_line(text, width, style, false));
            }
            Block::HeadingRule => {
                out.push(Line::from(Span::styled(
                    "─".repeat(width.min(40)),
                    Style::default().fg(palette::TEXT_DIM),
                )));
            }
            Block::HorizontalRule => {
                out.push(Line::from(Span::styled(
                    "─".repeat(width.min(60)),
                    Style::default().fg(palette::TEXT_DIM),
                )));
            }
            Block::TableRow(cells) => {
                out.extend(render_table_row(cells, width, base_style));
            }
            Block::ListItem { bullet, text } => {
                let bullet_style = Style::default().fg(palette::XIAOMIMIMO_SKY);
                out.extend(render_list_line(
                    bullet,
                    text,
                    width,
                    bullet_style,
                    base_style,
                ));
            }
            Block::Code { line } => {
                let code_style = Style::default()
                    .fg(palette::XIAOMIMIMO_SKY)
                    .add_modifier(Modifier::ITALIC);
                out.extend(render_wrapped_line(line, width, code_style, true));
            }
            Block::Paragraph { text } => {
                let link_style = Style::default()
                    .fg(palette::XIAOMIMIMO_BLUE)
                    .add_modifier(Modifier::UNDERLINED);
                out.extend(render_line_with_links(text, width, base_style, link_style));
            }
            Block::Blank => {
                // Preserve paragraph spacing. The original renderer also pushed
                // a blank line for empty source lines that fell through the
                // paragraph branch; mirror that exactly.
                out.push(Line::from(""));
            }
        }
    }

    if out.is_empty() {
        out.push(Line::from(""));
    }

    out
}

/// Convenience wrapper: parse + render in one call.
///
/// Equivalent to `render_parsed(&parse(content), width, base_style)`. Callers
/// that don't manage their own cache (the Thinking body, the immediate message
/// body) use this.
#[must_use]
pub fn render_markdown(content: &str, width: u16, base_style: Style) -> Vec<Line<'static>> {
    let parsed = parse(content);
    render_parsed(&parsed, width, base_style)
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 {
        return None;
    }
    let text = trimmed[hashes..].trim();
    if text.is_empty() {
        None
    } else {
        Some((hashes, text))
    }
}

fn parse_list_item(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        return Some(("-".to_string(), trimmed[2..].trim()));
    }
    let bytes = trimmed.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    if idx == 0 || idx >= bytes.len() || bytes[idx] != b'.' {
        return None;
    }
    let rest = &trimmed[idx + 1..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some((format!("{}.", &trimmed[..idx]), rest.trim_start()))
}

fn render_wrapped_line(
    line: &str,
    width: usize,
    style: Style,
    indent_code: bool,
) -> Vec<Line<'static>> {
    let prefix = if indent_code { "  " } else { "" };
    let prefix_width = prefix.width();
    let available = width.saturating_sub(prefix_width).max(1);
    let wrapped = wrap_text(line, available);
    let mut out = Vec::new();

    for (idx, chunk) in wrapped.into_iter().enumerate() {
        if idx == 0 {
            out.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(chunk, style),
            ]));
        } else {
            out.push(Line::from(vec![
                Span::raw(" ".repeat(prefix_width)),
                Span::styled(chunk, style),
            ]));
        }
    }

    out
}

fn render_list_line(
    bullet: &str,
    text: &str,
    width: usize,
    bullet_style: Style,
    text_style: Style,
) -> Vec<Line<'static>> {
    let bullet_prefix = format!("{bullet} ");
    let bullet_width = bullet_prefix.width();
    let available = width.saturating_sub(bullet_width).max(1);
    let wrapped = render_line_with_links(text, available, text_style, link_style());

    let mut out = Vec::new();
    for (idx, line) in wrapped.into_iter().enumerate() {
        if idx == 0 {
            let mut spans = vec![Span::styled(bullet_prefix.clone(), bullet_style)];
            spans.extend(line.spans);
            out.push(Line::from(spans));
        } else {
            let mut spans = vec![Span::raw(" ".repeat(bullet_width))];
            spans.extend(line.spans);
            out.push(Line::from(spans));
        }
    }
    out
}

fn render_line_with_links(
    line: &str,
    width: usize,
    base_style: Style,
    link_style: Style,
) -> Vec<Line<'static>> {
    if line.trim().is_empty() {
        return vec![Line::from("")];
    }

    // Flatten inline tokens into (word, style) pairs preserving inter-token spaces.
    let tokens = parse_inline_spans(line, base_style, link_style);
    let mut words: Vec<(String, Style)> = Vec::new();
    for (text, style) in tokens {
        let mut first = true;
        for part in text.split(' ') {
            if !first {
                // The space consumed by split — attach as a plain space word
                // so the wrap loop can decide whether to keep or break it.
                words.push((" ".to_string(), style));
            }
            if !part.is_empty() {
                words.push((part.to_string(), style));
            }
            first = false;
        }
    }

    let mut lines = Vec::new();
    let mut current_spans: Vec<Span> = Vec::new();
    let mut current_width = 0usize;

    for (word, style) in words {
        let ww = word.width();
        if word == " " {
            // Space: emit only if we're mid-line and it fits; otherwise drop
            // (it's a potential wrap point, not content).
            if !current_spans.is_empty() && current_width < width {
                current_spans.push(Span::raw(" "));
                current_width += 1;
            }
            continue;
        }
        // Wrap before this word if it doesn't fit.
        if current_width > 0 && current_width + ww > width {
            // Trim trailing space span before breaking.
            if let Some(last) = current_spans.last()
                && last.content.as_ref() == " "
            {
                current_spans.pop();
            }
            lines.push(Line::from(current_spans));
            current_spans = Vec::new();
            current_width = 0;
        }
        if ww > width {
            if !current_spans.is_empty() {
                lines.push(Line::from(current_spans));
                current_spans = Vec::new();
                current_width = 0;
            }
            let mut chunks = hard_wrap_segment(&word, width);
            if let Some(tail) = chunks.pop() {
                for chunk in chunks {
                    lines.push(Line::from(Span::styled(chunk, style)));
                }
                current_width = tail.width();
                current_spans.push(Span::styled(tail, style));
            }
            continue;
        }
        current_spans.push(Span::styled(word, style));
        current_width += ww;
    }

    if !current_spans.is_empty() {
        lines.push(Line::from(current_spans));
    }
    if lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines
}

/// Parse an entire line into (text, style) segments, handling **bold**,
/// *italic*, and bare URLs that may span multiple words.
fn parse_inline_spans(line: &str, base_style: Style, link_style: Style) -> Vec<(String, Style)> {
    let bold_style = base_style.add_modifier(Modifier::BOLD);
    let italic_style = base_style.add_modifier(Modifier::ITALIC);
    let mut out = Vec::new();
    let mut rest = line;

    while !rest.is_empty() {
        // **bold**
        if let Some(end) = rest.strip_prefix("**").and_then(|s| s.find("**")) {
            let inner = &rest[2..2 + end];
            out.push((inner.to_string(), bold_style));
            rest = &rest[2 + end + 2..];
            continue;
        }
        // __bold__
        if let Some(end) = rest.strip_prefix("__").and_then(|s| s.find("__")) {
            let inner = &rest[2..2 + end];
            out.push((inner.to_string(), bold_style));
            rest = &rest[2 + end + 2..];
            continue;
        }
        // *italic*
        if rest.starts_with('*')
            && !rest.starts_with("**")
            && let Some(end) = rest[1..].find('*')
        {
            let inner = &rest[1..1 + end];
            out.push((inner.to_string(), italic_style));
            rest = &rest[1 + end + 1..];
            continue;
        }
        // _italic_
        if rest.starts_with('_')
            && !rest.starts_with("__")
            && let Some(end) = rest[1..].find('_')
        {
            let inner = &rest[1..1 + end];
            out.push((inner.to_string(), italic_style));
            rest = &rest[1 + end + 1..];
            continue;
        }
        // URL: consume until whitespace
        if rest.starts_with("http://") || rest.starts_with("https://") {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            out.push((rest[..end].to_string(), link_style));
            rest = &rest[end..];
            continue;
        }
        // Plain text: consume until next marker or URL; always advance at least 1 char.
        let next = find_next_marker(rest).max(rest.chars().next().map_or(1, |c| c.len_utf8()));
        out.push((rest[..next].to_string(), base_style));
        rest = &rest[next..];
    }
    out
}

/// Find the index of the next inline marker (`**`, `__`, `*`, `_`, `http`)
/// in `s`, or `s.len()` if none found.
fn find_next_marker(s: &str) -> usize {
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        let ch_len = s[i..].chars().next().map_or(1, |c| c.len_utf8());
        let slice = &s[i..];
        if slice.starts_with("**")
            || slice.starts_with("__")
            || (slice.starts_with('*') && !slice.starts_with("**"))
            || (slice.starts_with('_') && !slice.starts_with("__"))
            || slice.starts_with("http://")
            || slice.starts_with("https://")
        {
            return i;
        }
        i += ch_len;
    }
    s.len()
}

fn is_horizontal_rule(line: &str) -> bool {
    let stripped: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    (stripped.chars().all(|c| c == '-')
        || stripped.chars().all(|c| c == '*')
        || stripped.chars().all(|c| c == '_'))
        && stripped.len() >= 3
}

/// Parse a markdown table row like `| foo | bar |` into trimmed cell strings.
/// Returns `None` for separator rows (`|---|---|`).
fn parse_table_row(line: &str) -> Option<Vec<String>> {
    if !line.starts_with('|') {
        return None;
    }
    let inner = line.trim_matches('|');
    let cells: Vec<String> = inner.split('|').map(|c| c.trim().to_string()).collect();
    // Separator row: every non-empty cell is only dashes/colons/spaces
    if cells
        .iter()
        .all(|c| c.is_empty() || c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '))
    {
        return None;
    }
    Some(cells)
}

fn render_table_row(cells: &[String], width: usize, base_style: Style) -> Vec<Line<'static>> {
    if cells.is_empty() {
        return vec![Line::from("")];
    }
    let col_width = (width.saturating_sub(3 * cells.len() + 1)) / cells.len();
    let col_width = col_width.max(4);
    let sep_style = Style::default().fg(palette::TEXT_DIM);
    let wrapped_cells: Vec<Vec<String>> = cells
        .iter()
        .map(|cell| wrap_text(cell, col_width))
        .collect();
    let row_count = wrapped_cells
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1)
        .max(1);
    let mut rows = Vec::new();

    for row in 0..row_count {
        let mut spans: Vec<Span> = vec![Span::styled("\u{2502} ".to_string(), sep_style)];
        for (i, cell_rows) in wrapped_cells.iter().enumerate() {
            let cell = cell_rows.get(row).map(String::as_str).unwrap_or("");
            let cell_spans: Vec<(String, Style)> =
                parse_inline_spans(cell, base_style, link_style());
            let cell_width: usize = cell_spans.iter().map(|(t, _)| t.width()).sum();
            let pad = col_width.saturating_sub(cell_width);
            for (text, style) in cell_spans {
                spans.push(Span::styled(text, style));
            }
            spans.push(Span::raw(" ".repeat(pad)));
            if i + 1 < cells.len() {
                spans.push(Span::styled(" \u{2502} ".to_string(), sep_style));
            } else {
                spans.push(Span::styled(" \u{2502}".to_string(), sep_style));
            }
        }
        rows.push(Line::from(spans));
    }

    rows
}

fn link_style() -> Style {
    Style::default()
        .fg(palette::XIAOMIMIMO_BLUE)
        .add_modifier(Modifier::UNDERLINED)
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for word in text.split_whitespace() {
        let word_width = word.width();
        if word_width > width {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
            }
            lines.extend(hard_wrap_segment(word, width));
            continue;
        }
        let additional = if current.is_empty() {
            word_width
        } else {
            word_width + 1
        };
        if current_width + additional > width && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
            current_width = word_width;
        } else {
            if !current.is_empty() {
                current.push(' ');
                current_width += 1;
            }
            current.push_str(word);
            current_width += word_width;
        }
    }

    if current.is_empty() {
        lines.push(String::new());
    } else {
        lines.push(current);
    }

    lines
}

fn hard_wrap_segment(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    for ch in text.chars() {
        let char_width = ch.width().unwrap_or(1);
        if current_width + char_width > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push(ch);
        current_width += char_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    fn collect_text(lines: &[Line<'static>]) -> Vec<String> {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn render_markdown_matches_parse_then_render() {
        // The convenience wrapper must produce byte-identical output to the
        // explicit two-step path. Without this guarantee the transcript cache
        // and the live render diverge.
        let source = "# Title\n\nA paragraph with a https://example.com link.\n\n- one\n- two\n```\ncode\n```";
        let direct = render_markdown(source, 40, Style::default());
        let parsed = parse(source);
        let two_step = render_parsed(&parsed, 40, Style::default());
        assert_eq!(collect_text(&direct), collect_text(&two_step));
    }

    #[test]
    fn parse_is_width_independent() {
        // Same source, two parses, must produce identical AST. (Sanity:
        // parse must not depend on hidden global state like terminal width.)
        let source = "Hello\n\n## Heading\n- list\n";
        let a = parse(source);
        let b = parse(source);
        assert_eq!(a, b);
    }

    #[test]
    fn render_parsed_word_wrap_changes_with_width() {
        // The same AST must produce different layouts at different widths;
        // otherwise the split is decorative, not functional.
        let parsed = parse("alpha beta gamma delta epsilon zeta");
        let wide = render_parsed(&parsed, 80, Style::default());
        let narrow = render_parsed(&parsed, 10, Style::default());
        assert!(
            narrow.len() > wide.len(),
            "narrow should produce more lines"
        );
    }

    #[test]
    fn parse_invocations_increment() {
        // Counter is thread-local, so concurrent tests calling `parse()`
        // can't pollute each other.
        reset_parse_invocation_count();
        let _ = parse("hello\n");
        let _ = parse("world\n");
        assert_eq!(parse_invocation_count(), 2);
    }

    #[test]
    fn render_parsed_does_not_call_parse() {
        // Width-only changes must hit only the render path. This is the
        // perf invariant CX#6 was filed for.
        let parsed = parse("multiline\nsource\nwith several\nlines\n");
        reset_parse_invocation_count();
        let _ = render_parsed(&parsed, 80, Style::default());
        let _ = render_parsed(&parsed, 40, Style::default());
        let _ = render_parsed(&parsed, 20, Style::default());
        assert_eq!(
            parse_invocation_count(),
            0,
            "render_parsed must not call parse"
        );
    }

    #[test]
    fn fenced_code_block_collected_in_parse() {
        let parsed = parse("text\n```\ncode line one\ncode line two\n```\nmore\n");
        let blocks = &parsed.blocks;
        // text paragraph, two code lines, more paragraph (fences are dropped)
        let code_lines: Vec<_> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Code { line } => Some(line.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(code_lines, vec!["code line one", "code line two"]);
    }

    #[test]
    fn ordered_and_unordered_list_items_parse() {
        let parsed = parse("- alpha\n* beta\n1. gamma\n");
        let items: Vec<_> = parsed
            .blocks
            .iter()
            .filter_map(|b| match b {
                Block::ListItem { bullet, text } => Some((bullet.as_str(), text.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(items, vec![("-", "alpha"), ("-", "beta"), ("1.", "gamma")]);
    }

    #[test]
    fn table_separator_row_is_dropped() {
        let src = "| property | detail |\n|----------|------|\n| **language** | Rust 1.85+ |\n";
        let parsed = parse(src);
        let table_rows: Vec<_> = parsed
            .blocks
            .iter()
            .filter(|block| matches!(block, Block::TableRow(_)))
            .collect();
        assert_eq!(table_rows.len(), 2);
    }

    #[test]
    fn bold_and_italic_markers_are_stripped_in_render() {
        let src = "This is a **Rust workspace** with *multiple crates*.\n";
        let lines = render_markdown(src, 80, Style::default());
        let text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert!(!text.contains("**"), "bold markers leaked: {text:?}");
        assert!(
            !text.contains("*multiple crates*"),
            "italic markers leaked: {text:?}"
        );
        assert!(text.contains("Rust workspace"));
        assert!(text.contains("multiple crates"));
    }

    #[test]
    fn horizontal_rule_renders_as_rule_not_paragraph() {
        let lines = render_markdown("---\n", 20, Style::default());
        let text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert!(text.contains('\u{2500}'));
        assert!(!text.contains("---"));
    }

    #[test]
    fn table_renders_with_box_separator() {
        let src = "| file | change |\n|---|---|\n| foo.rs | rewrite |\n";
        let lines = render_markdown(src, 60, Style::default());
        let text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert!(
            text.contains('\u{2502}'),
            "table separator missing: {text:?}"
        );
        assert!(!text.contains("|---|"), "separator row leaked: {text:?}");
    }

    #[test]
    fn paragraph_hard_wraps_overlong_cjk_runs() {
        let source = "这是一个非常长的中文字符串".repeat(4);
        let lines = render_markdown(&source, 12, Style::default());
        let text_lines = collect_text(&lines);
        assert!(text_lines.len() > 1);
        assert!(text_lines.iter().all(|line| line.width() <= 12));
        assert_eq!(text_lines.join(""), source);
    }

    #[test]
    fn table_long_cells_wrap_without_ellipsis() {
        let long_cell = "这是一个非常长的中文表格单元格".repeat(3);
        let src = format!("| item | detail |\n|---|---|\n| a | {long_cell} |\n");
        let lines = render_markdown(&src, 36, Style::default());
        let text = collect_text(&lines).join("\n");
        assert!(!text.contains('…'));
        assert!(text.contains("中文表格"));
        assert!(lines.len() > 2, "long cell should wrap: {text:?}");
    }

    #[test]
    fn unclosed_inline_marker_does_not_loop() {
        let lines = render_markdown("prefix **unclosed marker", 80, Style::default());
        let text: String = lines
            .iter()
            .flat_map(|line| line.spans.iter().map(|span| span.content.as_ref()))
            .collect();
        assert!(text.contains("prefix **unclosed marker"));
    }
}
