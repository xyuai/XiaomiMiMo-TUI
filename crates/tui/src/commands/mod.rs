//! Slash command registry and dispatch system
//!
//! This module provides a modular command system inspired by Codex-rs.
//! Commands are organized by category and dispatched through a central registry.

mod attachment;
mod config;
mod core;
mod cycle;
mod debug;
mod init;
mod jobs;
mod mcp;
mod note;
mod provider;
mod queue;
mod restore;
mod review;
mod session;
mod skills;
mod task;

use crate::tui::app::{App, AppAction};

/// Result of executing a command
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Optional message to display to the user
    pub message: Option<String>,
    /// Optional action for the app to take
    pub action: Option<AppAction>,
}

impl CommandResult {
    /// Create an empty result (command succeeded with no output)
    pub fn ok() -> Self {
        Self {
            message: None,
            action: None,
        }
    }

    /// Create a result with just a message
    pub fn message(msg: impl Into<String>) -> Self {
        Self {
            message: Some(msg.into()),
            action: None,
        }
    }

    /// Create a result with an action
    pub fn action(action: AppAction) -> Self {
        Self {
            message: None,
            action: Some(action),
        }
    }

    /// Create a result with both message and action
    #[allow(dead_code)]
    pub fn with_message_and_action(msg: impl Into<String>, action: AppAction) -> Self {
        Self {
            message: Some(msg.into()),
            action: Some(action),
        }
    }

    /// Create an error message result
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            message: Some(format!("Error: {}", msg.into())),
            action: None,
        }
    }
}

/// Command metadata for help and autocomplete
#[derive(Debug, Clone, Copy)]
pub struct CommandInfo {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub usage: &'static str,
}

impl CommandInfo {
    pub fn requires_argument(&self) -> bool {
        self.usage.contains('<')
    }

    pub fn palette_command(&self) -> String {
        if self.requires_argument() {
            format!("/{} ", self.name)
        } else {
            format!("/{}", self.name)
        }
    }

    pub fn palette_description(&self) -> String {
        if self.aliases.is_empty() {
            self.description.to_string()
        } else {
            format!("{}  aliases: {}", self.description, self.aliases.join(", "))
        }
    }
}

