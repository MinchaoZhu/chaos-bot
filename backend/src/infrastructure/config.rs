use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::infrastructure::runtime_assets::{DEFAULT_AGENT_JSON, DEFAULT_ENV_EXAMPLE};

const DEFAULT_WORKSPACE_DIR: &str = ".chaos-bot";
const DEFAULT_CONFIG_FILE_NAME: &str = "config.json";
const LEGACY_CONFIG_FILE_NAME: &str = "agent.json";

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub provider: String,
    pub model: String,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
    pub temperature: f32,
    pub max_tokens: u32,
    pub max_iterations: usize,
    pub token_budget: u32,
    pub workspace: PathBuf,
    pub config_path: PathBuf,
    pub log_level: String,
    pub log_retention_days: u16,
    pub log_dir: PathBuf,
    pub working_dir: PathBuf,
    pub personality_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub memory_file: PathBuf,
    pub skills_dir: PathBuf,
}

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub app: AppConfig,
    pub file: AgentFileConfig,
    pub raw: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let workspace_base = workspace_base_for(&cwd);
        let workspace = default_workspace_path(&workspace_base);
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            openai_api_key: None,
            anthropic_api_key: None,
            gemini_api_key: None,
            temperature: 0.2,
            max_tokens: 1024,
            max_iterations: 6,
            token_budget: 12_000,
            config_path: default_config_path_for_workspace(&workspace),
            log_level: "info".to_string(),
            log_retention_days: 7,
            log_dir: workspace.join("logs"),
            working_dir: workspace.clone(),
            personality_dir: workspace.join("personality"),
            memory_dir: workspace.join("memory"),
            memory_file: workspace.join("MEMORY.md"),
            skills_dir: workspace.join("skills"),
            workspace,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        Ok(Self::load_with_source()?.app)
    }

    pub fn load_with_source() -> Result<LoadedConfig> {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let workspace_base = workspace_base_for(&cwd);
        let workspace = default_workspace_path(&workspace_base);
        let config_path = select_default_config_path(&workspace);
        let (app, file, raw) =
            Self::from_config_file_path(&config_path, EnvSecrets::from_env(), cwd)?;
        Ok(LoadedConfig { app, file, raw })
    }

    pub fn from_agent_file_path(
        path: &Path,
        env_secrets: EnvSecrets,
        cwd: PathBuf,
    ) -> Result<Self> {
        Ok(Self::from_config_file_path(path, env_secrets, cwd)?.0)
    }

    pub fn from_config_file_path(
        path: &Path,
        env_secrets: EnvSecrets,
        cwd: PathBuf,
    ) -> Result<(Self, AgentFileConfig, String)> {
        let config_path = resolve_config_path(&cwd, path);
        ensure_runtime_config_files(&config_path)?;
        let (file_config, raw) = read_config_file(&config_path)?;
        let workspace_base = workspace_base_for(&cwd);
        let mut app = Self::from_inputs(file_config.clone(), env_secrets, workspace_base);
        app.config_path = config_path;
        Ok((app, file_config, raw))
    }

    pub fn from_inputs(
        file_config: AgentFileConfig,
        env_secrets: EnvSecrets,
        workspace_base: PathBuf,
    ) -> Self {
        let defaults = Self::defaults_for_workspace_base(workspace_base.clone());
        let mut config = defaults;
        // Priority: defaults < env secrets < config secrets
        config.openai_api_key = env_secrets.openai_api_key;
        config.anthropic_api_key = env_secrets.anthropic_api_key;
        config.gemini_api_key = env_secrets.gemini_api_key;

        if let Some(host) = file_config.server.host {
            config.host = host;
        }
        if let Some(port) = file_config.server.port {
            config.port = port;
        }

        if let Some(provider) = file_config.llm.provider {
            config.provider = provider;
        }
        if let Some(model) = file_config.llm.model {
            config.model = model;
        }
        if let Some(temperature) = file_config.llm.temperature {
            config.temperature = temperature;
        }
        if let Some(max_tokens) = file_config.llm.max_tokens {
            config.max_tokens = max_tokens;
        }
        if let Some(max_iterations) = file_config.llm.max_iterations {
            config.max_iterations = max_iterations;
        }
        if let Some(token_budget) = file_config.llm.token_budget {
            config.token_budget = token_budget;
        }

        if let Some(workspace) = file_config.workspace {
            config.workspace = resolve_workspace_path(&workspace_base, workspace);
            config.derive_runtime_paths_from_workspace();
        }

        if let Some(level) = file_config.logging.level {
            config.log_level = normalize_log_level(level);
        }
        if let Some(retention_days) = file_config.logging.retention_days {
            config.log_retention_days = retention_days.max(1);
        }
        if let Some(directory) = file_config.logging.directory {
            config.log_dir = resolve_log_dir(&config.workspace, directory);
        }

        if let Some(openai_api_key) = file_config.secrets.openai_api_key {
            config.openai_api_key = Some(openai_api_key);
        }
        if let Some(anthropic_api_key) = file_config.secrets.anthropic_api_key {
            config.anthropic_api_key = Some(anthropic_api_key);
        }
        if let Some(gemini_api_key) = file_config.secrets.gemini_api_key {
            config.gemini_api_key = Some(gemini_api_key);
        }

        config
    }

    fn defaults_for_workspace_base(workspace_base: PathBuf) -> Self {
        let workspace = default_workspace_path(&workspace_base);
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            openai_api_key: None,
            anthropic_api_key: None,
            gemini_api_key: None,
            temperature: 0.2,
            max_tokens: 1024,
            max_iterations: 6,
            token_budget: 12_000,
            config_path: default_config_path_for_workspace(&workspace),
            log_level: "info".to_string(),
            log_retention_days: 7,
            log_dir: workspace.join("logs"),
            working_dir: workspace.clone(),
            personality_dir: workspace.join("personality"),
            memory_dir: workspace.join("memory"),
            memory_file: workspace.join("MEMORY.md"),
            skills_dir: workspace.join("skills"),
            workspace,
        }
    }

    fn derive_runtime_paths_from_workspace(&mut self) {
        self.log_dir = self.workspace.join("logs");
        self.working_dir = self.workspace.clone();
        self.personality_dir = self.workspace.join("personality");
        self.memory_dir = self.workspace.join("memory");
        self.memory_file = self.workspace.join("MEMORY.md");
        self.skills_dir = self.workspace.join("skills");
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EnvSecrets {
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

impl EnvSecrets {
    pub fn from_env() -> Self {
        Self {
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            gemini_api_key: env::var("GEMINI_API_KEY").ok(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AgentFileConfig {
    pub workspace: Option<PathBuf>,
    pub logging: AgentLoggingConfig,
    pub server: AgentServerConfig,
    pub llm: AgentLlmConfig,
    pub secrets: AgentSecretsConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AgentServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AgentLlmConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub max_iterations: Option<usize>,
    pub token_budget: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AgentLoggingConfig {
    pub level: Option<String>,
    pub retention_days: Option<u16>,
    pub directory: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct AgentSecretsConfig {
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub gemini_api_key: Option<String>,
}

impl AgentFileConfig {
    pub fn default_template() -> Self {
        serde_json::from_str(DEFAULT_AGENT_JSON)
            .expect("embedded templates/config/agent.json must be valid AgentFileConfig")
    }
}

pub fn workspace_base_for(cwd: &Path) -> PathBuf {
    home_dir().unwrap_or_else(|| cwd.to_path_buf())
}

pub fn default_workspace_path(base: &Path) -> PathBuf {
    base.join(DEFAULT_WORKSPACE_DIR)
}

pub fn default_config_path_for_workspace(workspace: &Path) -> PathBuf {
    workspace.join(DEFAULT_CONFIG_FILE_NAME)
}

pub fn legacy_config_path_for_workspace(workspace: &Path) -> PathBuf {
    workspace.join(LEGACY_CONFIG_FILE_NAME)
}

pub fn select_default_config_path(workspace: &Path) -> PathBuf {
    let default = default_config_path_for_workspace(workspace);
    if default.exists() {
        return default;
    }
    let legacy = legacy_config_path_for_workspace(workspace);
    if legacy.exists() {
        legacy
    } else {
        default
    }
}

pub fn read_config_file(path: &Path) -> Result<(AgentFileConfig, String)> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let parsed = serde_json::from_str::<AgentFileConfig>(&raw)
        .with_context(|| format!("invalid config json: {}", path.display()))?;
    Ok((parsed, raw))
}

pub fn write_config_file(path: &Path, config: &AgentFileConfig) -> Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = format!("{}\n", serde_json::to_string_pretty(config)?);
    fs::write(path, &raw).with_context(|| format!("failed to write config: {}", path.display()))?;
    Ok(raw)
}

fn resolve_workspace_path(base: &Path, workspace: PathBuf) -> PathBuf {
    if workspace.is_absolute() {
        workspace
    } else {
        base.join(workspace)
    }
}

fn resolve_log_dir(workspace: &Path, directory: PathBuf) -> PathBuf {
    if directory.is_absolute() {
        directory
    } else {
        workspace.join(directory)
    }
}

fn normalize_log_level(level: String) -> String {
    match level.to_ascii_lowercase().as_str() {
        "debug" => "debug".to_string(),
        "info" => "info".to_string(),
        "warning" | "warn" => "warn".to_string(),
        "error" => "error".to_string(),
        _ => "info".to_string(),
    }
}

fn resolve_config_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn ensure_runtime_config_files(config_path: &Path) -> Result<()> {
    ensure_file_exists_with_default(config_path, DEFAULT_AGENT_JSON)?;
    let env_example_path = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".env.example");
    ensure_file_exists_with_default(&env_example_path, DEFAULT_ENV_EXAMPLE)?;
    Ok(())
}

fn ensure_file_exists_with_default(path: &Path, default_content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, ensure_trailing_newline(default_content))?;
    Ok(())
}

fn ensure_trailing_newline(content: &str) -> String {
    if content.ends_with('\n') {
        content.to_string()
    } else {
        format!("{content}\n")
    }
}
