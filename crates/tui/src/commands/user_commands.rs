//! User-defined slash commands from workspace and global command directories.
//!
//! Drop Markdown files into `.xiaomimimo/commands/<name>.md` in the workspace
//! or `~/.xiaomimimo/commands/<name>.md` globally. The filename stem becomes a
//! slash command. Workspace commands take precedence over global commands and
//! built-ins.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::tui::app::{App, AppAction};

use super::CommandResult;

const COMMANDS_DIR_NAME: &str = "commands";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCommand {
    pub name: String,
    pub content: String,
    pub path: PathBuf,
}

pub fn workspace_commands_dir(workspace: &Path) -> PathBuf {
    workspace.join(".xiaomimimo").join(COMMANDS_DIR_NAME)
}

pub fn global_commands_dir() -> PathBuf {
    dirs::home_dir().map_or_else(
        || PathBuf::from("/tmp/xiaomimimo").join(COMMANDS_DIR_NAME),
        |home| home.join(".xiaomimimo").join(COMMANDS_DIR_NAME),
    )
}

fn command_dirs(app: &App) -> Vec<PathBuf> {
    let mut dirs = vec![
        workspace_commands_dir(&app.workspace),
        global_commands_dir(),
    ];
    let mut seen = HashSet::new();
    dirs.retain(|dir| seen.insert(normalize_for_dedup(dir)));
    dirs
}

pub fn load_user_commands(app: &App) -> Vec<UserCommand> {
    load_user_commands_from_dirs(command_dirs(app))
}

fn load_user_commands_from_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<UserCommand> {
    let mut commands = Vec::new();
    let mut seen = HashSet::new();

    for dir in dirs {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        let mut local = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let name = stem.trim().to_ascii_lowercase();
            if name.is_empty() || !is_valid_command_name(&name) || seen.contains(&name) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            local.push(UserCommand {
                name,
                content,
                path,
            });
        }

        local.sort_by(|a, b| a.name.cmp(&b.name));
        for command in local {
            if seen.insert(command.name.clone()) {
                commands.push(command);
            }
        }
    }

    commands
}

pub fn try_dispatch_user_command(app: &mut App, input: &str) -> Option<CommandResult> {
    let parts: Vec<&str> = input.trim().splitn(2, char::is_whitespace).collect();
    let command = parts[0].strip_prefix('/').unwrap_or(parts[0]);
    let command = command.to_ascii_lowercase();
    let args = parts.get(1).copied().unwrap_or("").trim();

    load_user_commands(app)
        .into_iter()
        .find(|entry| entry.name == command)
        .map(|entry| {
            let message = apply_template(&entry.content, args);
            CommandResult::action(AppAction::SendMessage(message))
        })
}

pub fn user_commands_matching(app: &App, prefix: &str) -> Vec<String> {
    let prefix = prefix
        .strip_prefix('/')
        .unwrap_or(prefix)
        .to_ascii_lowercase();
    load_user_commands(app)
        .into_iter()
        .filter(|entry| entry.name.starts_with(&prefix))
        .map(|entry| format!("/{}", entry.name))
        .collect()
}

fn apply_template(template: &str, args: &str) -> String {
    let positional: Vec<&str> = args.split_whitespace().collect();
    let mut result = template.replace("$ARGUMENTS", args);
    for (index, arg) in positional.iter().enumerate() {
        result = result.replace(&format!("${}", index + 1), arg);
    }
    result
}

fn is_valid_command_name(name: &str) -> bool {
    name.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
}

fn normalize_for_dedup(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_command(root: &Path, name: &str, body: &str) {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join(format!("{name}.md")), body).unwrap();
    }

    #[test]
    fn workspace_commands_override_global_commands() {
        let tmp = TempDir::new().unwrap();
        let workspace = tmp.path().join("workspace");
        let global = tmp.path().join("global");
        write_command(
            &workspace.join(".xiaomimimo").join("commands"),
            "ship",
            "workspace $1",
        );
        write_command(
            &global.join(".xiaomimimo").join("commands"),
            "ship",
            "global $1",
        );

        let commands = load_user_commands_from_dirs(vec![
            workspace_commands_dir(&workspace),
            global.join(".xiaomimimo").join("commands"),
        ]);

        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "ship");
        assert_eq!(apply_template(&commands[0].content, "now"), "workspace now");
    }

    #[test]
    fn template_expands_arguments() {
        let rendered = apply_template("all=$ARGUMENTS first=$1 second=$2", "alpha beta gamma");
        assert_eq!(rendered, "all=alpha beta gamma first=alpha second=beta");
    }

    #[test]
    fn matching_returns_slash_prefixed_names() {
        let tmp = TempDir::new().unwrap();
        let commands_dir = tmp.path().join(".xiaomimimo").join("commands");
        write_command(&commands_dir, "deploy", "Deploy");
        write_command(&commands_dir, "docs", "Docs");

        let commands = load_user_commands_from_dirs(vec![commands_dir])
            .into_iter()
            .filter(|entry| entry.name.starts_with('d'))
            .map(|entry| format!("/{}", entry.name))
            .collect::<Vec<_>>();

        assert_eq!(commands, vec!["/deploy", "/docs"]);
    }
}
