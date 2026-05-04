//! Tool-output spillover writer.
//!
//! Oversized successful tool outputs are expensive to keep inline: they bloat
//! the transcript, can overwhelm the model context, and make the details pager
//! hard to scan. This module keeps the useful head inline and preserves the
//! full original output under `~/.xiaomimimo/tool_outputs/`.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use crate::tools::spec::ToolResult;

#[cfg(test)]
use std::path::Path;

/// Name of the spillover directory under `~/.xiaomimimo/`.
pub const SPILLOVER_DIR_NAME: &str = "tool_outputs";

/// Successful tool results larger than this are saved to disk and replaced
/// inline with a bounded preview.
pub const SPILLOVER_THRESHOLD_BYTES: usize = 100 * 1024;

/// Inline head retained after a spillover. The full output is on disk and can
/// be loaded through the path in the footer / metadata.
pub const SPILLOVER_HEAD_BYTES: usize = 32 * 1024;

/// Default boot-prune age for stale spillover files.
pub const SPILLOVER_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Resolve `~/.xiaomimimo/tool_outputs/`.
#[must_use]
pub fn spillover_root() -> Option<PathBuf> {
    Some(
        dirs::home_dir()?
            .join(".xiaomimimo")
            .join(SPILLOVER_DIR_NAME),
    )
}

/// Resolve the spillover file path for a tool call id.
#[must_use]
pub fn spillover_path(id: &str) -> Option<PathBuf> {
    let sanitised = sanitise_id(id)?;
    Some(spillover_root()?.join(format!("{sanitised}.txt")))
}

/// Write `content` to the spillover file for `id`.
pub fn write_spillover(id: &str, content: &str) -> io::Result<PathBuf> {
    let path = spillover_path(id).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "could not resolve spillover path (empty/invalid id or missing home directory)",
        )
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    crate::utils::write_atomic(&path, content.as_bytes())?;
    Ok(path)
}

/// Drop spillover files older than `max_age`.
///
/// Missing directories return `Ok(0)`. Per-file failures are logged and
/// skipped; a bad stale file should never block startup.
pub fn prune_older_than(max_age: Duration) -> io::Result<usize> {
    let Some(root) = spillover_root() else {
        return Ok(0);
    };
    if !root.exists() {
        return Ok(0);
    }

    let cutoff = SystemTime::now()
        .checked_sub(max_age)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut pruned = 0usize;

    for entry in fs::read_dir(&root)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                tracing::warn!(target: "spillover", ?err, "skipping unreadable dir entry");
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let modified = match entry.metadata().and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(err) => {
                tracing::warn!(target: "spillover", ?err, ?path, "skipping unreadable mtime");
                continue;
            }
        };
        if modified < cutoff {
            if let Err(err) = fs::remove_file(&path) {
                tracing::warn!(target: "spillover", ?err, ?path, "spillover prune skipped a file");
                continue;
            }
            pruned += 1;
        }
    }

    Ok(pruned)
}

/// Write a too-large `content` to disk and return the inline head plus path.
pub fn maybe_spillover(
    id: &str,
    content: &str,
    threshold: usize,
    head_bytes: usize,
) -> io::Result<Option<(String, PathBuf)>> {
    if content.len() <= threshold {
        return Ok(None);
    }

    let path = write_spillover(id, content)?;
    let cut = head_bytes.min(content.len());
    let cut = (0..=cut)
        .rev()
        .find(|&i| content.is_char_boundary(i))
        .unwrap_or(0);
    Ok(Some((content[..cut].to_string(), path)))
}

