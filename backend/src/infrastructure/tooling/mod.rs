use crate::domain::ports::{ToolExecutionContext, ToolExecutorPort};
use crate::domain::types::{ToolExecution, ToolResult, ToolSpec};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use walkdir::WalkDir;

pub type ToolContext = ToolExecutionContext;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution>;
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn register_coding_tools(&mut self) {
        self.register(ReadTool);
        self.register(WriteTool);
        self.register(EditTool);
        self.register(BashTool);
    }

    pub fn register_read_only_tools(&mut self) {
        self.register(ReadTool);
        self.register(GrepTool);
        self.register(FindTool);
        self.register(LsTool);
    }

    pub fn register_memory_tools(&mut self) {
        self.register(MemoryGetTool);
        self.register(MemorySearchTool);
    }

    pub fn register_default_tools(&mut self) {
        self.register_coding_tools();
        self.register_read_only_tools();
        self.register_memory_tools();
    }

    pub fn specs(&self) -> Vec<ToolSpec> {
        self.tools
            .values()
            .map(|tool| ToolSpec {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters_schema: tool.parameters_schema(),
            })
            .collect()
    }

    pub async fn dispatch(
        &self,
        tool_call_id: &str,
        name: &str,
        args: Value,
        context: &ToolContext,
    ) -> Result<ToolResult> {
        tracing::debug!(tool_call_id, tool_name = name, "dispatching tool");
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| anyhow!("tool not found: {}", name))?
            .clone();

        let output = tool.execute(args, context).await?;
        tracing::debug!(
            tool_call_id,
            tool_name = name,
            is_error = output.is_error,
            "tool execution completed"
        );

        Ok(ToolResult {
            tool_call_id: tool_call_id.to_string(),
            name: name.to_string(),
            output: output.output,
            is_error: output.is_error,
        })
    }
}

#[async_trait]
impl ToolExecutorPort for ToolRegistry {
    fn specs(&self) -> Vec<ToolSpec> {
        ToolRegistry::specs(self)
    }

    async fn execute(
        &self,
        tool_call_id: &str,
        name: &str,
        args: Value,
        context: &ToolExecutionContext,
    ) -> Result<ToolResult> {
        self.dispatch(tool_call_id, name, args, context).await
    }
}

pub fn ensure_within_root(root_dir: &Path, path: &Path) -> Result<()> {
    let root = std::fs::canonicalize(root_dir)
        .with_context(|| format!("cannot canonicalize root: {}", root_dir.display()))?;
    let candidate = std::fs::canonicalize(path)
        .with_context(|| format!("cannot canonicalize path: {}", path.display()))?;

    if candidate.starts_with(&root) {
        Ok(())
    } else {
        Err(anyhow!(
            "path escapes working directory: {}",
            candidate.display()
        ))
    }
}

fn resolve_existing_path(root_dir: &Path, input_path: &str) -> Result<PathBuf> {
    let path = if Path::new(input_path).is_absolute() {
        PathBuf::from(input_path)
    } else {
        root_dir.join(input_path)
    };

    if !path.exists() {
        return Err(anyhow!("path does not exist: {}", path.display()));
    }

    ensure_within_root(root_dir, &path)?;
    Ok(path)
}

fn resolve_existing_path_unrestricted(root_dir: &Path, input_path: &str) -> Result<PathBuf> {
    let path = if Path::new(input_path).is_absolute() {
        PathBuf::from(input_path)
    } else {
        root_dir.join(input_path)
    };

    if !path.exists() {
        return Err(anyhow!("path does not exist: {}", path.display()));
    }

    Ok(path)
}

fn resolve_write_path_unrestricted(root_dir: &Path, input_path: &str) -> Result<PathBuf> {
    let path = if Path::new(input_path).is_absolute() {
        PathBuf::from(input_path)
    } else {
        root_dir.join(input_path)
    };

    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("invalid write path: {}", input_path))?;
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(path)
}

