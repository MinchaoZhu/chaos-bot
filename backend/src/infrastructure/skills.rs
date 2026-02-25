use crate::domain::ports::SkillPort;
use crate::domain::skills::{SkillDetail, SkillMeta};
use crate::infrastructure::runtime_assets::DEFAULT_SKILL_CREATOR_MD;
use anyhow::{bail, Result};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

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
            tracing::info!(
                skill_id = "skill-creator",
                "seeded built-in skill"
            );
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
            name: if name.is_empty() { id.to_string() } else { name },
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
}