/// All registered commands
pub const COMMANDS: &[CommandInfo] = &[
    // Core commands
    CommandInfo {
        name: "help",
        aliases: &["?"],
        description: "Show help information",
        usage: "/help [command]",
    },
    CommandInfo {
        name: "clear",
        aliases: &[],
        description: "Clear conversation history",
        usage: "/clear",
    },
    CommandInfo {
        name: "exit",
        aliases: &["quit", "q"],
        description: "Exit the application",
        usage: "/exit",
    },
    CommandInfo {
        name: "model",
        aliases: &[],
        description: "Switch or view current model",
        usage: "/model [name]",
    },
    CommandInfo {
        name: "models",
        aliases: &[],
        description: "List available models from API",
        usage: "/models",
    },
    CommandInfo {
        name: "provider",
        aliases: &[],
        description: "Switch or view the active LLM backend (xiaomimimo | nvidia-nim)",
        usage: "/provider [name]",
    },
    CommandInfo {
        name: "queue",
        aliases: &["queued"],
        description: "View or edit queued messages",
        usage: "/queue [list|edit <n>|drop <n>|clear]",
    },
    CommandInfo {
        name: "subagents",
        aliases: &["agents"],
        description: "List sub-agent status",
        usage: "/subagents",
    },
    CommandInfo {
        name: "links",
        aliases: &["dashboard", "api"],
        description: "Show XiaomiMiMo dashboard and docs links",
        usage: "/links",
    },
    CommandInfo {
        name: "home",
        aliases: &["stats", "overview"],
        description: "Show home dashboard with stats and quick actions",
        usage: "/home",
    },
    CommandInfo {
        name: "note",
        aliases: &[],
        description: "Append note to persistent notes file (.xiaomimimo/notes.md)",
        usage: "/note <text>",
    },
    CommandInfo {
        name: "attach",
        aliases: &["image", "media"],
        description: "Attach image/video media; use @path for text files or directories",
        usage: "/attach <path>",
    },
    CommandInfo {
        name: "task",
        aliases: &["tasks"],
        description: "Manage background tasks",
        usage: "/task [add <prompt>|list|show <id>|cancel <id>]",
    },
    CommandInfo {
        name: "jobs",
        aliases: &["job"],
        description: "Inspect and control background shell jobs",
        usage: "/jobs [list|show <id>|poll <id>|wait <id>|stdin <id> <input>|cancel <id>]",
    },
    CommandInfo {
        name: "mcp",
        aliases: &[],
        description: "Open or manage MCP servers",
        usage: "/mcp [init|add stdio <name> <command> [args...]|add http <name> <url>|enable <name>|disable <name>|remove <name>|validate|reload]",
    },
    // Session commands
    CommandInfo {
        name: "save",
        aliases: &[],
        description: "Save session to file",
        usage: "/save [path]",
    },
    CommandInfo {
        name: "sessions",
        aliases: &["resume"],
        description: "Open session picker",
        usage: "/sessions",
    },
    CommandInfo {
        name: "load",
        aliases: &[],
        description: "Load session from file",
        usage: "/load [path]",
    },
    CommandInfo {
        name: "compact",
        aliases: &[],
        description: "Trigger context compaction to free up space (legacy; v0.6.6 prefers cycle restart)",
        usage: "/compact",
    },
    CommandInfo {
        name: "context",
        aliases: &["ctx"],
        description: "Open compact session context inspector",
        usage: "/context",
    },
    CommandInfo {
        name: "cycles",
        aliases: &[],
        description: "List checkpoint-restart cycle handoffs in this session",
        usage: "/cycles",
    },
    CommandInfo {
        name: "cycle",
        aliases: &[],
        description: "Show the carry-forward briefing for a specific cycle",
        usage: "/cycle <n>",
    },
    CommandInfo {
        name: "recall",
        aliases: &[],
        description: "Search prior cycle archives (BM25 over message text)",
        usage: "/recall <query>",
    },
    CommandInfo {
        name: "export",
        aliases: &[],
        description: "Export conversation to markdown",
        usage: "/export [path]",
    },
    // Config commands
    CommandInfo {
        name: "config",
        aliases: &[],
        description: "Open interactive configuration editor or update settings",
        usage: "/config [key [value]|native]",
    },
    CommandInfo {
        name: "lsp",
        aliases: &[],
        description: "Show or update LSP diagnostics startup setting",
        usage: "/lsp [status|on|off]",
    },
    CommandInfo {
        name: "profile",
        aliases: &[],
        description: "Apply a named config profile to the current session",
        usage: "/profile <name>",
    },
    CommandInfo {
        name: "yolo",
        aliases: &[],
        description: "Enable YOLO mode (shell + trust + auto-approve)",
        usage: "/yolo",
    },
    CommandInfo {
        name: "agent",
        aliases: &[],
        description: "Switch to agent mode",
        usage: "/agent",
    },
    CommandInfo {
        name: "plan",
        aliases: &[],
        description: "Switch to plan mode and review suggested implementation steps",
        usage: "/plan",
    },
    CommandInfo {
        name: "trust",
        aliases: &[],
        description: "Manage workspace trust and per-path allowlist (`/trust add <path>`, `/trust list`, `/trust on|off`)",
        usage: "/trust [on|off|add <path>|remove <path>|list]",
    },
    CommandInfo {
        name: "logout",
        aliases: &[],
        description: "Clear API key and return to setup",
        usage: "/logout",
    },
    // Debug commands
    CommandInfo {
        name: "tokens",
        aliases: &[],
        description: "Show token usage for session",
        usage: "/tokens",
    },
    CommandInfo {
        name: "cache",
        aliases: &[],
        description: "Show prompt-cache telemetry for the latest turn",
        usage: "/cache [status]",
    },
    CommandInfo {
        name: "system",
        aliases: &[],
        description: "Show current system prompt",
        usage: "/system",
    },
    CommandInfo {
        name: "undo",
        aliases: &[],
        description: "Remove last message pair",
        usage: "/undo",
    },
    CommandInfo {
        name: "retry",
        aliases: &[],
        description: "Retry the last request",
        usage: "/retry",
    },
    CommandInfo {
        name: "init",
        aliases: &[],
        description: "Generate AGENTS.md for project",
        usage: "/init",
    },
    CommandInfo {
        name: "settings",
        aliases: &[],
        description: "Show persistent settings",
        usage: "/settings",
    },
    CommandInfo {
        name: "statusline",
        aliases: &["status"],
        description: "Configure which items appear in the footer",
        usage: "/statusline",
    },
    // Skills commands
    CommandInfo {
        name: "skills",
        aliases: &[],
        description: "List local skills (or --remote to browse the curated registry)",
        usage: "/skills [--remote]",
    },
    CommandInfo {
        name: "skill",
        aliases: &[],
        description: "Activate a skill, or install/update/uninstall/trust a community skill",
        usage: "/skill <name|install <spec>|update <name>|uninstall <name>|trust <name>>",
    },
    CommandInfo {
        name: "review",
        aliases: &[],
        description: "Run a structured code review on a file, diff, or PR",
        usage: "/review <target>",
    },
    CommandInfo {
        name: "restore",
        aliases: &[],
        description: "Roll back the workspace to a prior pre/post-turn snapshot. With no arg, lists recent snapshots.",
        usage: "/restore [N]",
    },
    // RLM command
    CommandInfo {
        name: "rlm",
        aliases: &["recursive"],
        description: "Recursive Language Model (RLM) turn — store the prompt in a Python REPL and let the model write code to process it, with `llm_query()` / `sub_rlm()` for sub-LLM calls.",
        usage: "/rlm <prompt>",
    },
    // Debug/cost command
    CommandInfo {
        name: "cost",
        aliases: &[],
        description: "Show session token usage",
        usage: "/cost",
    },
];

