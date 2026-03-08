use crate::domain::ports::SkillPort;
use crate::domain::skills::{SkillDetail, SkillMeta};
use crate::infrastructure::runtime_assets::DEFAULT_SKILL_CREATOR_MD;
use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;
use walkdir::WalkDir;

// ---------------------------------------------------------------------------
// Frontmatter parsing
// ---------------------------------------------------------------------------

/// Parse a SKILL.md file into (name, description, body).
/// Frontmatter is expected between the first pair of `---` delimiters.
fn parse_skill_md(content: &str) -> (String, String, String) {
    if let Some(after_open) = content.strip_prefix("---\n") {
        if let Some(close_idx) = after_open.find("\n---\n") {
            let fm = &after_open[..close_idx];
            let body = after_open[close_idx + 5..].trim_start().to_string();
            let name = extract_fm_field(fm, "name");
            let description = extract_fm_field(fm, "description");
            return (name, description, body);
        }
        // Handle trailing `\n---` at end of file (no newline after)
        if let Some(close_idx) = after_open.find("\n---") {
            let fm = &after_open[..close_idx];
            let raw_tail = &after_open[close_idx + 4..];
            let body = raw_tail.trim_start_matches(['\n', '\r']).to_string();
            let name = extract_fm_field(fm, "name");
            let description = extract_fm_field(fm, "description");
            return (name, description, body);
        }
    }
    (String::new(), String::new(), content.to_string())
}

fn extract_fm_field(fm: &str, key: &str) -> String {
    let prefix = format!("{key}:");
    for line in fm.lines() {
        if let Some(val) = line.strip_prefix(&prefix) {
            return val.trim().trim_matches('"').to_string();
        }
    }
    String::new()
}

fn parse_git_source(source: &str) -> Result<(String, Option<String>, PathBuf)> {
    let source = source.trim();
    if source.is_empty() {
        bail!("skill source is required");
    }

    if source.ends_with(".git") {
        return Ok((source.to_string(), None, PathBuf::from(".")));
    }

    // Supports GitHub tree URL:
    // https://github.com/<owner>/<repo>/tree/<branch>/<path...>
    let Some(tree_marker) = source.find("/tree/") else {
        bail!("unsupported source format; expected .git URL or GitHub tree URL");
    };

    let repo_prefix = &source[..tree_marker];
    let tree_tail = &source[tree_marker + "/tree/".len()..];
    let mut segments = tree_tail.split('/').filter(|segment| !segment.is_empty());
    let Some(branch) = segments.next() else {
        bail!("invalid GitHub tree URL: missing branch");
    };
    let subpath = segments.collect::<Vec<_>>().join("/");
    let clone_url = if repo_prefix.ends_with(".git") {
        repo_prefix.to_string()
    } else {
        format!("{repo_prefix}.git")
    };

    if !clone_url.contains("github.com/") {
        bail!("unsupported tree URL host; only github.com tree URLs are supported");
    }

    Ok((
        clone_url,
        Some(branch.to_string()),
        if subpath.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(subpath)
        },
    ))
}

fn validate_skill_id(skill_name: &str) -> Result<()> {
    if skill_name.trim().is_empty() {
        bail!("skill_name is required");
    }
    if skill_name.contains("..") || skill_name.contains('/') || skill_name.contains('\\') {
        bail!("skill_name must be a folder name");
    }
    Ok(())
}

pub async fn read_skill_markdown(skills_dir: &Path, skill_name: &str) -> Result<String> {
    validate_skill_id(skill_name)?;
    let skill_md = skills_dir.join(skill_name).join("SKILL.md");
    if !skill_md.exists() {
        bail!("skill '{}' not found", skill_name);
    }
    Ok(fs::read_to_string(skill_md).await?)
}

fn copy_directory_recursive(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in WalkDir::new(from)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let src_path = entry.path();
        let relative = src_path
            .strip_prefix(from)
            .with_context(|| format!("failed to strip prefix from {}", src_path.display()))?;
        let dst_path = to.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&dst_path)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(src_path, &dst_path)?;
        }
    }
    Ok(())
}

