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

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let cwd = env::current_dir()?;
        let host = env::var("CHAOS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("CHAOS_PORT")
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(3000);
        let provider = env::var("CHAOS_PROVIDER").unwrap_or_else(|_| "openai".to_string());
        let model = env::var("CHAOS_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let temperature = env::var("CHAOS_TEMPERATURE")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(0.2);
        let max_tokens = env::var("CHAOS_MAX_TOKENS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(1024);
        let max_iterations = env::var("CHAOS_MAX_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(6);
        let token_budget = env::var("CHAOS_TOKEN_BUDGET")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(12_000);

        let working_dir = env::var("CHAOS_WORKING_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.clone());
        let personality_dir = env::var("CHAOS_PERSONALITY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.join("personality"));
        let memory_dir = env::var("CHAOS_MEMORY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.join("memory"));
        let memory_file = env::var("CHAOS_MEMORY_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.join("MEMORY.md"));

        Ok(Self {
            host,
            port,
            provider,
            model,
            openai_api_key,
            temperature,
            max_tokens,
            max_iterations,
            token_budget,
            working_dir,
            personality_dir,
            memory_dir,
            memory_file,
        })
    }
}
