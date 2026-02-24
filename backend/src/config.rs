use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::runtime_assets::{DEFAULT_AGENT_JSON, DEFAULT_ENV_EXAMPLE};

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
    pub working_dir: PathBuf,
    pub personality_dir: PathBuf,
    pub memory_dir: PathBuf,
    pub memory_file: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
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
            working_dir: cwd.clone(),
            personality_dir: cwd.join("personality"),
            memory_dir: cwd.join("memory"),
            memory_file: cwd.join("MEMORY.md"),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        dotenvy::dotenv().ok();
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let agent_file_path = env::var("AGENT_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.join("agent.json"));
        let resolved_agent_file_path = resolve_config_path(&cwd, &agent_file_path);
        let config_root = config_root_from_agent_path(&cwd, &resolved_agent_file_path);
        Self::from_agent_file_path(
            &resolved_agent_file_path,
            EnvSecrets::from_env(),
            config_root,
        )
    }

    pub fn from_agent_file_path(
        path: &Path,
        env_secrets: EnvSecrets,
        cwd: PathBuf,
    ) -> Result<Self> {
        let agent_path = resolve_config_path(&cwd, path);
        ensure_runtime_config_files(&agent_path)?;
        let content = fs::read_to_string(&agent_path)?;
        let file_config: AgentFileConfig = serde_json::from_str(&content)?;
        Ok(Self::from_inputs(file_config, env_secrets, cwd))
    }

    pub fn from_inputs(
        file_config: AgentFileConfig,
        env_secrets: EnvSecrets,
        cwd: PathBuf,
    ) -> Self {
        let defaults = Self::defaults_for_cwd(cwd.clone());
        let mut config = defaults.clone();

        // Priority: defaults < env secrets < agent.json secrets
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

        if let Some(path) = file_config.paths.working_dir {
            config.working_dir = resolve_path(&cwd, path);
        }
        if let Some(path) = file_config.paths.personality_dir {
            config.personality_dir = resolve_path(&cwd, path);
        }
        if let Some(path) = file_config.paths.memory_dir {
            config.memory_dir = resolve_path(&cwd, path);
        }
        if let Some(path) = file_config.paths.memory_file {
            config.memory_file = resolve_path(&cwd, path);
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

    fn defaults_for_cwd(cwd: PathBuf) -> Self {
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
            working_dir: cwd.clone(),
            personality_dir: cwd.join("personality"),
            memory_dir: cwd.join("memory"),
            memory_file: cwd.join("MEMORY.md"),
        }
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
    pub server: AgentServerConfig,
    pub llm: AgentLlmConfig,
    pub paths: AgentPathsConfig,
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
pub struct AgentPathsConfig {
    pub working_dir: Option<PathBuf>,
    pub personality_dir: Option<PathBuf>,
    pub memory_dir: Option<PathBuf>,
    pub memory_file: Option<PathBuf>,
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

fn resolve_path(cwd: &Path, path: PathBuf) -> PathBuf {
    if path.is_relative() {
        cwd.join(path)
    } else {
        path
    }
}

fn resolve_config_path(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn config_root_from_agent_path(cwd: &Path, path: &Path) -> PathBuf {
    let resolved = resolve_config_path(cwd, path);
    resolved
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| cwd.to_path_buf())
}

fn ensure_runtime_config_files(agent_path: &Path) -> Result<()> {
    ensure_file_exists_with_default(agent_path, DEFAULT_AGENT_JSON)?;
    let env_example_path = agent_path
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
