//! Utility helpers shared across the `XiaomiMiMo` CLI.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::models::{ContentBlock, Message};
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde_json::Value;

// === Project Mapping Helpers ===

/// Identify if a file is a "key" file for project identification.
#[must_use]
pub fn is_key_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    matches!(
        file_name.to_lowercase().as_str(),
        "cargo.toml"
            | "package.json"
            | "requirements.txt"
            | "build.gradle"
            | "pom.xml"
            | "readme.md"
            | "agents.md"
            | "claude.md"
            | "makefile"
            | "dockerfile"
            | "main.rs"
            | "lib.rs"
            | "index.js"
            | "index.ts"
            | "app.py"
    )
}

/// Generate a high-level summary of the project based on key files.
#[must_use]
pub fn summarize_project(root: &Path) -> String {
    let mut key_files = Vec::new();

    let mut builder = WalkBuilder::new(root);
    builder.hidden(false).follow_links(true).max_depth(Some(2));
    let walker = builder.build();

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if is_key_file(entry.path())
            && let Ok(rel) = entry.path().strip_prefix(root)
        {
            key_files.push(rel.to_string_lossy().to_string());
        }
    }

    if key_files.is_empty() {
        return "Unknown project type".to_string();
    }

    let mut types = Vec::new();
    if key_files
        .iter()
        .any(|f| f.to_lowercase().contains("cargo.toml"))
    {
        types.push("Rust");
    }
    if key_files
        .iter()
        .any(|f| f.to_lowercase().contains("package.json"))
    {
        types.push("JavaScript/Node.js");
    }
    if key_files
        .iter()
        .any(|f| f.to_lowercase().contains("requirements.txt"))
    {
        types.push("Python");
    }

    if types.is_empty() {
        format!("Project with key files: {}", key_files.join(", "))
    } else {
        format!("A {} project", types.join(" and "))
    }
}

/// Render a path for user-facing display with the home directory contracted
/// to `~`. Do not use this for persisted paths or model-visible paths.
#[must_use]
#[allow(dead_code)]
pub fn display_path(path: &Path) -> String {
    let Some(home) = dirs::home_dir() else {
        return path.display().to_string();
    };
    if let Ok(rest) = path.strip_prefix(&home) {
        if rest.as_os_str().is_empty() {
            return "~".to_string();
        }
        let sep = std::path::MAIN_SEPARATOR;
        return format!("~{sep}{}", rest.display());
    }
    path.display().to_string()
}

/// Generate a tree-like view of the project structure.
#[must_use]
pub fn project_tree(root: &Path, max_depth: usize) -> String {
    let mut tree_lines = Vec::new();

    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .follow_links(true)
        .max_depth(Some(max_depth + 1));
    let walker = builder.build();

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        let depth = entry.depth();

        if depth == 0 || depth > max_depth {
            continue;
        }

        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let indent = "  ".repeat(depth - 1);
        let prefix = if entry.file_type().is_some_and(|ft| ft.is_dir()) {
            "DIR: "
        } else {
            "FILE: "
        };

        tree_lines.push(format!(
            "{}{}{}",
            indent,
            prefix,
            rel_path.file_name().unwrap_or_default().to_string_lossy()
        ));
    }

    tree_lines.join("\n")
}

// === Filesystem Helpers ===

/// Atomically write `contents` to `path` using a same-directory temporary file,
/// fsync, and rename. Readers see either the previous complete file or the new
/// complete file, never a partially-written config/session.
pub fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("path has no parent directory: {}", path.display()),
        )
    })?;
    std::fs::create_dir_all(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(contents)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|err| err.error)?;
    // Best effort: fsync the directory entry on platforms that allow opening
    // directories as files. Windows often rejects this, so ignore failures.
    let _ = std::fs::File::open(parent).and_then(|dir| dir.sync_all());
    Ok(())
}

/// Spawn a tokio task with panic supervision. The panic is logged and a
/// best-effort crash dump is written under `~/.xiaomimimo/crashes/` so a
/// background task panic does not silently disappear.
pub fn spawn_supervised<F>(
    name: &'static str,
    location: &'static std::panic::Location<'static>,
    future: F,
) -> tokio::task::JoinHandle<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    tokio::spawn(async move {
        use futures_util::FutureExt;
        let result = std::panic::AssertUnwindSafe(future).catch_unwind().await;
        if let Err(panic_info) = result {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            tracing::error!(
                target: "panic",
                "Task '{name}' panicked at {}: {msg}",
                location,
            );
            let _ = write_panic_dump(name, location, &msg);
        }
    })
}

fn write_panic_dump(
    name: &str,
    location: &std::panic::Location<'_>,
    message: &str,
) -> std::io::Result<()> {
    let home = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    let crash_dir = home.join(".xiaomimimo").join("crashes");
    std::fs::create_dir_all(&crash_dir)?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let safe_name: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let path = crash_dir.join(format!("{timestamp}-{safe_name}.log"));
    let contents =
        format!("Task: {name}\nLocation: {location}\nTimestamp: {timestamp}\nPanic: {message}\n");
    write_atomic(&path, contents.as_bytes())
}

#[allow(dead_code)]
pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))
}

/// Render JSON with pretty formatting, falling back to a compact string on error.
#[must_use]
#[allow(dead_code)]
pub fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

/// Truncate a string to a maximum length, adding an ellipsis if truncated.
///
/// Uses char boundaries to avoid panicking on multi-byte UTF-8 characters.
#[must_use]
pub fn truncate_with_ellipsis(s: &str, max_len: usize, ellipsis: &str) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let budget = max_len.saturating_sub(ellipsis.len());
    // Find the last char boundary that fits within the byte budget.
    let safe_end = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= budget)
        .last()
        .unwrap_or(0);
    format!("{}{}", &s[..safe_end], ellipsis)
}

/// Percent-encode a string for use in URL query parameters.
///
/// Encodes all characters except unreserved characters (A-Z, a-z, 0-9, `-`, `_`, `.`, `~`).
/// Spaces are encoded as `+`.
#[must_use]
pub fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for ch in input.bytes() {
        match ch {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(ch as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{ch:02X}")),
        }
    }
    encoded
}

/// Estimate the total character count across message content blocks.
#[must_use]
pub fn estimate_message_chars(messages: &[Message]) -> usize {
    let mut total = 0;
    for msg in messages {
        for block in &msg.content {
            match block {
                ContentBlock::Text { text, .. } => total += text.len(),
                ContentBlock::Thinking { thinking } => total += thinking.len(),
                ContentBlock::ToolUse { input, .. } => total += input.to_string().len(),
                ContentBlock::ToolResult { content, .. } => total += content.len(),
                ContentBlock::ServerToolUse { .. }
                | ContentBlock::ToolSearchToolResult { .. }
                | ContentBlock::CodeExecutionToolResult { .. } => {}
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::write_atomic;

    #[test]
    fn write_atomic_creates_and_replaces_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        write_atomic(&path, b"old").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "old");
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }
}
