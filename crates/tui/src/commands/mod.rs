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
            message: Some(format!("错误：{}", msg.into())),
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
            format!("{}  别名：{}", self.description, self.aliases.join(", "))
        }
    }
}

/// All registered commands
pub const COMMANDS: &[CommandInfo] = &[
    // Core commands
    CommandInfo {
        name: "help",
        aliases: &["?"],
        description: "显示帮助信息",
        usage: "/help [command]",
    },
    CommandInfo {
        name: "clear",
        aliases: &[],
        description: "清空对话历史",
        usage: "/clear",
    },
    CommandInfo {
        name: "exit",
        aliases: &["quit", "q"],
        description: "退出应用",
        usage: "/exit",
    },
    CommandInfo {
        name: "model",
        aliases: &[],
        description: "切换或查看当前模型",
        usage: "/model [name]",
    },
    CommandInfo {
        name: "models",
        aliases: &[],
        description: "从 API 获取可用模型列表",
        usage: "/models",
    },
    CommandInfo {
        name: "provider",
        aliases: &[],
        description: "切换或查看当前 LLM 后端（xiaomimimo | nvidia-nim）",
        usage: "/provider [name]",
    },
    CommandInfo {
        name: "queue",
        aliases: &["queued"],
        description: "查看或编辑排队消息",
        usage: "/queue [list|edit <n>|drop <n>|clear]",
    },
    CommandInfo {
        name: "subagents",
        aliases: &["agents"],
        description: "查看子代理状态",
        usage: "/subagents",
    },
    CommandInfo {
        name: "links",
        aliases: &["dashboard", "api"],
        description: "显示 XiaomiMiMo 控制台和文档链接",
        usage: "/links",
    },
    CommandInfo {
        name: "home",
        aliases: &["stats", "overview"],
        description: "显示包含统计和快捷操作的首页",
        usage: "/home",
    },
    CommandInfo {
        name: "note",
        aliases: &[],
        description: "把备注追加到持久备注文件（.xiaomimimo/notes.md）",
        usage: "/note <text>",
    },
    CommandInfo {
        name: "attach",
        aliases: &["image", "media"],
        description: "附加图片/视频媒体；文本文件或目录请用 @path",
        usage: "/attach <path>",
    },
    CommandInfo {
        name: "task",
        aliases: &["tasks"],
        description: "管理后台任务",
        usage: "/task [add <prompt>|list|show <id>|cancel <id>]",
    },
    CommandInfo {
        name: "jobs",
        aliases: &["job"],
        description: "查看和控制后台 Shell 作业",
        usage: "/jobs [list|show <id>|poll <id>|wait <id>|stdin <id> <input>|cancel <id>]",
    },
    CommandInfo {
        name: "mcp",
        aliases: &[],
        description: "打开或管理 MCP 服务器",
        usage: "/mcp [init|add stdio <name> <command> [args...]|add http <name> <url>|enable <name>|disable <name>|remove <name>|validate|reload]",
    },
    // Session commands
    CommandInfo {
        name: "save",
        aliases: &[],
        description: "将会话保存到文件",
        usage: "/save [path]",
    },
    CommandInfo {
        name: "sessions",
        aliases: &["resume"],
        description: "打开会话选择器",
        usage: "/sessions",
    },
    CommandInfo {
        name: "load",
        aliases: &[],
        description: "从文件加载会话",
        usage: "/load [path]",
    },
    CommandInfo {
        name: "compact",
        aliases: &[],
        description: "触发上下文压缩以释放空间（旧方式；v0.6.6 优先使用周期重启）",
        usage: "/compact",
    },
    CommandInfo {
        name: "context",
        aliases: &["ctx"],
        description: "打开会话上下文检查器",
        usage: "/context",
    },
    CommandInfo {
        name: "cycles",
        aliases: &[],
        description: "列出本会话中的检查点重启周期交接",
        usage: "/cycles",
    },
    CommandInfo {
        name: "cycle",
        aliases: &[],
        description: "显示指定周期的延续简报",
        usage: "/cycle <n>",
    },
    CommandInfo {
        name: "recall",
        aliases: &[],
        description: "搜索历史周期归档（基于消息文本的 BM25）",
        usage: "/recall <query>",
    },
    CommandInfo {
        name: "export",
        aliases: &[],
        description: "将对话导出为 Markdown",
        usage: "/export [path]",
    },
    // Config commands
    CommandInfo {
        name: "config",
        aliases: &[],
        description: "打开交互式配置编辑器或更新设置",
        usage: "/config [key [value]|native]",
    },
    CommandInfo {
        name: "lsp",
        aliases: &[],
        description: "显示或更新 LSP 诊断启动设置",
        usage: "/lsp [status|on|off]",
    },
    CommandInfo {
        name: "profile",
        aliases: &[],
        description: "将命名配置档应用到当前会话",
        usage: "/profile <name>",
    },
    CommandInfo {
        name: "yolo",
        aliases: &[],
        description: "启用 YOLO 模式（Shell + 信任 + 自动批准）",
        usage: "/yolo",
    },
    CommandInfo {
        name: "agent",
        aliases: &[],
        description: "切换到 Agent 模式",
        usage: "/agent",
    },
    CommandInfo {
        name: "plan",
        aliases: &[],
        description: "切换到 Plan 模式并先查看建议实现步骤",
        usage: "/plan",
    },
    CommandInfo {
        name: "trust",
        aliases: &[],
        description: "管理工作区信任和路径白名单（`/trust add <path>`、`/trust list`、`/trust on|off`）",
        usage: "/trust [on|off|add <path>|remove <path>|list]",
    },
    CommandInfo {
        name: "logout",
        aliases: &[],
        description: "清除 API key 并返回设置流程",
        usage: "/logout",
    },
    // Debug commands
    CommandInfo {
        name: "tokens",
        aliases: &[],
        description: "显示会话 token 用量",
        usage: "/tokens",
    },
    CommandInfo {
        name: "cache",
        aliases: &[],
        description: "显示最近一轮 prompt 缓存遥测",
        usage: "/cache [status]",
    },
    CommandInfo {
        name: "system",
        aliases: &[],
        description: "显示当前系统提示词",
        usage: "/system",
    },
    CommandInfo {
        name: "undo",
        aliases: &[],
        description: "移除最后一组消息",
        usage: "/undo",
    },
    CommandInfo {
        name: "retry",
        aliases: &[],
        description: "重试上一条请求",
        usage: "/retry",
    },
    CommandInfo {
        name: "init",
        aliases: &[],
        description: "为项目生成 AGENTS.md",
        usage: "/init",
    },
    CommandInfo {
        name: "settings",
        aliases: &[],
        description: "显示持久化设置",
        usage: "/settings",
    },
    CommandInfo {
        name: "statusline",
        aliases: &["status"],
        description: "配置底部状态栏显示项",
        usage: "/statusline",
    },
    // Skills commands
    CommandInfo {
        name: "skills",
        aliases: &[],
        description: "列出本地技能（或用 --remote 浏览精选仓库）",
        usage: "/skills [--remote]",
    },
    CommandInfo {
        name: "skill",
        aliases: &[],
        description: "激活技能，或安装/更新/卸载/信任社区技能",
        usage: "/skill <name|install <spec>|update <name>|uninstall <name>|trust <name>>",
    },
    CommandInfo {
        name: "review",
        aliases: &[],
        description: "对文件、diff 或 PR 执行结构化代码审查",
        usage: "/review <target>",
    },
    CommandInfo {
        name: "restore",
        aliases: &[],
        description: "将工作区回滚到之前的回合前/后快照；不带参数时列出最近快照",
        usage: "/restore [N]",
    },
    // RLM command
    CommandInfo {
        name: "rlm",
        aliases: &["recursive"],
        description: "递归语言模型（RLM）回合：把 prompt 存入 Python REPL，让模型编写代码处理，并可通过 `llm_query()` / `sub_rlm()` 调用子模型。",
        usage: "/rlm <prompt>",
    },
    // Debug/cost command
    CommandInfo {
        name: "cost",
        aliases: &[],
        description: "显示会话 token 用量",
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
            "/set 指令已停用。请用 /config 编辑设置，用 /settings 查看当前值。",
        ),
        "normal" => config::normal_mode(app),
        "xiaomimimo" => {
            CommandResult::error("/xiaomimimo 指令已改名。请用 /links（别名：/dashboard、/api）。")
        }

        _ => {
            let suggestions = suggest_command_names(command, 3);
            if suggestions.is_empty() {
                CommandResult::error(format!("未知指令：/{command}。输入 /help 查看可用指令。"))
            } else {
                let list = suggestions
                    .into_iter()
                    .map(|name| format!("/{name}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                CommandResult::error(format!(
                    "未知指令：/{command}。你是不是想输入：{list}？输入 /help 查看可用指令。"
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
                "用法：/rlm <prompt>\n\n\
                 使用递归语言模型（RLM）处理 prompt。\n\
                 prompt 会存入 REPL，由模型编写代码进行递归拆解和处理。"
                    .to_string(),
            );
        }
    };

    // Sanity-check: RLM is most useful for longer prompts.
    if prompt.len() < 50 {
        return CommandResult::message(
            "提示：RLM 更适合处理较长 prompt（>100 字符）。\
             短问题直接输入消息即可。"
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
            "正在启动 RLM 回合：prompt {} 个字符，模型 {}（子模型={}，深度={}）...",
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
        assert!(lsp.contains("LSP 诊断"));

        app.last_prompt_tokens = Some(10);
        app.last_prompt_cache_hit_tokens = Some(7);
        app.last_prompt_cache_miss_tokens = Some(3);
        let cache = execute("/cache", &mut app).message.expect("cache message");
        assert!(cache.contains("Prompt 缓存"));

        let profile = execute("/profile definitely-missing-profile", &mut app)
            .message
            .expect("profile error message");
        assert!(profile.contains("错误："));
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
        assert!(set_msg.contains("/set 指令已停用"));
        assert!(set_msg.contains("/config"));
        assert!(set_msg.contains("/settings"));
        assert!(set_result.action.is_none());

        let xiaomimimo_result = execute("/xiaomimimo", &mut app);
        let xiaomimimo_msg = xiaomimimo_result
            .message
            .expect("legacy command should return an error message");
        assert!(xiaomimimo_msg.contains("/xiaomimimo 指令已改名"));
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
        assert!(msg.contains("未知指令：/modle"));
        assert!(msg.contains("你是不是想输入："));
        assert!(msg.contains("/model"));
    }

    #[test]
    fn unknown_command_without_close_match_keeps_help_guidance() {
        let mut app = create_test_app();
        let result = execute("/zzzzzz", &mut app);
        let msg = result
            .message
            .expect("unknown command should return an error message");
        assert!(msg.contains("未知指令：/zzzzzz"));
        assert!(msg.contains("输入 /help 查看可用指令。"));
    }
}