/// Apply spillover to a successful tool result in place.
///
/// Returns the spillover path when a write happened. Write failures are logged
/// but deliberately degrade to no-op so a successful tool call is not marked
/// failed because the sidecar store was unavailable.
pub fn apply_spillover(result: &mut ToolResult, tool_id: &str) -> Option<PathBuf> {
    if !result.success || result.content.len() <= SPILLOVER_THRESHOLD_BYTES {
        return None;
    }

    let total = result.content.len();
    let (head, path) = match maybe_spillover(
        tool_id,
        &result.content,
        SPILLOVER_THRESHOLD_BYTES,
        SPILLOVER_HEAD_BYTES,
    ) {
        Ok(Some(pair)) => pair,
        Ok(None) => return None,
        Err(err) => {
            tracing::warn!(
                target: "spillover",
                ?err,
                tool_id,
                "spillover write failed; passing original content through"
            );
            return None;
        }
    };

    let path_str = path.display().to_string();
    let footer = format!(
        "\n\n[Output truncated: {head_kib} KiB of {total_kib} KiB shown. \
         Full output saved to {path_str}. Use `read_file path={path_str}` \
         if you need the elided tail.]",
        head_kib = head.len() / 1024,
        total_kib = total / 1024,
    );
    result.content = format!("{head}{footer}");

    let metadata = result.metadata.get_or_insert_with(|| serde_json::json!({}));
    if let Some(obj) = metadata.as_object_mut() {
        obj.insert("spillover_path".into(), serde_json::Value::String(path_str));
    } else {
        let prior = std::mem::replace(metadata, serde_json::json!({}));
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("_prior".into(), prior);
            obj.insert(
                "spillover_path".into(),
                serde_json::Value::String(path.display().to_string()),
            );
        }
    }

    Some(path)
}

/// Sanitise a tool call id for use as a filename.
fn sanitise_id(id: &str) -> Option<String> {
    let cleaned: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(test)]
