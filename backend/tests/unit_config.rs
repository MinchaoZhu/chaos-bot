use chaos_bot_backend::infrastructure::config::{
    default_config_path_for_workspace, default_workspace_path, AgentFileConfig, AgentLlmConfig,
    AgentLoggingConfig, AgentSearchConfig, AgentSecretsConfig, AppConfig, EnvSecrets,
};
use serial_test::serial;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(value) = &self.original {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn enter(path: &Path) -> Self {
        let original = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(path).expect("set current dir");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).expect("restore current dir");
    }
}

fn clear_envs() {
    for key in &[
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "AGENT_CONFIG_PATH",
    ] {
        std::env::remove_var(key);
    }
}

fn setup_home(root: &Path) -> EnvVarGuard {
    std::fs::create_dir_all(root).unwrap();
    EnvVarGuard::set("HOME", root.to_str().unwrap())
}

#[test]
#[serial]
fn load_creates_default_config_json_when_missing() {
    clear_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let _cwd_guard = CurrentDirGuard::enter(&cwd);

    let config = AppConfig::load().expect("load config");
    let workspace = default_workspace_path(&home);
    let config_path = default_config_path_for_workspace(&workspace);

    assert_eq!(config.workspace, workspace);
    assert_eq!(config.config_path, config_path);
    assert!(config.config_path.exists());
    assert!(workspace.join(".env.example").exists());
    assert_eq!(config.provider, "openai");
    assert_eq!(config.sessions_dir, workspace.join("sessions"));
}

#[test]
#[serial]
fn load_prefers_existing_config_json_over_legacy_agent_json() {
    clear_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let _cwd_guard = CurrentDirGuard::enter(&cwd);

    let workspace = default_workspace_path(&home);
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::write(
        workspace.join("config.json"),
        r#"{ "llm": { "provider": "mock" } }"#,
    )
    .unwrap();
    std::fs::write(
        workspace.join("agent.json"),
        r#"{ "llm": { "provider": "openai" } }"#,
    )
    .unwrap();

    let config = AppConfig::load().expect("load config");
    assert_eq!(config.config_path, workspace.join("config.json"));
    assert_eq!(config.provider, "mock");
}

#[test]
#[serial]
fn config_file_secrets_override_env_secrets() {
    clear_envs();
    let temp = tempdir().unwrap();
    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let config_path = cwd.join("config.json");
    std::fs::write(
        &config_path,
        r#"{
  "secrets": {
    "openai_api_key": "openai-json",
    "anthropic_api_key": "anthropic-json",
    "gemini_api_key": "gemini-json"
  }
}"#,
    )
    .unwrap();

    std::env::set_var("OPENAI_API_KEY", "openai-env");
    std::env::set_var("ANTHROPIC_API_KEY", "anthropic-env");
    std::env::set_var("GEMINI_API_KEY", "gemini-env");

    let config =
        AppConfig::from_config_file_path(&config_path, EnvSecrets::from_env(), cwd.clone())
            .expect("load")
            .0;

    assert_eq!(config.openai_api_key.as_deref(), Some("openai-json"));
    assert_eq!(config.anthropic_api_key.as_deref(), Some("anthropic-json"));
    assert_eq!(config.gemini_api_key.as_deref(), Some("gemini-json"));
}

#[test]
fn from_inputs_applies_workspace_logging_and_search_settings() {
    let home = PathBuf::from("/tmp/home-base");
    let file_config = AgentFileConfig {
        workspace: Some(PathBuf::from("./wd")),
        logging: AgentLoggingConfig {
            level: Some("debug".to_string()),
            retention_days: Some(3),
            directory: Some(PathBuf::from("./my-logs")),
        },
        llm: AgentLlmConfig {
            provider: Some("mock".to_string()),
            model: Some("m".to_string()),
            temperature: Some(0.1),
            max_tokens: Some(256),
            max_iterations: Some(2),
            token_budget: Some(4096),
        },
        search: AgentSearchConfig {
            provider: Some("Perplexity".to_string()),
        },
        secrets: AgentSecretsConfig {
            openai_api_key: Some("json-key".to_string()),
            anthropic_api_key: None,
            gemini_api_key: None,
            perplexity_api_key: Some("pplx-json".to_string()),
            tavily_api_key: Some("tvly-json".to_string()),
            brave_search_api_key: Some("brave-json".to_string()),
        },
    };

    let config = AppConfig::from_inputs(file_config, EnvSecrets::default(), home.clone());

    assert_eq!(config.provider, "mock");
    assert_eq!(config.model, "m");
    assert_eq!(config.search_provider.as_deref(), Some("perplexity"));
    assert_eq!(config.openai_api_key.as_deref(), Some("json-key"));
    assert_eq!(config.perplexity_api_key.as_deref(), Some("pplx-json"));
    assert_eq!(config.tavily_api_key.as_deref(), Some("tvly-json"));
    assert_eq!(config.brave_search_api_key.as_deref(), Some("brave-json"));
    assert_eq!(config.workspace, home.join("wd"));
    assert_eq!(config.log_level, "debug");
    assert_eq!(config.log_retention_days, 3);
    assert_eq!(config.log_dir, home.join("wd/my-logs"));
    assert_eq!(config.sessions_dir, home.join("wd/sessions"));
}
