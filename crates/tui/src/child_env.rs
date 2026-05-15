//! Sanitized environment handling for child processes.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};

/// Convert a string env map into owned OS strings for child env helpers.
pub fn string_map_env(
    env: &HashMap<String, String>,
) -> impl Iterator<Item = (OsString, OsString)> + '_ {
    env.iter()
        .map(|(key, value)| (OsString::from(key), OsString::from(value)))
}

/// Return the environment for a child process after dropping parent secrets.
///
/// `overrides` are trusted call-site values, such as sandbox markers, hook
/// variables, MCP server config, or RLM context path. They are applied after
/// the parent allowlist so explicit values win.
pub fn sanitized_child_env<I, K, V>(overrides: I) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut env = Vec::new();
    for (key, value) in std::env::vars_os() {
        if is_allowed_parent_env_key(&key) {
            upsert_env(&mut env, key, value);
        }
    }
    for (key, value) in overrides {
        upsert_env(
            &mut env,
            key.as_ref().to_os_string(),
            value.as_ref().to_os_string(),
        );
    }
    env
}

pub fn apply_to_command<I, K, V>(cmd: &mut std::process::Command, overrides: I)
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    cmd.env_clear();
    for (key, value) in sanitized_child_env(overrides) {
        cmd.env(key, value);
    }
}

pub fn apply_to_pty_command<I, K, V>(cmd: &mut portable_pty::CommandBuilder, overrides: I)
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    cmd.env_clear();
    for (key, value) in sanitized_child_env(overrides) {
        cmd.env(key, value);
    }
}

/// Build the sanitized child environment used for MCP stdio servers.
///
/// MCP stdio servers are user-configured integrations, so their allowlist is
/// wider than the base shell-tool allowlist: it preserves common Node/npm,
/// Python, Ruby, Java, proxy, CA-bundle, and Windows toolchain bootstrap
/// variables while still dropping arbitrary parent secrets.
pub fn sanitized_mcp_env<I, K, V>(overrides: I) -> Vec<(OsString, OsString)>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut env = Vec::new();
    for (key, value) in std::env::vars_os() {
        if is_allowed_mcp_env_key(&key) {
            upsert_env(&mut env, key, value);
        }
    }
    for (key, value) in overrides {
        upsert_env(
            &mut env,
            key.as_ref().to_os_string(),
            value.as_ref().to_os_string(),
        );
    }
    env
}

pub fn apply_to_tokio_command_mcp<I, K, V>(cmd: &mut tokio::process::Command, overrides: I)
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    cmd.env_clear();
    for (key, value) in sanitized_mcp_env(overrides) {
        cmd.env(key, value);
    }
}

fn is_allowed_parent_env_key(key: &OsStr) -> bool {
    let normalized = key.to_string_lossy().to_ascii_uppercase();
    matches!(
        normalized.as_str(),
        "PATH"
            | "HOME"
            | "USER"
            | "USERNAME"
            | "LOGNAME"
            | "LANG"
            | "LANGUAGE"
            | "LC_ALL"
            | "LC_CTYPE"
            | "LC_MESSAGES"
            | "TERM"
            | "COLORTERM"
            | "NO_COLOR"
            | "FORCE_COLOR"
            | "SHELL"
            | "TMPDIR"
            | "TMP"
            | "TEMP"
            | "__CF_USER_TEXT_ENCODING"
            | "SYSTEMROOT"
            | "WINDIR"
            | "COMSPEC"
            | "PATHEXT"
            | "USERPROFILE"
            | "HOMEDRIVE"
            | "HOMEPATH"
            // Preserve Windows developer shell state for cargo/MSVC builds.
            | "LIB"
            | "LIBPATH"
            | "INCLUDE"
            | "VSINSTALLDIR"
            | "VCINSTALLDIR"
            | "VCTOOLSINSTALLDIR"
            | "WINDOWSSDKDIR"
            | "WINDOWSSDKVERSION"
            | "UNIVERSALCRTSDKDIR"
            | "UCRTVERSION"
            | "EXTENSIONSDKDIR"
            | "DEVENVDIR"
            | "VISUALSTUDIOVERSION"
            // Proxy variables are required in many corporate/WSL networks.
            | "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "NO_PROXY"
            | "ALL_PROXY"
            | "FTP_PROXY"
    ) || normalized.starts_with("LC_")
}

