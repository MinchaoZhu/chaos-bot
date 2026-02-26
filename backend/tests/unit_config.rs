use chaos_bot_backend::infrastructure::config::{
    default_config_path_for_workspace, default_workspace_path, AgentChannelsConfig,
    AgentFileConfig, AgentLlmConfig, AgentLoggingConfig, AgentSecretsConfig, AgentServerConfig,
    AgentTelegramConfig, AppConfig, EnvSecrets,
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
        "CHAOS_HOST",
        "CHAOS_PORT",
        "CHAOS_PROVIDER",
        "CHAOS_MODEL",
        "CHAOS_TEMPERATURE",
        "CHAOS_MAX_TOKENS",
        "CHAOS_MAX_ITERATIONS",
        "CHAOS_TOKEN_BUDGET",
        "CHAOS_WORKING_DIR",
        "CHAOS_PERSONALITY_DIR",
        "CHAOS_MEMORY_DIR",
        "CHAOS_MEMORY_FILE",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "TELEGRAM_BOT_TOKEN",
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
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
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
        r#"{ "server": { "port": 4100 }, "llm": { "provider": "mock" } }"#,
    )
    .unwrap();
    std::fs::write(
        workspace.join("agent.json"),
        r#"{ "server": { "port": 4200 }, "llm": { "provider": "openai" } }"#,
    )
    .unwrap();

    let config = AppConfig::load().expect("load config");

    assert_eq!(config.config_path, workspace.join("config.json"));
    assert_eq!(config.port, 4100);
    assert_eq!(config.provider, "mock");
}

#[test]
#[serial]
fn load_falls_back_to_legacy_agent_json_when_present() {
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
        workspace.join("agent.json"),
        r#"{ "server": { "port": 4300 }, "llm": { "provider": "mock" } }"#,
    )
    .unwrap();

    let config = AppConfig::load().expect("load config");

    assert_eq!(config.config_path, workspace.join("agent.json"));
    assert_eq!(config.port, 4300);
    assert_eq!(config.provider, "mock");
}

#[test]
#[serial]
fn load_ignores_agent_config_path_env() {
    clear_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let _cwd_guard = CurrentDirGuard::enter(&cwd);

    let external_dir = temp.path().join("external");
    std::fs::create_dir_all(&external_dir).unwrap();
    let external_config = external_dir.join("agent.custom.json");
    std::fs::write(
        &external_config,
        r#"{ "server": { "port": 9999 }, "llm": { "provider": "mock" } }"#,
    )
    .unwrap();
    let _guard = EnvVarGuard::set("AGENT_CONFIG_PATH", external_config.to_str().unwrap());

    let config = AppConfig::load().expect("load config");

    let workspace = default_workspace_path(&home);
    assert_eq!(config.config_path, workspace.join("config.json"));
    assert_eq!(config.port, 3000);
    assert_eq!(config.provider, "openai");
}

#[test]
#[serial]
fn legacy_chaos_env_vars_are_ignored() {
    clear_envs();
    std::env::set_var("CHAOS_HOST", "127.0.0.1");
    std::env::set_var("CHAOS_PORT", "9999");
    std::env::set_var("CHAOS_PROVIDER", "mock");

    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let _cwd_guard = CurrentDirGuard::enter(&cwd);

    let config = AppConfig::load().expect("load config");

    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.provider, "openai");

    std::env::remove_var("CHAOS_HOST");
    std::env::remove_var("CHAOS_PORT");
    std::env::remove_var("CHAOS_PROVIDER");
}

#[test]
#[serial]
fn config_file_secrets_override_env_secrets() {
    clear_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

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
}
"#,
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

    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("GEMINI_API_KEY");
}

#[test]
#[serial]
fn env_secrets_are_used_when_config_secrets_missing() {
    clear_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let config_path = cwd.join("config.json");
    std::fs::write(
        &config_path,
        r#"{
  "llm": { "provider": "openai", "model": "gpt-4o-mini" },
  "secrets": {}
}
"#,
    )
    .unwrap();

    std::env::set_var("OPENAI_API_KEY", "openai-env");
    std::env::set_var("ANTHROPIC_API_KEY", "anthropic-env");
    std::env::set_var("GEMINI_API_KEY", "gemini-env");

    let config =
        AppConfig::from_config_file_path(&config_path, EnvSecrets::from_env(), cwd.clone())
            .expect("load")
            .0;

    assert_eq!(config.openai_api_key.as_deref(), Some("openai-env"));
    assert_eq!(config.anthropic_api_key.as_deref(), Some("anthropic-env"));
    assert_eq!(config.gemini_api_key.as_deref(), Some("gemini-env"));

    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("GEMINI_API_KEY");
}

