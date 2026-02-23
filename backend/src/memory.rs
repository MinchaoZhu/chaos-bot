use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct MemoryStore {
    memory_dir: PathBuf,
    curated_file: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryHit {
    pub path: String,
    pub line: usize,
    pub snippet: String,
}

#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn search(&self, keyword: &str) -> Result<Vec<MemoryHit>>;
    async fn append_daily_log(&self, summary: &str) -> Result<PathBuf>;
    async fn get_file(
        &self,
        relative_path: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Result<String>;
    async fn read_curated(&self) -> Result<String>;
    async fn write_curated(&self, content: &str) -> Result<()>;
    async fn ensure_layout(&self) -> Result<()>;
}

impl MemoryStore {
    pub fn new(memory_dir: impl Into<PathBuf>, curated_file: impl Into<PathBuf>) -> Self {
        Self {
            memory_dir: memory_dir.into(),
            curated_file: curated_file.into(),
        }
    }

    pub fn memory_dir(&self) -> &Path {
        &self.memory_dir
    }

    pub fn curated_file(&self) -> &Path {
        &self.curated_file
    }

    pub async fn ensure_layout(&self) -> Result<()> {
        if !self.memory_dir.exists() {
            fs::create_dir_all(&self.memory_dir).await?;
        }
        if !self.curated_file.exists() {
            fs::write(&self.curated_file, "# Long-Term Memory\n").await?;
        }
        Ok(())
    }

    pub async fn read_curated(&self) -> Result<String> {
        self.ensure_layout().await?;
        Ok(fs::read_to_string(&self.curated_file).await?)
    }

    pub async fn write_curated(&self, content: &str) -> Result<()> {
        self.ensure_layout().await?;
        fs::write(&self.curated_file, content).await?;
        Ok(())
    }

    pub async fn append_daily_log(&self, summary: &str) -> Result<PathBuf> {
        self.ensure_layout().await?;
        let file_path = self
            .memory_dir
            .join(format!("{}.md", Utc::now().format("%Y-%m-%d")));

        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;
        file.write_all(format!("- {}\n", summary.trim()).as_bytes())
            .await?;
        Ok(file_path)
    }

    pub async fn get_file(
        &self,
        relative_path: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Result<String> {
        let path = if relative_path == "MEMORY.md" {
            self.curated_file.clone()
        } else {
            self.memory_dir.join(relative_path)
        };

        if !path.exists() {
            return Err(anyhow!("memory file not found: {}", relative_path));
        }

        let content = fs::read_to_string(path).await?;
        let lines: Vec<&str> = content.lines().collect();

        if let (Some(start), Some(end)) = (start_line, end_line) {
            let from = start.saturating_sub(1);
            let to = end.min(lines.len());
            if from >= to {
                return Ok(String::new());
            }
            return Ok(lines[from..to].join("\n"));
        }

        Ok(content)
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<MemoryHit>> {
        self.ensure_layout().await?;
        let needle = keyword.trim().to_lowercase();
        if needle.is_empty() {
            return Ok(Vec::new());
        }

        let mut hits = Vec::new();
        for path in self.all_memory_files() {
            let content = match fs::read_to_string(&path).await {
                Ok(value) => value,
                Err(_) => continue,
            };
            for (idx, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    hits.push(MemoryHit {
                        path: path.to_string_lossy().to_string(),
                        line: idx + 1,
                        snippet: line.to_string(),
                    });
                }
            }
        }
        Ok(hits)
    }

    fn all_memory_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if self.curated_file.exists() {
            files.push(self.curated_file.clone());
        }
        if self.memory_dir.exists() {
            for entry in WalkDir::new(&self.memory_dir)
                .into_iter()
                .filter_map(std::result::Result::ok)
            {
                let path = entry.path();
                if path.is_file() {
                    files.push(path.to_path_buf());
                }
            }
        }
        files
    }
}

#[async_trait]
impl MemoryBackend for MemoryStore {
    async fn search(&self, keyword: &str) -> Result<Vec<MemoryHit>> {
        MemoryStore::search(self, keyword).await
    }

    async fn append_daily_log(&self, summary: &str) -> Result<PathBuf> {
        MemoryStore::append_daily_log(self, summary).await
    }

    async fn get_file(
        &self,
        relative_path: &str,
        start_line: Option<usize>,
        end_line: Option<usize>,
    ) -> Result<String> {
        MemoryStore::get_file(self, relative_path, start_line, end_line).await
    }

    async fn read_curated(&self) -> Result<String> {
        MemoryStore::read_curated(self).await
    }

    async fn write_curated(&self, content: &str) -> Result<()> {
        MemoryStore::write_curated(self, content).await
    }

    async fn ensure_layout(&self) -> Result<()> {
        MemoryStore::ensure_layout(self).await
    }
}