pub async fn install_skills_from_git_source(
    skills_dir: &Path,
    source: &str,
) -> Result<Vec<SkillMeta>> {
    let (clone_url, branch, subpath) = parse_git_source(source)?;
    let install_root = std::env::temp_dir().join(format!(
        "chaos-bot-skill-install-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ));
    let repo_dir = install_root.join("repo");
    fs::create_dir_all(&install_root).await?;

    let result = async {
        let mut clone = Command::new("git");
        clone.arg("clone").arg("--depth").arg("1");
        if let Some(branch) = &branch {
            clone.arg("--branch").arg(branch);
        }
        clone.arg(&clone_url).arg(&repo_dir);

        let output = clone
            .output()
            .await
            .context("failed to execute git clone")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("git clone failed: {}", stderr.trim());
        }

        let target_root = repo_dir.join(&subpath);
        if !target_root.exists() {
            bail!(
                "path '{}' not found in cloned repository",
                subpath.display()
            );
        }

        fs::create_dir_all(skills_dir).await?;
        let mut skill_dirs = BTreeSet::new();
        for entry in WalkDir::new(&target_root)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            if !entry.file_type().is_file() || entry.file_name() != "SKILL.md" {
                continue;
            }
            if let Some(parent) = entry.path().parent() {
                skill_dirs.insert(parent.to_path_buf());
            }
        }
        if skill_dirs.is_empty() {
            bail!("no SKILL.md found under {}", target_root.display());
        }

        let mut installed = Vec::new();
        for src_dir in skill_dirs {
            let id = src_dir
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .ok_or_else(|| anyhow!("invalid skill folder path: {}", src_dir.display()))?;
            validate_skill_id(&id)?;

            let dst_dir = skills_dir.join(&id);
            if dst_dir.exists() {
                fs::remove_dir_all(&dst_dir).await?;
            }
            copy_directory_recursive(&src_dir, &dst_dir)?;

            let content = fs::read_to_string(dst_dir.join("SKILL.md")).await?;
            let (name, description, _body) = parse_skill_md(&content);
            installed.push(SkillMeta {
                id: id.clone(),
                name: if name.is_empty() { id } else { name },
                description,
            });
        }
        installed.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(installed)
    }
    .await;

    let _ = fs::remove_dir_all(&install_root).await;
    result
}

// ---------------------------------------------------------------------------
// SkillStore — filesystem-backed implementation
// ---------------------------------------------------------------------------

pub struct SkillStore {
    skills_dir: PathBuf,
}

impl SkillStore {
    pub fn new(skills_dir: PathBuf) -> Self {
        Self { skills_dir }
    }
}

#[async_trait]
impl SkillPort for SkillStore {
    async fn ensure_layout(&self) -> Result<()> {
        fs::create_dir_all(&self.skills_dir).await?;
        let creator_dir = self.skills_dir.join("skill-creator");
        if !creator_dir.exists() {
            fs::create_dir_all(&creator_dir).await?;
            fs::write(creator_dir.join("SKILL.md"), DEFAULT_SKILL_CREATOR_MD).await?;
            tracing::info!(skill_id = "skill-creator", "seeded built-in skill");
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<SkillMeta>> {
        if !self.skills_dir.exists() {
            return Ok(Vec::new());
        }

        let mut skills = Vec::new();
        let mut entries = fs::read_dir(&self.skills_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }
            let content = match fs::read_to_string(&skill_md).await {
                Ok(c) => c,
                Err(e) => {
                    let id = entry.file_name().to_string_lossy().to_string();
                    tracing::warn!(skill_id = %id, error = %e, "failed to read SKILL.md; skipping");
                    continue;
                }
            };
            let id = entry.file_name().to_string_lossy().to_string();
            let (name, description, _body) = parse_skill_md(&content);
            skills.push(SkillMeta {
                name: if name.is_empty() { id.clone() } else { name },
                description,
                id,
            });
        }

        skills.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(skills)
    }

    async fn get(&self, id: &str) -> Result<SkillDetail> {
        let skill_md = self.skills_dir.join(id).join("SKILL.md");
        if !skill_md.exists() {
            bail!("skill '{}' not found", id);
        }
        let content = fs::read_to_string(&skill_md).await?;
        let (name, description, body) = parse_skill_md(&content);
        let meta = SkillMeta {
            id: id.to_string(),
            name: if name.is_empty() {
                id.to_string()
            } else {
                name
            },
            description,
        };
        Ok(SkillDetail { meta, body })
    }
}

// ---------------------------------------------------------------------------
// EmptySkillStore — no-op for tests and backward compatibility
// ---------------------------------------------------------------------------

pub struct EmptySkillStore;

#[async_trait]
impl SkillPort for EmptySkillStore {
    async fn ensure_layout(&self) -> Result<()> {
        Ok(())
    }

    async fn list(&self) -> Result<Vec<SkillMeta>> {
        Ok(Vec::new())
    }

    async fn get(&self, id: &str) -> Result<SkillDetail> {
        bail!("skill '{}' not found", id)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn ensure_layout_creates_skill_creator() {
        let tmp = tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        let store = SkillStore::new(skills_dir.clone());

        store.ensure_layout().await.unwrap();

        assert!(skills_dir.exists());
        assert!(skills_dir.join("skill-creator").join("SKILL.md").exists());
    }

    #[tokio::test]
    async fn list_returns_installed_skills() {
        let tmp = tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        std::fs::create_dir_all(skills_dir.join("my-skill")).unwrap();
        std::fs::write(
            skills_dir.join("my-skill").join("SKILL.md"),
            "---\nname: My Skill\ndescription: Does something.\n---\n\nBody text.",
        )
        .unwrap();

        let store = SkillStore::new(skills_dir);
        let list = store.list().await.unwrap();

        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "my-skill");
        assert_eq!(list[0].name, "My Skill");
        assert_eq!(list[0].description, "Does something.");
    }

