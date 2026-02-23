use anyhow::Result;
use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub provider: String,
    pub model: String,
    pub openai_api_key: Option<String>,
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
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let defaults = Self::default();

        Ok(Self {
            host: env::var("CHAOS_HOST").unwrap_or(defaults.host),
            port: env::var("CHAOS_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(defaults.port),
            provider: env::var("CHAOS_PROVIDER").unwrap_or(defaults.provider),
            model: env::var("CHAOS_MODEL").unwrap_or(defaults.model),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            temperature: env::var("CHAOS_TEMPERATURE")
                .ok()
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(defaults.temperature),
            max_tokens: env::var("CHAOS_MAX_TOKENS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(defaults.max_tokens),
            max_iterations: env::var("CHAOS_MAX_ITERATIONS")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(defaults.max_iterations),
            token_budget: env::var("CHAOS_TOKEN_BUDGET")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(defaults.token_budget),
            working_dir: env::var("CHAOS_WORKING_DIR")
                .map(PathBuf::from)
                .unwrap_or(defaults.working_dir),
            personality_dir: env::var("CHAOS_PERSONALITY_DIR")
                .map(PathBuf::from)
                .unwrap_or(defaults.personality_dir),
            memory_dir: env::var("CHAOS_MEMORY_DIR")
                .map(PathBuf::from)
                .unwrap_or(defaults.memory_dir),
            memory_file: env::var("CHAOS_MEMORY_FILE")
                .map(PathBuf::from)
                .unwrap_or(defaults.memory_file),
        })
    }
}