pub fn slice_lines(content: &str, start_line: Option<usize>, end_line: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    match (start_line, end_line) {
        (Some(start), Some(end)) => {
            let from = start.saturating_sub(1);
            let to = end.min(lines.len());
            if from >= to {
                String::new()
            } else {
                lines[from..to].join("\n")
            }
        }
        (Some(start), None) => {
            let from = start.saturating_sub(1);
            if from >= lines.len() {
                String::new()
            } else {
                lines[from..].join("\n")
            }
        }
        _ => content.to_string(),
    }
}

pub struct ReadTool;

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read a text file from the working directory"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "start_line": {"type": "integer", "minimum": 1},
                "end_line": {"type": "integer", "minimum": 1}
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("read.path is required"))?;
        let start_line = args
            .get("start_line")
            .and_then(|value| value.as_u64())
            .map(|v| v as usize);
        let end_line = args
            .get("end_line")
            .and_then(|value| value.as_u64())
            .map(|v| v as usize);

        let resolved = resolve_existing_path(&context.root_dir, path)?;
        let content = fs::read_to_string(resolved).await?;
        let output = slice_lines(&content, start_line, end_line);

        Ok(ToolExecution {
            name: self.name().to_string(),
            output,
            is_error: false,
        })
    }
}

pub struct WriteTool;

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "write"
    }

    fn description(&self) -> &'static str {
        "Write content to a file under the working directory"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "content": {"type": "string"},
                "append": {"type": "boolean"}
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("write.path is required"))?;
        let content = args
            .get("content")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("write.content is required"))?;
        let append = args
            .get("append")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        let resolved = resolve_write_path_unrestricted(&context.root_dir, path)?;
        if append {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(&resolved)
                .await?;
            file.write_all(content.as_bytes()).await?;
            file.flush().await?;
            file.sync_data().await?;
        } else {
            fs::write(&resolved, content).await?;
        }
        let final_path = std::fs::canonicalize(&resolved).unwrap_or(resolved.clone());

        Ok(ToolExecution {
            name: self.name().to_string(),
            output: format!("wrote {} bytes to {}", content.len(), final_path.display()),
            is_error: false,
        })
    }
}

pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Replace a string in an existing text file"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "find": {"type": "string"},
                "replace": {"type": "string"}
            },
            "required": ["path", "find", "replace"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("edit.path is required"))?;
        let find = args
            .get("find")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("edit.find is required"))?;
        let replace = args
            .get("replace")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("edit.replace is required"))?;

        let resolved = resolve_existing_path_unrestricted(&context.root_dir, path)?;
        let content = fs::read_to_string(&resolved).await?;
        if !content.contains(find) {
            return Err(anyhow!("target string not found in {}", resolved.display()));
        }

        let updated = content.replace(find, replace);
        fs::write(&resolved, updated).await?;
        let final_path = std::fs::canonicalize(&resolved).unwrap_or(resolved.clone());

        Ok(ToolExecution {
            name: self.name().to_string(),
            output: format!("updated {}", final_path.display()),
            is_error: false,
        })
    }
}

pub struct BashTool;