    #[tokio::test]
    async fn get_returns_skill_detail() {
        let tmp = tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        std::fs::create_dir_all(skills_dir.join("demo")).unwrap();
        std::fs::write(
            skills_dir.join("demo").join("SKILL.md"),
            "---\nname: Demo\ndescription: Demo skill.\n---\n\nHere are the instructions.",
        )
        .unwrap();

        let store = SkillStore::new(skills_dir);
        let detail = store.get("demo").await.unwrap();

        assert_eq!(detail.meta.id, "demo");
        assert_eq!(detail.meta.name, "Demo");
        assert_eq!(detail.body, "Here are the instructions.");
    }

    #[tokio::test]
    async fn get_missing_skill_errors() {
        let tmp = tempdir().unwrap();
        let store = SkillStore::new(tmp.path().join("skills"));
        assert!(store.get("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn list_skips_files_without_skill_md() {
        let tmp = tempdir().unwrap();
        let skills_dir = tmp.path().join("skills");
        std::fs::create_dir_all(skills_dir.join("incomplete")).unwrap();
        // no SKILL.md in incomplete/
        let store = SkillStore::new(skills_dir);
        let list = store.list().await.unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn parse_frontmatter_basic() {
        let content = "---\nname: Test\ndescription: A test skill.\n---\n\nBody here.";
        let (name, desc, body) = parse_skill_md(content);
        assert_eq!(name, "Test");
        assert_eq!(desc, "A test skill.");
        assert_eq!(body, "Body here.");
    }

    #[test]
    fn parse_frontmatter_no_fm() {
        let content = "Just plain text.";
        let (_name, _desc, body) = parse_skill_md(content);
        assert_eq!(body, "Just plain text.");
    }

    #[tokio::test]
    async fn empty_skill_store_returns_empty() {
        let store = EmptySkillStore;
        assert!(store.list().await.unwrap().is_empty());
        assert!(store.get("any").await.is_err());
    }

    #[test]
    fn parse_git_source_supports_git_url() {
        let (clone_url, branch, subpath) =
            parse_git_source("https://github.com/acme/skills.git").unwrap();
        assert_eq!(clone_url, "https://github.com/acme/skills.git");
        assert!(branch.is_none());
        assert_eq!(subpath, PathBuf::from("."));
    }

    #[test]
    fn parse_git_source_supports_github_tree_url() {
        let (clone_url, branch, subpath) =
            parse_git_source("https://github.com/acme/skills/tree/main/skills/skill-creator")
                .unwrap();
        assert_eq!(clone_url, "https://github.com/acme/skills.git");
        assert_eq!(branch.as_deref(), Some("main"));
        assert_eq!(subpath, PathBuf::from("skills/skill-creator"));
    }

    #[tokio::test]
    async fn read_skill_markdown_rejects_invalid_name() {
        let tmp = tempdir().unwrap();
        let result = read_skill_markdown(tmp.path(), "../oops").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn install_skills_from_local_git_repo() {
        let tmp = tempdir().unwrap();
        let remote = tmp.path().join("remote.git");
        std::fs::create_dir_all(&remote).unwrap();

        let init = std::process::Command::new("git")
            .arg("init")
            .arg("-b")
            .arg("main")
            .arg(&remote)
            .output()
            .unwrap();
        assert!(init.status.success());

        let config_email = std::process::Command::new("git")
            .arg("-C")
            .arg(&remote)
            .arg("config")
            .arg("user.email")
            .arg("skill-test@example.com")
            .output()
            .unwrap();
        assert!(config_email.status.success());
        let config_name = std::process::Command::new("git")
            .arg("-C")
            .arg(&remote)
            .arg("config")
            .arg("user.name")
            .arg("Skill Tester")
            .output()
            .unwrap();
        assert!(config_name.status.success());

        let skill_dir = remote.join("skills").join("demo-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: Demo Skill\ndescription: Demo.\n---\n\nBody.",
        )
        .unwrap();

        let add = std::process::Command::new("git")
            .arg("-C")
            .arg(&remote)
            .arg("add")
            .arg(".")
            .output()
            .unwrap();
        assert!(add.status.success());
        let commit = std::process::Command::new("git")
            .arg("-C")
            .arg(&remote)
            .arg("commit")
            .arg("-m")
            .arg("init")
            .output()
            .unwrap();
        assert!(commit.status.success());

        let installed_dir = tmp.path().join("installed-skills");
        let installed = install_skills_from_git_source(&installed_dir, &remote.to_string_lossy())
            .await
            .unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id, "demo-skill");
        assert!(installed_dir.join("demo-skill").join("SKILL.md").exists());
    }
}
