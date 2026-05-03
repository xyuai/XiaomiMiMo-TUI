//! `/restore` slash command — roll back the workspace to a prior snapshot.
//!
//! `/restore` (no arg) lists the most recent snapshots so the user can
//! see what's available. `/restore <N>` restores the *N*th-most-recent
//! snapshot, where `N=1` is the newest. In non-YOLO mode we refuse to
//! mutate files unless the user has explicitly trusted the workspace
//! (`/trust on` or YOLO) — the user can always view the list, just not
//! one-shot revert without a safety net.

use super::CommandResult;
use crate::snapshot::SnapshotRepo;
use crate::tui::app::App;

const LIST_LIMIT: usize = 10;

/// Entry point for `/restore [N]`.
pub fn restore(app: &mut App, arg: Option<&str>) -> CommandResult {
    let workspace = app.workspace.clone();
    let repo = match SnapshotRepo::open_or_init(&workspace) {
        Ok(r) => r,
        Err(e) => {
            return CommandResult::error(
                format!("快照仓库不可用（{}）：{e}", workspace.display(),),
            );
        }
    };

    let snapshots = match repo.list(LIST_LIMIT) {
        Ok(s) => s,
        Err(e) => return CommandResult::error(format!("列出快照失败：{e}")),
    };

    if snapshots.is_empty() {
        return CommandResult::message("还没有快照。发送一条消息后会创建第一个回合前快照。");
    }

    let Some(arg) = arg.map(str::trim).filter(|s| !s.is_empty()) else {
        return CommandResult::message(format_listing(&snapshots));
    };

    let n: usize = match arg.parse() {
        Ok(n) if n >= 1 => n,
        _ => {
            return CommandResult::error(format!(
                "用法：/restore <N>  （N 从 1 开始；收到 '{arg}'）",
            ));
        }
    };

    if n > snapshots.len() {
        return CommandResult::error(format!(
            "当前只有 {} 个快照；请求的是 #{n}。",
            snapshots.len(),
        ));
    }

    // Non-YOLO sessions get a confirmation gate. We don't have a true
    // modal-confirmation path inside slash commands today, so the gate
    // is "require trust mode" — `/trust on` or YOLO. Users in plain
    // Agent mode get a clear message explaining how to proceed.
    if !(app.yolo || app.trust_mode) {
        return CommandResult::message(format!(
            "当前不在信任模式，拒绝恢复快照 #{n}（'{}'）。\n\
             请先运行 `/trust on` 或 `/yolo`，然后重新运行 `/restore {n}`。",
            snapshots[n - 1].label,
        ));
    }

    let target = &snapshots[n - 1];
    if let Err(e) = repo.restore(&target.id) {
        return CommandResult::error(format!("恢复失败：{e}"));
    }

    CommandResult::message(format!(
        "已恢复快照 #{n}（'{}'，{}）。工作区文件已回滚；对话历史保持不变。",
        target.label,
        short_sha(target.id.as_str()),
    ))
}

fn format_listing(snapshots: &[crate::snapshot::Snapshot]) -> String {
    let mut out = String::from("最近快照（最新在前；使用 /restore <N> 回滚）：\n");
    for (i, s) in snapshots.iter().enumerate() {
        out.push_str(&format!(
            "  #{:<2}  {}  {}\n",
            i + 1,
            short_sha(s.id.as_str()),
            s.label,
        ));
    }
    out
}

