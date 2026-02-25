use serde::{Deserialize, Serialize};

/// Lightweight metadata for a skill (from SKILL.md frontmatter).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillMeta {
    /// Directory name used as the skill identifier.
    pub id: String,
    /// Human-readable name from frontmatter `name:` field.
    pub name: String,
    /// Single-line description from frontmatter `description:` field.
    pub description: String,
}

/// Full skill data including the body content after the frontmatter block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillDetail {
    pub meta: SkillMeta,
    /// Body text of SKILL.md (everything after the closing `---`).
    pub body: String,
}