/// Allowlist for MCP stdio launches. Strict superset of
/// `is_allowed_parent_env_key`.
fn is_allowed_mcp_env_key(key: &OsStr) -> bool {
    if is_allowed_parent_env_key(key) {
        return true;
    }
    let normalized = key.to_string_lossy().to_ascii_uppercase();
    if matches!(
        normalized.as_str(),
        "NVM_DIR"
            | "NVM_BIN"
            | "NVM_INC"
            | "VOLTA_HOME"
            | "COREPACK_HOME"
            | "NODE_PATH"
            | "NODE_OPTIONS"
            | "NODE_EXTRA_CA_CERTS"
            | "PYTHONPATH"
            | "PYTHONHOME"
            | "PYTHONDONTWRITEBYTECODE"
            | "PYTHONUNBUFFERED"
            | "VIRTUAL_ENV"
            | "POETRY_HOME"
            | "PIPX_HOME"
            | "PIPX_BIN_DIR"
            | "GEM_HOME"
            | "GEM_PATH"
            | "BUNDLE_PATH"
            | "BUNDLE_GEMFILE"
            | "JAVA_HOME"
            | "SSL_CERT_FILE"
            | "SSL_CERT_DIR"
            | "REQUESTS_CA_BUNDLE"
            | "CURL_CA_BUNDLE"
    ) {
        return true;
    }
    normalized.starts_with("NPM_CONFIG_") || normalized.starts_with("UV_")
}

fn upsert_env(env: &mut Vec<(OsString, OsString)>, key: OsString, value: OsString) {
    let normalized = normalize_key(&key);
    env.retain(|(existing, _)| normalize_key(existing) != normalized);
    env.push((key, value));
}

fn normalize_key(key: &OsStr) -> String {
    key.to_string_lossy().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn child_env_allowlist_includes_proxy_and_windows_toolchain_keys() {
        for key in [
            "PATH",
            "HTTP_PROXY",
            "https_proxy",
            "NO_PROXY",
            "LIB",
            "LIBPATH",
            "INCLUDE",
            "VCTOOLSINSTALLDIR",
            "WINDOWSSDKDIR",
        ] {
            assert!(
                is_allowed_parent_env_key(OsStr::new(key)),
                "child env allowlist should include {key}"
            );
        }
    }

    #[test]
    fn mcp_env_allowlist_includes_bootstrap_keys() {
        for key in [
            "NVM_DIR",
            "NODE_EXTRA_CA_CERTS",
            "NPM_CONFIG_CACHE",
            "PYTHONPATH",
            "VIRTUAL_ENV",
            "UV_CACHE_DIR",
            "REQUESTS_CA_BUNDLE",
        ] {
            assert!(
                is_allowed_mcp_env_key(OsStr::new(key)),
                "MCP allowlist should include {key}"
            );
        }
    }

    #[test]
    fn mcp_env_allowlist_excludes_secrets_and_creds() {
        for key in [
            "AWS_SECRET_ACCESS_KEY",
            "AWS_ACCESS_KEY_ID",
            "GITHUB_TOKEN",
            "OPENAI_API_KEY",
            "XIAOMIMIMO_API_KEY",
            "SLACK_TOKEN",
            "MY_RANDOM_SECRET",
        ] {
            assert!(
                !is_allowed_mcp_env_key(OsStr::new(key)),
                "MCP allowlist must not include {key}"
            );
        }
    }

    #[test]
    fn sanitized_child_env_drops_parent_secret_like_values() {
        let _guard = env_lock().lock().expect("env lock");
        let previous = std::env::var_os("XIAOMIMIMO_CHILD_ENV_TEST_SECRET");
        unsafe {
            std::env::set_var("XIAOMIMIMO_CHILD_ENV_TEST_SECRET", "parent-secret");
        }

        let env = sanitized_child_env(std::iter::empty::<(OsString, OsString)>());

        match previous {
            Some(value) => unsafe {
                std::env::set_var("XIAOMIMIMO_CHILD_ENV_TEST_SECRET", value);
            },
            None => unsafe {
                std::env::remove_var("XIAOMIMIMO_CHILD_ENV_TEST_SECRET");
            },
        }

        assert!(
            env.iter()
                .all(|(key, _)| key != "XIAOMIMIMO_CHILD_ENV_TEST_SECRET")
        );
    }

    #[test]
    fn explicit_child_env_values_win_over_parent_allowlist() {
        let _guard = env_lock().lock().expect("env lock");
        let previous = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "/parent/bin");
        }

        let env = sanitized_child_env([(OsString::from("PATH"), OsString::from("/explicit/bin"))]);

        match previous {
            Some(value) => unsafe {
                std::env::set_var("PATH", value);
            },
            None => unsafe {
                std::env::remove_var("PATH");
            },
        }

        let path = env
            .iter()
            .find(|(key, _)| normalize_key(key) == "PATH")
            .map(|(_, value)| value);
        assert_eq!(path, Some(&OsString::from("/explicit/bin")));
    }
}
