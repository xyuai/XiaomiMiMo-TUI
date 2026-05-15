//! Skill discovery and registry for local SKILL.md files.

pub mod install;
mod system;
// Re-exports kept for documentation parity and downstream consumers; the
// binary itself imports directly from `skills::install`. `#[allow(...)]`
// silences the dead-code warning that fires because no `bin` source path
// references these names through `skills::*`.
#[allow(unused_imports)]
pub use install::{
    DEFAULT_MAX_SIZE_BYTES, DEFAULT_REGISTRY_URL, INSTALLED_FROM_MARKER, InstallOutcome,
    InstallSource, InstalledSkill, RegistryDocument, RegistryEntry, RegistryFetchResult,
    UpdateResult,
};
pub use system::install_system_skills;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::logging;

const MAX_SKILL_DESCRIPTION_CHARS: usize = 512;
const MAX_AVAILABLE_SKILLS_CHARS: usize = 12_000;
pub const ACTIVE_SKILL_BODY_MAX_CHARS: usize = 48_000;

// === Defaults ===

#[allow(dead_code)]
#[must_use]
pub fn default_skills_dir() -> PathBuf {
    dirs::home_dir().map_or_else(
        || PathBuf::from("/tmp/xiaomimimo/skills"),
        |p| p.join(".xiaomimimo").join("skills"),
    )
}

/// Workspace-local skills directory used ahead of configured/global skills.
#[must_use]
pub fn workspace_skills_dir(workspace: &Path) -> PathBuf {
    workspace.join(".xiaomimimo").join("skills")
}

/// Build the ordered skill search path for a workspace.
///
/// Earlier directories win on duplicate skill names. Kept small and explicit
/// so CLI/TUI/API callers can share the same precedence.
#[must_use]
pub fn skill_search_dirs(workspace: &Path, configured_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![
        workspace_skills_dir(workspace),
        workspace.join(".agents").join("skills"),
        workspace.join("skills"),
        configured_dir.to_path_buf(),
    ];
    let global_dir = default_skills_dir();
    dirs.push(global_dir);

    let mut seen = std::collections::HashSet::new();
    dirs.into_iter()
        .filter(|dir| seen.insert(normalize_for_dedup(dir)))
        .collect()
}

// === Types ===

/// Parsed representation of a SKILL.md definition.
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub body: String,
}

/// Collection of discovered skills.
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    skills: Vec<Skill>,
    warnings: Vec<String>,
}

impl SkillRegistry {
    /// Discover skills from the given directory.
    #[must_use]
    pub fn discover(dir: &Path) -> Self {
        let mut registry = Self::default();
        if !dir.exists() {
            return registry;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(ft) = entry.file_type()
                    && ft.is_dir()
                {
                    let skill_path = entry.path().join("SKILL.md");
                    match fs::read_to_string(&skill_path) {
                        Ok(content) => match Self::parse_skill(&skill_path, &content) {
                            Ok(mut skill) => {
                                skill.path = skill_path.clone();
                                registry.skills.push(skill);
                            }
                            Err(reason) => registry.push_warning(format!(
                                "Failed to parse {}: {reason}",
                                skill_path.display()
                            )),
                        },
                        Err(err) if skill_path.exists() => {
                            registry.push_warning(format!(
                                "Failed to read {}: {err}",
                                skill_path.display()
                            ));
                        }
                        Err(_) => {}
                    }
                }
            }
        } else {
            registry.push_warning(format!("Failed to read skills directory {}", dir.display()));
        }
        registry
    }

    /// Discover skills from multiple directories in precedence order.
    ///
    /// The first directory that defines a skill name wins; later duplicates are
    /// ignored so workspace-local skills can override global/system skills.
    #[must_use]
    pub fn discover_many<'a>(dirs: impl IntoIterator<Item = &'a Path>) -> Self {
        let mut merged = Self::default();
        let mut seen = std::collections::HashSet::new();

        for dir in dirs {
            let registry = Self::discover(dir);
            for warning in registry.warnings {
                merged.warnings.push(warning);
            }
            for skill in registry.skills {
                if seen.insert(skill.name.clone()) {
                    merged.skills.push(skill);
                }
            }
        }

        merged
    }

    fn push_warning(&mut self, warning: String) {
        logging::warn(&warning);
        self.warnings.push(warning);
    }

    fn parse_skill(path: &Path, content: &str) -> std::result::Result<Skill, String> {
        let trimmed = content.trim_start();
        let (frontmatter, body) = if trimmed.starts_with("---") {
            let start = content
                .find("---")
                .ok_or_else(|| "missing frontmatter opening delimiter".to_string())?;
            let rest = &content[start + 3..];
            let end = rest
                .find("---")
                .ok_or_else(|| "missing frontmatter closing delimiter".to_string())?;
            (&rest[..end], &rest[end + 3..])
        } else {
            return Err("missing frontmatter opening delimiter '---'".to_string());
        };

        let mut metadata = HashMap::new();
        for raw in frontmatter.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                metadata.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
            }
        }

        let name = metadata
            .get("name")
            .filter(|name| !name.is_empty())
            .cloned()
            .ok_or_else(|| "missing required frontmatter field: name".to_string())?;

        let description = metadata.get("description").cloned().unwrap_or_default();

        let body = body.trim().to_string();

        Ok(Skill {
            name,
            description,
            path: path.to_path_buf(),
            body,
        })
    }

    /// Lookup a skill by name.
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Return all loaded skills.
    pub fn list(&self) -> &[Skill] {
        &self.skills
    }

    /// Parse or I/O warnings encountered while discovering skills.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Check whether any skills were loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Return the number of loaded skills.
    #[must_use]
    pub fn len(&self) -> usize {
        self.skills.len()
    }
}

