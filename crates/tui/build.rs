use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");

    let manifest_dir =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_else(|| ".".into()));
    let repo_root = manifest_dir.join("../..");
    watch_git_head(&repo_root);

    if let Some(sha) = env_or_git("GITHUB_SHA", &repo_root, &["rev-parse", "--verify", "HEAD"]) {
        println!("cargo:rustc-env=XIAOMIMIMO_BUILD_GIT_SHA={sha}");
    }
    if let Some(tag) = git_output(&repo_root, &["describe", "--tags", "--exact-match", "HEAD"]) {
        println!("cargo:rustc-env=XIAOMIMIMO_BUILD_GIT_TAG={tag}");
    }
    if let Some(ref_name) = std::env::var("GITHUB_REF_NAME")
        .ok()
        .filter(|s| !s.is_empty())
    {
        println!("cargo:rustc-env=XIAOMIMIMO_BUILD_GIT_REF={ref_name}");
    }
}

fn watch_git_head(repo_root: &Path) {
    let git_dir = repo_root.join(".git");
    let head = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head.display());
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );

    let Ok(head_text) = std::fs::read_to_string(&head) else {
        return;
    };
    let Some(ref_name) = head_text.trim().strip_prefix("ref: ") else {
        return;
    };
    println!(
        "cargo:rerun-if-changed={}",
        git_dir
            .join(ref_name.replace('/', std::path::MAIN_SEPARATOR_STR))
            .display()
    );
    if let Some(branch) = ref_name.strip_prefix("refs/heads/") {
        println!(
            "cargo:rerun-if-changed={}",
            git_dir
                .join("logs")
                .join("refs")
                .join("heads")
                .join(branch)
                .display()
        );
    }
}

fn env_or_git(env_name: &str, repo_root: &Path, args: &[&str]) -> Option<String> {
    std::env::var(env_name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| git_output(repo_root, args))
}

fn git_output(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}