/// Conservative slash-command smoke fixture used by tests and manual audits.
///
/// The command avoids destructive side effects and external network access while
/// still exercising the same registry entry and parser branch as the public
/// slash command.
#[cfg(test)]
fn smoke_test_invocation(command: &CommandInfo) -> String {
    match command.name {
        "help" => "/help clear",
        "model" => "/model mimo-v2.5-tts",
        "provider" => "/provider xiaomimimo",
        "queue" => "/queue list",
        "note" => "/note slash command smoke test",
        "attach" => "/attach missing-smoke-test-image.png",
        "task" => "/task list",
        "jobs" => "/jobs list",
        "mcp" => "/mcp status",
        "lsp" => "/lsp status",
        "profile" => "/profile missing-smoke-profile",
        "save" => "/save slash_command_smoke_session.json",
        "load" => "/load missing-smoke-test-session.json",
        "cycle" => "/cycle 1",
        "recall" => "/recall",
        "trust" => "/trust status",
        "skill" => "/skill missing-smoke-test-skill",
        "review" => "/review smoke-target",
        "restore" => "/restore",
        "rlm" => {
            "/rlm This is a deliberately long smoke-test prompt that exercises the RLM slash command parser without starting from the usage-error branch."
        }
        other => {
            if command.requires_argument() {
                panic!("No smoke-test invocation registered for argument-taking command /{other}");
            }
            return command.palette_command();
        }
    }
    .to_string()
}