/// Render a stable, bounded instruction block for an activated skill.
///
/// Long skill files are truncated deterministically by preserving the start and
/// end. This keeps the selected workspace skill present even in large projects
/// while avoiding unbounded prompt growth.
#[must_use]
pub fn render_active_skill_instruction(skill: &Skill) -> String {
    let body = truncate_skill_body_for_activation(&skill.body, ACTIVE_SKILL_BODY_MAX_CHARS);
    format!(
        "You are now using a skill. Follow these instructions:\n\n# Skill: {}\n\n{}\n\n---\n\nNow respond to the user's request following the above skill instructions.",
        skill.name, body
    )
}

fn truncate_skill_body_for_activation(body: &str, max_chars: usize) -> String {
    if body.chars().count() <= max_chars {
        return body.to_string();
    }
    let marker = "\n\n[... skill body truncated; preserving beginning and end ...]\n\n";
    let marker_len = marker.chars().count();
    if max_chars <= marker_len + 2 {
        return body.chars().take(max_chars).collect();
    }
    let budget = max_chars - marker_len;
    let head_len = budget / 2;
    let tail_len = budget - head_len;
    let head: String = body.chars().take(head_len).collect();
    let tail_chars: Vec<char> = body.chars().rev().take(tail_len).collect();
    let tail: String = tail_chars.into_iter().rev().collect();
    format!("{head}{marker}{tail}")
}

/// Render a compact model-visible skills block.
///
/// The full `SKILL.md` body is intentionally not included here. This mirrors
/// Codex's progressive-disclosure contract: the model sees skill names,
/// descriptions, and paths up front, then opens the specific `SKILL.md` only
/// when a skill is relevant.
#[must_use]
#[allow(dead_code)]
pub fn render_available_skills_context(skills_dir: &Path) -> Option<String> {
    let registry = SkillRegistry::discover(skills_dir);
    render_available_skills_context_from_registry(registry)
}

/// Render a compact model-visible skills block from the shared workspace/global
/// search path.
#[must_use]
pub fn render_available_skills_context_for_workspace(
    workspace: &Path,
    configured_dir: &Path,
) -> Option<String> {
    let dirs = skill_search_dirs(workspace, configured_dir);
    let registry = SkillRegistry::discover_many(dirs.iter().map(PathBuf::as_path));
    render_available_skills_context_from_registry(registry)
}