fn short_sha(sha: &str) -> &str {
    &sha[..sha.len().min(8)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::test_support::lock_test_env;
    use crate::tui::app::TuiOptions;
    use std::sync::MutexGuard;
    use tempfile::TempDir;

    fn make_app(tmp: &TempDir, yolo: bool) -> App {
        let workspace = tmp.path().to_path_buf();
        let options = TuiOptions {
            model: "mimo-v2.5-pro".to_string(),
            workspace,
            workspace_explicit: true,
            allow_shell: false,
            use_alt_screen: true,
            use_mouse_capture: false,
            use_bracketed_paste: true,
            max_subagents: 1,
            skills_dir: tmp.path().join("skills"),
            memory_path: tmp.path().join("memory.md"),
            notes_path: tmp.path().join("notes.txt"),
            mcp_config_path: tmp.path().join("mcp.json"),
            use_memory: false,
            start_in_agent_mode: false,
            skip_onboarding: true,
            yolo,
            resume_session_id: None,
        };
        App::new(options, &Config::default())
    }

    /// Pins HOME to a tempdir for the duration of the test under the
    /// crate-wide env mutex.
    struct ScopedHome {
        prev: Option<std::ffi::OsString>,
        _guard: MutexGuard<'static, ()>,
    }
    impl Drop for ScopedHome {
        fn drop(&mut self) {
            // SAFETY: process-wide lock still held.
            unsafe {
                match self.prev.take() {
                    Some(v) => std::env::set_var("HOME", v),
                    None => std::env::remove_var("HOME"),
                }
            }
        }
    }
    fn scoped_home(tmp: &TempDir) -> ScopedHome {
        let guard = lock_test_env();
        let prev = std::env::var_os("HOME");
        // SAFETY: serialised by the global env lock.
        unsafe {
            std::env::set_var("HOME", tmp.path());
        }
        ScopedHome {
            prev,
            _guard: guard,
        }
    }

    #[test]
    fn restore_with_no_snapshots_shows_empty_message() {
        let tmp = TempDir::new().unwrap();
        let _home = scoped_home(&tmp);
        let mut app = make_app(&tmp, true);
        let result = restore(&mut app, None);
        let msg = result.message.expect("expected message");
        assert!(msg.contains("还没有快照"));
    }

    #[test]
    fn restore_lists_when_no_arg_provided() {
        let tmp = TempDir::new().unwrap();
        let _home = scoped_home(&tmp);
        let mut app = make_app(&tmp, true);
        let repo = SnapshotRepo::open_or_init(&app.workspace).unwrap();
        std::fs::write(app.workspace.join("a.txt"), b"v1").unwrap();
        repo.snapshot("pre-turn:1").unwrap();
        std::fs::write(app.workspace.join("a.txt"), b"v2").unwrap();
        repo.snapshot("post-turn:1").unwrap();

        let result = restore(&mut app, None);
        let msg = result.message.expect("expected message");
        assert!(msg.contains("post-turn:1"));
        assert!(msg.contains("pre-turn:1"));
        assert!(msg.contains("#1"));
        assert!(msg.contains("#2"));
    }

    #[test]
    fn restore_in_yolo_reverts_workspace() {
        let tmp = TempDir::new().unwrap();
        let _home = scoped_home(&tmp);
        let mut app = make_app(&tmp, true);
        let repo = SnapshotRepo::open_or_init(&app.workspace).unwrap();
        let f = app.workspace.join("a.txt");

        std::fs::write(&f, b"original").unwrap();
        repo.snapshot("pre-turn:1").unwrap();
        std::fs::write(&f, b"clobbered").unwrap();
        repo.snapshot("post-turn:1").unwrap();

        let result = restore(&mut app, Some("2"));
        assert!(result.message.unwrap().contains("已恢复快照"));
        let after = std::fs::read_to_string(&f).unwrap();
        assert_eq!(after, "original");
    }

    #[test]
    fn restore_outside_trust_mode_refuses() {
        let tmp = TempDir::new().unwrap();
        let _home = scoped_home(&tmp);
        let mut app = make_app(&tmp, false);
        let repo = SnapshotRepo::open_or_init(&app.workspace).unwrap();
        std::fs::write(app.workspace.join("a.txt"), b"v1").unwrap();
        repo.snapshot("pre-turn:1").unwrap();

        let result = restore(&mut app, Some("1"));
        let msg = result.message.expect("expected message");
        assert!(msg.contains("拒绝恢复"));
        assert!(msg.contains("/trust on"));
    }

    #[test]
    fn restore_invalid_index_returns_error() {
        let tmp = TempDir::new().unwrap();
        let _home = scoped_home(&tmp);
        let mut app = make_app(&tmp, true);
        let repo = SnapshotRepo::open_or_init(&app.workspace).unwrap();
        std::fs::write(app.workspace.join("a.txt"), b"v1").unwrap();
        repo.snapshot("pre-turn:1").unwrap();

        let result = restore(&mut app, Some("99"));
        let msg = result.message.expect("expected message");
        assert!(msg.contains("当前只有 1 个快照"));
    }

    #[test]
    fn restore_zero_index_returns_error() {
        let tmp = TempDir::new().unwrap();
        let _home = scoped_home(&tmp);
        let mut app = make_app(&tmp, true);
        // Need at least one snapshot so we exercise the parse-index
        // branch instead of the "no snapshots" early return.
        let repo = SnapshotRepo::open_or_init(&app.workspace).unwrap();
        std::fs::write(app.workspace.join("a.txt"), b"v1").unwrap();
        repo.snapshot("pre-turn:1").unwrap();

        let result = restore(&mut app, Some("0"));
        let msg = result.message.expect("expected message");
        assert!(msg.contains("用法："));
    }
}
