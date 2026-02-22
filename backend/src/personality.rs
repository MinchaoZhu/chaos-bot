use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::fs;

const PERSONALITY_ORDER: [&str; 4] = ["SOUL.md", "IDENTITY.md", "USER.md", "AGENTS.md"];

#[derive(Clone, Debug)]
pub struct PersonalityLoader {
    dir: PathBuf,
}

impl PersonalityLoader {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub async fn load_sections(&self) -> Result<Vec<(String, String)>> {
        let mut sections = Vec::new();
        for filename in PERSONALITY_ORDER {
            let path = self.dir.join(filename);
            if path.exists() {
                let content = fs::read_to_string(path).await?;
                sections.push((filename.to_string(), content));
            }
        }
        Ok(sections)
    }

    pub async fn system_prompt(&self) -> Result<String> {
        let sections = self.load_sections().await?;
        let mut out = String::new();
        for (name, content) in sections {
            out.push_str(&format!("## {}\n{}\n\n", name, content.trim()));
        }
        Ok(out)
    }
}