fn render_available_skills_context_from_registry(registry: SkillRegistry) -> Option<String> {
    if registry.is_empty() {
        return None;
    }

    let mut skills = registry.list().to_vec();
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    let mut out = String::new();
    out.push_str("## Skills\n");
    out.push_str(
        "A skill is a set of local instructions stored in a `SKILL.md` file. \
Below is the list of skills available in this session. Each entry includes a \
name, description, and file path so you can open the source for full \
instructions when using a specific skill.\n\n",
    );
    out.push_str("### Available skills\n");

    let mut omitted = 0usize;
    for skill in skills {
        let path = skill.path.clone();
        let description = truncate_for_prompt(&skill.description, MAX_SKILL_DESCRIPTION_CHARS);
        let line = if description.is_empty() {
            format!("- {}: (file: {})\n", skill.name, path.display())
        } else {
            format!(
                "- {}: {} (file: {})\n",
                skill.name,
                description,
                path.display()
            )
        };

        if out.chars().count() + line.chars().count() > MAX_AVAILABLE_SKILLS_CHARS {
            omitted += 1;
        } else {
            out.push_str(&line);
        }
    }

    if omitted > 0 {
        out.push_str(&format!(
            "- ... {omitted} additional skills omitted from this prompt budget.\n"
        ));
    }

    if !registry.warnings().is_empty() {
        out.push_str("\n### Skill load warnings\n");
        for warning in registry.warnings().iter().take(8) {
            out.push_str("- ");
            out.push_str(&truncate_for_prompt(warning, MAX_SKILL_DESCRIPTION_CHARS));
            out.push('\n');
        }
    }

    out.push_str(
        "\n### How to use skills\n\
- Discovery: The list above is the skills available in this session. Skill bodies live on disk at the listed paths.\n\
- Trigger rules: If the user names a skill (with `$SkillName`, `/skill <name>`, or plain text) OR the task clearly matches a skill description above, use that skill for that turn. Multiple mentions mean use them all. Do not carry skills across turns unless re-mentioned.\n\
- Missing/blocked: If a named skill is missing or its `SKILL.md` cannot be read, say so briefly and continue with the best fallback.\n\
- Progressive disclosure: After deciding to use a skill, read only that skill's `SKILL.md`. When it references relative paths such as `scripts/foo.py`, resolve them relative to the skill directory.\n\
- Context hygiene: Load only the specific referenced files needed for the task. Avoid bulk-loading unrelated skill resources.\n\
- Safety: Do not execute scripts from a community skill unless the user explicitly asks or the skill has been trusted for script use.\n",
    );

    Some(out)
}