#[test]
#[serial]
fn from_inputs_supports_injected_config_source() {
    let home = std::path::PathBuf::from("/tmp/home-base");
    let file_config = AgentFileConfig {
        workspace: Some(std::path::PathBuf::from("./wd")),
        logging: AgentLoggingConfig {
            level: Some("debug".to_string()),
            retention_days: Some(3),
            directory: Some(std::path::PathBuf::from("./my-logs")),
        },
        server: AgentServerConfig {
            host: Some("localhost".to_string()),
            port: Some(4444),
        },
        llm: AgentLlmConfig {
            provider: Some("mock".to_string()),
            model: Some("m".to_string()),
            temperature: Some(0.1),
            max_tokens: Some(256),
            max_iterations: Some(2),
            token_budget: Some(4096),
        },
        channels: AgentChannelsConfig::default(),
        secrets: AgentSecretsConfig {
            openai_api_key: Some("json-key".to_string()),
            anthropic_api_key: None,
            gemini_api_key: None,
            telegram_bot_token: Some("telegram-json".to_string()),
        },
    };
    let env_secrets = EnvSecrets {
        openai_api_key: Some("env-key".to_string()),
        anthropic_api_key: None,
        gemini_api_key: None,
        telegram_bot_token: Some("telegram-env".to_string()),
    };

    let config = AppConfig::from_inputs(file_config, env_secrets, home.clone());

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 4444);
    assert_eq!(config.provider, "mock");
    assert_eq!(config.model, "m");
    assert_eq!(config.openai_api_key.as_deref(), Some("json-key"));
    assert_eq!(config.telegram_bot_token.as_deref(), Some("telegram-json"));
    assert_eq!(config.workspace, home.join("wd"));
    assert_eq!(config.log_level, "debug");
    assert_eq!(config.log_retention_days, 3);
    assert_eq!(config.log_dir, home.join("wd/my-logs"));
    assert_eq!(config.working_dir, home.join("wd"));
    assert_eq!(config.personality_dir, home.join("wd/personality"));
    assert_eq!(config.memory_dir, home.join("wd/memory"));
    assert_eq!(config.memory_file, home.join("wd/MEMORY.md"));
}

#[test]
#[serial]
fn from_inputs_applies_telegram_channel_and_secret_settings() {
    let home = std::path::PathBuf::from("/tmp/home-base-telegram");
    let file_config = AgentFileConfig {
        workspace: Some(std::path::PathBuf::from("./wd")),
        logging: AgentLoggingConfig::default(),
        server: AgentServerConfig::default(),
        llm: AgentLlmConfig::default(),
        channels: AgentChannelsConfig {
            telegram: AgentTelegramConfig {
                enabled: Some(true),
                webhook_secret: Some("secret-123".to_string()),
                webhook_base_url: Some("https://example.test/hook".to_string()),
                polling: Some(false),
                api_base_url: Some("https://telegram.example".to_string()),
            },
        },
        secrets: AgentSecretsConfig {
            openai_api_key: None,
            anthropic_api_key: None,
            gemini_api_key: None,
            telegram_bot_token: Some("bot-token-json".to_string()),
        },
    };
    let env_secrets = EnvSecrets {
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        telegram_bot_token: Some("bot-token-env".to_string()),
    };

    let config = AppConfig::from_inputs(file_config, env_secrets, home);

    assert!(config.telegram_enabled);
    assert_eq!(config.telegram_webhook_secret.as_deref(), Some("secret-123"));
    assert_eq!(
        config.telegram_webhook_base_url.as_deref(),
        Some("https://example.test/hook")
    );
    assert_eq!(
        config.telegram_api_base_url,
        "https://telegram.example".to_string()
    );
    assert_eq!(config.telegram_bot_token.as_deref(), Some("bot-token-json"));
}