/// Execute a slash command
pub fn execute(cmd: &str, app: &mut App) -> CommandResult {
    let parts: Vec<&str> = cmd.trim().splitn(2, ' ').collect();
    let command = parts[0].to_lowercase();
    let command = command.strip_prefix('/').unwrap_or(&command);
    let arg = parts.get(1).map(|s| s.trim());

    // Match command or alias
    match command {
        // Core commands
        "help" | "?" => core::help(app, arg),
        "clear" => core::clear(app),
        "exit" | "quit" | "q" => core::exit(),
        "model" => core::model(app, arg),
        "models" => core::models(app),
        "provider" => provider::provider(app, arg),
        "queue" | "queued" => queue::queue(app, arg),
        "subagents" | "agents" => core::subagents(app),
        "links" | "dashboard" | "api" => core::xiaomimimo_links(),
        "home" | "stats" | "overview" => core::home_dashboard(app),
        "note" => note::note(app, arg),
        "attach" | "image" | "media" => attachment::attach(app, arg),
        "task" | "tasks" => task::task(app, arg),
        "jobs" | "job" => jobs::jobs(app, arg),
        "mcp" => mcp::mcp(app, arg),

        // Session commands
        "save" => session::save(app, arg),
        "sessions" | "resume" => session::sessions(app),
        "load" => session::load(app, arg),
        "compact" => session::compact(app),
        "cycles" => cycle::list_cycles(app),
        "cycle" => cycle::show_cycle(app, arg),
        "recall" => cycle::recall_archive(app, arg),
        "export" => session::export(app, arg),

        // Config commands
        "config" => config::config_command(app, arg),
        "lsp" => config::lsp_command(app, arg),
        "profile" => config::profile(app, arg),
        "settings" => config::show_settings(app),
        "statusline" | "status" => config::status_line(app),
        "yolo" => config::yolo(app),
        "agent" => config::agent_mode(app),
        "plan" => config::plan_mode(app),
        "trust" => config::trust(app, arg),
        "logout" => config::logout(app),

        // Debug commands
        "tokens" => debug::tokens(app),
        "cost" => debug::cost(app),
        "cache" => debug::cache(app, arg),
        "system" => debug::system_prompt(app),
        "context" | "ctx" => debug::context(app),
        "undo" => debug::undo(app),
        "retry" => debug::retry(app),

        // Project commands
        "init" => init::init(app),

        // Skills commands
        "skills" => skills::list_skills(app, arg),
        "skill" => skills::run_skill(app, arg),
        "review" => review::review(app, arg),
        "restore" => restore::restore(app, arg),

        // RLM command
        "rlm" | "recursive" => rlm(app, arg),

        // Legacy command migrations (kept out of registry/autocomplete intentionally).
        "set" => CommandResult::error(
            "The /set command was retired. Use /config to edit settings and /settings to inspect current values.",
        ),
        "normal" => config::normal_mode(app),
        "xiaomimimo" => CommandResult::error(
            "The /xiaomimimo command was renamed. Use /links (aliases: /dashboard, /api).",
        ),

        _ => {
            let suggestions = suggest_command_names(command, 3);
            if suggestions.is_empty() {
                CommandResult::error(format!(
                    "Unknown command: /{command}. Type /help for available commands."
                ))
            } else {
                let list = suggestions
                    .into_iter()
                    .map(|name| format!("/{name}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                CommandResult::error(format!(
                    "Unknown command: /{command}. Did you mean: {list}? Type /help for available commands."
                ))
            }
        }
    }
}

/// Update a configuration value programmatically (used by interactive UI views).
pub fn set_config_value(app: &mut App, key: &str, value: &str, persist: bool) -> CommandResult {
    config::set_config_value(app, key, value, persist)
}

/// Persist the user's chosen footer items to `~/.xiaomimimo/config.toml` under
/// `tui.status_items`. See [`config::persist_status_items`] for details.
pub fn persist_status_items(
    items: &[crate::config::StatusItem],
) -> anyhow::Result<std::path::PathBuf> {
    config::persist_status_items(items)
}