fn normalize_for_dedup(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn truncate_for_prompt(value: &str, max_chars: usize) -> String {
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= max_chars {
        return single_line;
    }

    let mut truncated = single_line
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

// === CLI Helpers ===

#[allow(dead_code)] // CLI utility for future use
pub fn list(skills_dir: &Path) -> Result<()> {
    if !skills_dir.exists() {
        println!("No skills directory found at {}", skills_dir.display());
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(skills_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            entries.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    if entries.is_empty() {
        println!("No skills found in {}", skills_dir.display());
        return Ok(());
    }

    entries.sort();
    for entry in entries {
        println!("{entry}");
    }
    Ok(())
}

#[allow(dead_code)] // CLI utility for future use
pub fn show(skills_dir: &Path, name: &str) -> Result<()> {
    let path = skills_dir.join(name).join("SKILL.md");
    let contents =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    println!("{contents}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    fn create_skill_dir(tmpdir: &TempDir, skill_name: &str, skill_content: &str) {
        let skill_dir = tmpdir.path().join("skills").join(skill_name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), skill_content).unwrap();
    }

    #[test]
    fn render_available_skills_context_lists_paths_and_usage() {
        let tmpdir = TempDir::new().unwrap();
        create_skill_dir(
            &tmpdir,
            "test-skill",
            "---\nname: test-skill\ndescription: A test skill\n---\nDo something special",
        );

        let rendered =
            crate::skills::render_available_skills_context(&tmpdir.path().join("skills"))
                .expect("skill context");

        let expected_path = tmpdir
            .path()
            .join("skills")
            .join("test-skill")
            .join("SKILL.md")
            .display()
            .to_string();

        assert!(rendered.contains("## Skills"));
        assert!(rendered.contains("- test-skill: A test skill"));
        assert!(
            rendered.contains(&expected_path),
            "expected path {expected_path:?} not in rendered output"
        );
        assert!(rendered.contains("### How to use skills"));
    }

    #[test]
    fn render_available_skills_context_uses_real_dir_name_not_frontmatter_name() {
        let tmpdir = TempDir::new().unwrap();
        create_skill_dir(
            &tmpdir,
            "actual-dir",
            "---\nname: display-name\ndescription: A test skill\n---\nbody",
        );

        let rendered =
            crate::skills::render_available_skills_context(&tmpdir.path().join("skills"))
                .expect("skill context");
        let real_path = tmpdir
            .path()
            .join("skills")
            .join("actual-dir")
            .join("SKILL.md")
            .display()
            .to_string();
        let fake_path = tmpdir
            .path()
            .join("skills")
            .join("display-name")
            .join("SKILL.md")
            .display()
            .to_string();

        assert!(rendered.contains("- display-name: A test skill"));
        assert!(rendered.contains(&real_path), "got:\n{rendered}");
        assert!(!rendered.contains(&fake_path), "got:\n{rendered}");
    }

    #[test]
    fn render_available_skills_context_returns_none_when_empty() {
        let tmpdir = TempDir::new().unwrap();
        let empty = tmpdir.path().join("skills");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(crate::skills::render_available_skills_context(&empty).is_none());

        let missing = tmpdir.path().join("does-not-exist");
        assert!(crate::skills::render_available_skills_context(&missing).is_none());
    }

    #[test]
    fn render_available_skills_context_truncates_long_descriptions() {
        let tmpdir = TempDir::new().unwrap();
        let long_desc = "x".repeat(2_000);
        let body = format!("---\nname: bigdesc\ndescription: {long_desc}\n---\nbody");
        create_skill_dir(&tmpdir, "bigdesc", &body);

        let rendered =
            crate::skills::render_available_skills_context(&tmpdir.path().join("skills"))
                .expect("skill context");

        let max = super::MAX_SKILL_DESCRIPTION_CHARS;
        assert!(rendered.contains('…'), "expected truncation marker");
        assert!(
            !rendered.contains(&"x".repeat(max + 1)),
            "untruncated long run should not appear"
        );
    }

    #[test]
    fn render_available_skills_context_collapses_internal_whitespace() {
        let tmpdir = TempDir::new().unwrap();
        create_skill_dir(
            &tmpdir,
            "spaced-skill",
            "---\nname: spaced-skill\ndescription: alpha  \t  beta   gamma\n---\nbody",
        );

        let rendered =
            crate::skills::render_available_skills_context(&tmpdir.path().join("skills"))
                .expect("skill context");

        let line = rendered
            .lines()
            .find(|l| l.starts_with("- spaced-skill:"))
            .expect("skill line");
        assert!(line.contains("alpha beta gamma"), "got: {line:?}");
    }

    #[test]
    fn render_available_skills_context_omits_overflowing_skills() {
        let tmpdir = TempDir::new().unwrap();
        let big_desc = "y".repeat(super::MAX_SKILL_DESCRIPTION_CHARS - 20);
        for i in 0..200 {
            let body = format!("---\nname: skill-{i:03}\ndescription: {big_desc}\n---\nbody");
            create_skill_dir(&tmpdir, &format!("skill-{i:03}"), &body);
        }

        let rendered =
            crate::skills::render_available_skills_context(&tmpdir.path().join("skills"))
                .expect("skill context");

        assert!(
            rendered.contains("additional skills omitted from this prompt budget"),
            "expected overflow notice"
        );
        assert!(
            rendered.chars().count() < super::MAX_AVAILABLE_SKILLS_CHARS + 4_000,
            "rendered length should stay near the budget"
        );
    }

    #[test]
    fn discover_many_uses_workspace_precedence() {
        let tmpdir = TempDir::new().unwrap();
        let workspace = tmpdir.path().join("workspace");
        let global = tmpdir.path().join("global");
        std::fs::create_dir_all(workspace.join("dupe")).unwrap();
        std::fs::create_dir_all(global.join("dupe")).unwrap();
        std::fs::write(
            workspace.join("dupe").join("SKILL.md"),
            "---\nname: dupe\ndescription: workspace\n---\nworkspace body",
        )
        .unwrap();
        std::fs::write(
            global.join("dupe").join("SKILL.md"),
            "---\nname: dupe\ndescription: global\n---\nglobal body",
        )
        .unwrap();

        let registry =
            crate::skills::SkillRegistry::discover_many([workspace.as_path(), global.as_path()]);
        let skill = registry.get("dupe").expect("skill");
        assert_eq!(skill.description, "workspace");
        assert_eq!(skill.body, "workspace body");
    }

    #[test]
    fn active_skill_instruction_is_bounded_and_stable() {
        let body = format!(
            "---\nname: huge\ndescription: Huge skill\n---\n{}MIDDLE{}",
            "A".repeat(super::ACTIVE_SKILL_BODY_MAX_CHARS),
            "Z".repeat(super::ACTIVE_SKILL_BODY_MAX_CHARS)
        );
        let path = std::path::PathBuf::from("SKILL.md");
        let skill = crate::skills::SkillRegistry::parse_skill(&path, &body).unwrap();
        let rendered = crate::skills::render_active_skill_instruction(&skill);

        assert!(rendered.contains("A"));
        assert!(rendered.contains("Z"));
        assert!(rendered.contains("skill body truncated"));
        assert!(rendered.chars().count() < super::ACTIVE_SKILL_BODY_MAX_CHARS + 512);
    }
}