fn with_test_home<F, R>(home: &Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    // SAFETY: tests in this module serialize through `TEST_GUARD` because
    // they mutate process-global environment variables used by `dirs`.
    let prior_home = std::env::var_os("HOME");
    let prior_profile = std::env::var_os("USERPROFILE");
    unsafe {
        std::env::set_var("HOME", home);
        std::env::set_var("USERPROFILE", home);
    }
    let out = f();
    unsafe {
        if let Some(value) = prior_home {
            std::env::set_var("HOME", value);
        } else {
            std::env::remove_var("HOME");
        }
        if let Some(value) = prior_profile {
            std::env::set_var("USERPROFILE", value);
        } else {
            std::env::remove_var("USERPROFILE");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static TEST_GUARD: Mutex<()> = Mutex::new(());

    fn setup() -> std::sync::MutexGuard<'static, ()> {
        TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn sanitise_id_keeps_safe_chars_and_drops_dangerous() {
        assert_eq!(super::sanitise_id("abc-123_x"), Some("abc-123_x".into()));
        assert_eq!(super::sanitise_id("../etc"), Some("etc".into()));
        assert_eq!(super::sanitise_id("/etc/passwd"), Some("etcpasswd".into()));
        assert!(super::sanitise_id("...").is_none());
        assert!(super::sanitise_id("").is_none());
    }

    #[test]
    fn write_spillover_creates_directory_and_writes_file() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let path = write_spillover("call-abc", "hello world").expect("write");
            assert!(path.exists(), "{path:?} missing");
            assert_eq!(fs::read_to_string(&path).unwrap(), "hello world");

            let components: Vec<&str> = path
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();
            assert!(
                components.contains(&".xiaomimimo") && components.contains(&"tool_outputs"),
                "unexpected spillover path: {path:?}"
            );
        });
    }

    #[test]
    fn write_spillover_rejects_empty_id() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let err = write_spillover("...", "x").unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        });
    }

    #[test]
    fn maybe_spillover_returns_none_below_threshold() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let out = maybe_spillover("call-1", "tiny content", 100 * 1024, 4 * 1024).unwrap();
            assert!(out.is_none());
        });
    }

    #[test]
    fn maybe_spillover_writes_and_returns_head_above_threshold() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let big = "A".repeat(2_000);
            let (head, path) = maybe_spillover("call-2", &big, 1_000, 256)
                .unwrap()
                .expect("should spill");
            assert_eq!(head.len(), 256);
            assert_eq!(fs::read_to_string(&path).unwrap().len(), 2_000);
        });
    }

    #[test]
    fn maybe_spillover_does_not_split_inside_a_codepoint() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let s = "🐳🐳🐳🐳";
            assert_eq!(s.len(), 16);
            let (head, _) = maybe_spillover("call-3", s, 1, 3)
                .unwrap()
                .expect("spilled");
            assert_eq!(head, "");
            let (head, _) = maybe_spillover("call-3b", s, 1, 4)
                .unwrap()
                .expect("spilled");
            assert_eq!(head, "🐳");
        });
    }

    #[test]
    fn prune_older_than_handles_missing_root() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            assert_eq!(prune_older_than(SPILLOVER_MAX_AGE).unwrap(), 0);
        });
    }

    #[test]
    #[cfg(unix)]
    fn prune_older_than_keeps_fresh_files_drops_stale_ones() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let fresh = write_spillover("fresh", "x").unwrap();
            let stale = write_spillover("stale", "y").unwrap();

            let thirty_days = SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60);
            filetime_set_modified(&stale, thirty_days);

            assert_eq!(prune_older_than(SPILLOVER_MAX_AGE).unwrap(), 1);
            assert!(fresh.exists());
            assert!(!stale.exists());
        });
    }

    #[cfg(unix)]
    fn filetime_set_modified(path: &Path, when: SystemTime) {
        use std::os::unix::ffi::OsStrExt;

        let secs = when
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as libc::time_t;
        let times = [
            libc::timespec {
                tv_sec: secs,
                tv_nsec: 0,
            },
            libc::timespec {
                tv_sec: secs,
                tv_nsec: 0,
            },
        ];
        let path_c = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: `path_c` is a valid CString and `times` has the two entries
        // expected by `utimensat`.
        let rc = unsafe { libc::utimensat(libc::AT_FDCWD, path_c.as_ptr(), times.as_ptr(), 0) };
        assert_eq!(
            rc,
            0,
            "utimensat failed: {}",
            std::io::Error::last_os_error()
        );
    }

    #[test]
    fn apply_spillover_is_noop_below_threshold() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let mut result = ToolResult::success("small payload");
            assert!(apply_spillover(&mut result, "call-small").is_none());
            assert_eq!(result.content, "small payload");
            assert!(result.metadata.is_none());
        });
    }

    #[test]
    fn apply_spillover_is_noop_for_error_results() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let big_err = "boom\n".repeat(50_000);
            let mut result = ToolResult::error(big_err.clone());
            assert!(apply_spillover(&mut result, "call-err").is_none());
            assert_eq!(result.content, big_err);
        });
    }

    #[test]
    fn apply_spillover_truncates_and_stamps_metadata_above_threshold() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let big = "X".repeat(200 * 1024);
            let mut result = ToolResult::success(big.clone());
            let path = apply_spillover(&mut result, "call-big").expect("should spill");

            assert!(result.content.len() < big.len());
            assert!(result.content.contains("Output truncated:"));
            assert!(result.content.contains("read_file path="));
            assert!(path.exists());
            assert_eq!(fs::read_to_string(&path).unwrap().len(), 200 * 1024);

            let metadata = result.metadata.expect("metadata stamped");
            let stamped = metadata
                .get("spillover_path")
                .and_then(serde_json::Value::as_str)
                .expect("spillover_path key");
            assert_eq!(stamped, path.display().to_string());
        });
    }

    #[test]
    fn apply_spillover_preserves_existing_metadata() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let big = "Y".repeat(200 * 1024);
            let mut result =
                ToolResult::success(big).with_metadata(serde_json::json!({"prior_key": "prior"}));
            let path = apply_spillover(&mut result, "call-meta").expect("should spill");

            let metadata = result.metadata.expect("metadata present");
            assert_eq!(
                metadata
                    .get("prior_key")
                    .and_then(serde_json::Value::as_str),
                Some("prior")
            );
            assert_eq!(
                metadata
                    .get("spillover_path")
                    .and_then(serde_json::Value::as_str),
                Some(path.display().to_string().as_str())
            );
        });
    }

    #[test]
    fn apply_spillover_wraps_non_object_metadata_under_prior_key() {
        let _guard = setup();
        let tmp = tempdir().unwrap();
        with_test_home(tmp.path(), || {
            let big = "Z".repeat(200 * 1024);
            let mut result =
                ToolResult::success(big).with_metadata(serde_json::json!(["array", "payload"]));
            let path = apply_spillover(&mut result, "call-arr").expect("should spill");

            let metadata = result.metadata.expect("metadata stamped");
            assert_eq!(
                metadata.get("_prior"),
                Some(&serde_json::json!(["array", "payload"]))
            );
            assert_eq!(
                metadata
                    .get("spillover_path")
                    .and_then(serde_json::Value::as_str),
                Some(path.display().to_string().as_str())
            );
        });
    }
}