/// Execute a Recursive Language Model (RLM) turn — Algorithm 1 from
/// Zhang et al. (arXiv:2512.24601).
///
/// The user's prompt text is passed as the argument. It will be stored
/// in the REPL as the `PROMPT` variable. The root LLM will only see
/// metadata about the REPL state, never the prompt text directly.
pub fn rlm(app: &mut App, arg: Option<&str>) -> CommandResult {
    let prompt = match arg {
        Some(p) if !p.trim().is_empty() => p.trim().to_string(),
        _ => {
            return CommandResult::error(
                "Usage: /rlm <prompt>\n\n\
                 Process a prompt using a Recursive Language Model (RLM).\n\
                 The prompt is stored in a REPL and the model writes code\n\
                 to decompose and process it recursively."
                    .to_string(),
            );
        }
    };

    // Sanity-check: RLM is most useful for longer prompts.
    if prompt.len() < 50 {
        return CommandResult::message(
            "Tip: RLM is designed for processing LONG prompts (>100 chars). \
             For short queries, just type the message directly."
                .to_string(),
        );
    }

    let model = app.model.clone();
    let child_model = "mimo-v2-flash".to_string();
    // Paper experiments use depth=1 (one level of `sub_rlm`); we default to
    // depth=2 so the model can recurse twice if it chooses to.
    let max_depth: u32 = 2;

    CommandResult::with_message_and_action(
        format!(
            "Starting RLM turn for {} chars of prompt using {} (child={}, depth={})...",
            prompt.len(),
            model,
            child_model,
            max_depth,
        ),
        AppAction::Rlm {
            prompt,
            model,
            child_model,
            max_depth,
        },
    )
}

/// Get command info by name or alias
pub fn get_command_info(name: &str) -> Option<&'static CommandInfo> {
    let name = name.strip_prefix('/').unwrap_or(name);
    COMMANDS
        .iter()
        .find(|cmd| cmd.name == name || cmd.aliases.contains(&name))
}

/// Get all commands matching a prefix (for autocomplete)
#[allow(dead_code)]
pub fn commands_matching(prefix: &str) -> Vec<&'static CommandInfo> {
    let prefix = prefix.strip_prefix('/').unwrap_or(prefix).to_lowercase();
    COMMANDS
        .iter()
        .filter(|cmd| {
            cmd.name.starts_with(&prefix) || cmd.aliases.iter().any(|a| a.starts_with(&prefix))
        })
        .collect()
}

fn edit_distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    if a.is_empty() {
        return b.chars().count();
    }
    if b.is_empty() {
        return a.chars().count();
    }

    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b_chars.len()).collect();
    let mut curr = vec![0usize; b_chars.len() + 1];

    for (i, a_ch) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, b_ch) in b_chars.iter().enumerate() {
            let cost = if a_ch == *b_ch { 0 } else { 1 };
            let delete = prev[j + 1] + 1;
            let insert = curr[j] + 1;
            let substitute = prev[j] + cost;
            curr[j + 1] = delete.min(insert).min(substitute);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_chars.len()]
}