impl BashTool {
    fn allowed(command: &str) -> bool {
        let Some(parts) = shlex::split(command) else {
            return false;
        };
        let Some(first) = parts.first() else {
            return false;
        };
        matches!(
            first.as_str(),
            "ls" | "pwd" | "cat" | "echo" | "head" | "tail" | "rg" | "wc" | "date"
        )
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Run a safe, allowlisted shell command in the working directory"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string"}
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let command = args
            .get("command")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("bash.command is required"))?;

        if !Self::allowed(command) {
            return Err(anyhow!("command is not allowlisted"));
        }

        let output = Command::new("bash")
            .arg("-lc")
            .arg(command)
            .current_dir(&context.root_dir)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}{}", stdout, stderr);

        Ok(ToolExecution {
            name: self.name().to_string(),
            output: combined.chars().take(8_000).collect(),
            is_error: !output.status.success(),
        })
    }
}

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search for a text pattern inside files"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "path": {"type": "string"}
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let pattern = args
            .get("pattern")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("grep.pattern is required"))?
            .to_lowercase();
        let target_path = args
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or(".");

        let root = resolve_existing_path(&context.root_dir, target_path)?;
        let mut matches = Vec::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let content = match fs::read_to_string(path).await {
                Ok(value) => value,
                Err(_) => continue,
            };
            for (idx, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&pattern) {
                    matches.push(format!("{}:{}:{}", path.display(), idx + 1, line.trim()));
                }
            }
        }

        Ok(ToolExecution {
            name: self.name().to_string(),
            output: matches.into_iter().take(200).collect::<Vec<_>>().join("\n"),
            is_error: false,
        })
    }
}

pub struct FindTool;

#[async_trait]
impl Tool for FindTool {
    fn name(&self) -> &'static str {
        "find"
    }

    fn description(&self) -> &'static str {
        "Find files by path pattern"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "path": {"type": "string"}
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let pattern = args
            .get("pattern")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("find.pattern is required"))?
            .to_lowercase();
        let target_path = args
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or(".");

        let root = resolve_existing_path(&context.root_dir, target_path)?;
        let mut files = Vec::new();

        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();
            if path.to_string_lossy().to_lowercase().contains(&pattern) {
                files.push(path.display().to_string());
            }
        }

        Ok(ToolExecution {
            name: self.name().to_string(),
            output: files.into_iter().take(500).collect::<Vec<_>>().join("\n"),
            is_error: false,
        })
    }
}

pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "List files and directories"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"}
            }
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let target_path = args
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or(".");

        let root = resolve_existing_path(&context.root_dir, target_path)?;
        let mut items = fs::read_dir(root).await?;
        let mut lines = Vec::new();

        while let Some(entry) = items.next_entry().await? {
            let file_type = entry.file_type().await?;
            let marker = if file_type.is_dir() { "/" } else { "" };
            lines.push(format!("{}{}", entry.file_name().to_string_lossy(), marker));
        }

        lines.sort();
        Ok(ToolExecution {
            name: self.name().to_string(),
            output: lines.join("\n"),
            is_error: false,
        })
    }
}

pub struct MemoryGetTool;

#[async_trait]
impl Tool for MemoryGetTool {
    fn name(&self) -> &'static str {
        "memory_get"
    }

    fn description(&self) -> &'static str {
        "Read MEMORY.md or a memory log file"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "start_line": {"type": "integer", "minimum": 1},
                "end_line": {"type": "integer", "minimum": 1}
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let path = args
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("memory_get.path is required"))?;
        let start_line = args
            .get("start_line")
            .and_then(|value| value.as_u64())
            .map(|v| v as usize);
        let end_line = args
            .get("end_line")
            .and_then(|value| value.as_u64())
            .map(|v| v as usize);

        let content = context.memory.get_file(path, start_line, end_line).await?;

        Ok(ToolExecution {
            name: self.name().to_string(),
            output: content,
            is_error: false,
        })
    }
}

pub struct MemorySearchTool;

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &'static str {
        "memory_search"
    }

    fn description(&self) -> &'static str {
        "Search keyword over MEMORY.md and memory/*.md"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "keyword": {"type": "string"}
            },
            "required": ["keyword"]
        })
    }

    async fn execute(&self, args: Value, context: &ToolContext) -> Result<ToolExecution> {
        let keyword = args
            .get("keyword")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("memory_search.keyword is required"))?;

        let hits = context.memory.search(keyword).await?;
        let output = serde_json::to_string_pretty(&hits)?;

        Ok(ToolExecution {
            name: self.name().to_string(),
            output,
            is_error: false,
        })
    }
}