fn suggest_command_names(input: &str, limit: usize) -> Vec<String> {
    let query = input.trim().to_ascii_lowercase();
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut scored: Vec<(u8, usize, String)> = Vec::new();
    for command in COMMANDS {
        let mut best: Option<(u8, usize)> = None;
        for candidate in std::iter::once(command.name).chain(command.aliases.iter().copied()) {
            let candidate = candidate.to_ascii_lowercase();
            let prefix_match = candidate.starts_with(&query) || query.starts_with(&candidate);
            let contains_match = candidate.contains(&query) || query.contains(&candidate);
            let distance = edit_distance(&candidate, &query);
            let close_typo = distance <= 2;
            if !(prefix_match || contains_match || close_typo) {
                continue;
            }

            let rank = if prefix_match {
                0
            } else if contains_match {
                1
            } else {
                2
            };

            match best {
                Some((best_rank, best_distance))
                    if rank > best_rank || (rank == best_rank && distance >= best_distance) => {}
                _ => best = Some((rank, distance)),
            }
        }

        if let Some((rank, distance)) = best {
            scored.push((rank, distance, command.name.to_string()));
        }
    }

    scored.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    scored
        .into_iter()
        .take(limit)
        .map(|(_, _, name)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::app::{App, AppAction, TuiOptions};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_app() -> App {
        App::new(test_options(PathBuf::from(".")), &Config::default())
    }

    fn test_options(workspace: PathBuf) -> TuiOptions {
        TuiOptions {
            model: "mimo-v2.5-pro".to_string(),
            skills_dir: workspace.join("skills"),
            memory_path: workspace.join("memory.md"),
            notes_path: workspace.join("notes.txt"),
            mcp_config_path: workspace.join("mcp.json"),
            workspace,
            workspace_explicit: true,
            allow_shell: false,
            use_alt_screen: true,
            use_mouse_capture: false,
            use_bracketed_paste: true,
            max_subagents: 1,
            use_memory: false,
            start_in_agent_mode: false,
            skip_onboarding: true,
            yolo: false,
            resume_session_id: None,
        }
    }

    fn create_test_app_in(tmpdir: &TempDir) -> App {
        App::new(
            test_options(tmpdir.path().to_path_buf()),
            &Config::default(),
        )
    }

    #[test]
    fn command_registry_has_unique_names_and_aliases() {
        let mut names = std::collections::BTreeSet::new();
        let mut aliases = std::collections::BTreeMap::new();
        for command in COMMANDS {
            assert!(
                names.insert(command.name),
                "duplicate command /{}",
                command.name
            );
            for alias in command.aliases {
                assert_ne!(
                    *alias, command.name,
                    "command /{} aliases itself",
                    command.name
                );
                assert!(
                    get_command_info(alias).is_some(),
                    "alias /{} for /{} is not resolvable",
                    alias,
                    command.name
                );
                if let Some(previous) = aliases.insert(*alias, command.name) {
                    panic!(
                        "alias /{} is registered for both /{} and /{}",
                        alias, previous, command.name
                    );
                }
            }
        }
    }

    #[test]
    fn every_registered_command_has_a_smoke_test_invocation() {
        for command in COMMANDS {
            let invocation = smoke_test_invocation(command);
            assert!(
                invocation.starts_with('/'),
                "smoke invocation for /{} must start with slash: {}",
                command.name,
                invocation
            );
            let invoked_name = invocation
                .trim_start_matches('/')
                .split_whitespace()
                .next()
                .unwrap_or("");
            let resolved = get_command_info(invoked_name)
                .unwrap_or_else(|| panic!("smoke invocation is not registered: {invocation}"));
            assert_eq!(
                resolved.name, command.name,
                "smoke invocation {invocation} resolved to /{} instead of /{}",
                resolved.name, command.name
            );
        }
    }

    #[test]
    fn all_safe_smoke_commands_dispatch_without_panic() {
        let tmpdir = TempDir::new().expect("tempdir");
        let mut app = create_test_app_in(&tmpdir);

        for command in COMMANDS {
            let invocation = smoke_test_invocation(command);
            let result = execute(&invocation, &mut app);
            assert!(
                result.message.is_some()
                    || result.action.is_some()
                    || matches!(command.name, "help" | "sessions"),
                "smoke invocation {invocation} for /{} returned an empty result",
                command.name
            );
        }
    }

    #[test]
    fn command_registry_contains_config_and_links_but_not_set_or_xiaomimimo() {
        assert!(COMMANDS.iter().any(|cmd| cmd.name == "config"));
        assert!(COMMANDS.iter().any(|cmd| cmd.name == "links"));
        assert!(!COMMANDS.iter().any(|cmd| cmd.name == "set"));
        assert!(!COMMANDS.iter().any(|cmd| cmd.name == "xiaomimimo"));
    }

    #[test]
    fn links_command_has_dashboard_and_api_aliases() {
        let links = COMMANDS
            .iter()
            .find(|cmd| cmd.name == "links")
            .expect("links command should exist");
        assert_eq!(links.aliases, &["dashboard", "api"]);
    }

    #[test]
    fn execute_config_opens_config_view_action() {
        let mut app = create_test_app();
        let result = execute("/config", &mut app);
        assert!(result.message.is_none());
        assert!(matches!(result.action, Some(AppAction::OpenConfigView)));
    }

    #[test]
    fn execute_config_can_show_and_set_runtime_values() {
        let mut app = create_test_app();
        let result = execute("/config model", &mut app);
        let msg = result.message.expect("config key should return value");
        assert!(msg.contains("model ="));

        let result = execute("/config approval never", &mut app);
        let msg = result.message.expect("config set should return value");
        assert!(msg.contains("approval_mode"));
        assert!(msg.contains("never"));
    }

    #[test]
    fn execute_lsp_cache_and_profile_dispatch() {
        let tmpdir = TempDir::new().expect("tempdir");
        let mut app = create_test_app_in(&tmpdir);
        let lsp = execute("/lsp status", &mut app)
            .message
            .expect("lsp status message");
        assert!(lsp.contains("LSP diagnostics"));

        app.last_prompt_tokens = Some(10);
        app.last_prompt_cache_hit_tokens = Some(7);
        app.last_prompt_cache_miss_tokens = Some(3);
        let cache = execute("/cache", &mut app).message.expect("cache message");
        assert!(cache.contains("Prompt Cache"));

        let profile = execute("/profile definitely-missing-profile", &mut app)
            .message
            .expect("profile error message");
        assert!(profile.contains("Error:"));
    }

    #[test]
    fn execute_links_and_aliases_return_links_message() {
        let mut app = create_test_app();
        for cmd in ["/links", "/dashboard", "/api"] {
            let result = execute(cmd, &mut app);
            let msg = result.message.expect("links commands should return text");
            assert!(msg.contains("https://platform.xiaomimimo.com"));
            assert!(result.action.is_none());
        }
    }

    #[test]
    fn removed_set_and_xiaomimimo_commands_show_migration_hints() {
        let mut app = create_test_app();
        let set_result = execute("/set model mimo-v2.5-pro", &mut app);
        let set_msg = set_result
            .message
            .expect("legacy command should return an error message");
        assert!(set_msg.contains("The /set command was retired"));
        assert!(set_msg.contains("/config"));
        assert!(set_msg.contains("/settings"));
        assert!(set_result.action.is_none());

        let xiaomimimo_result = execute("/xiaomimimo", &mut app);
        let xiaomimimo_msg = xiaomimimo_result
            .message
            .expect("legacy command should return an error message");
        assert!(xiaomimimo_msg.contains("The /xiaomimimo command was renamed"));
        assert!(xiaomimimo_msg.contains("/links"));
        assert!(xiaomimimo_msg.contains("/dashboard"));
        assert!(xiaomimimo_msg.contains("/api"));
        assert!(xiaomimimo_result.action.is_none());
    }

    #[test]
    fn unknown_command_suggests_nearest_match() {
        let mut app = create_test_app();
        let result = execute("/modle", &mut app);
        let msg = result
            .message
            .expect("unknown command should return an error message");
        assert!(msg.contains("Unknown command: /modle"));
        assert!(msg.contains("Did you mean:"));
        assert!(msg.contains("/model"));
    }

    #[test]
    fn unknown_command_without_close_match_keeps_help_guidance() {
        let mut app = create_test_app();
        let result = execute("/zzzzzz", &mut app);
        let msg = result
            .message
            .expect("unknown command should return an error message");
        assert!(msg.contains("Unknown command: /zzzzzz"));
        assert!(msg.contains("Type /help for available commands."));
    }
}
