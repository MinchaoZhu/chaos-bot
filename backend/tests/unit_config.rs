use chaos_bot_backend::config::{
    AgentFileConfig, AgentLlmConfig, AgentLoggingConfig, AgentSecretsConfig, AgentServerConfig,
    AppConfig, EnvSecrets,
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

fn clear_legacy_and_secret_envs() {
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
fn creates_default_agent_json_when_missing() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let agent_path = cwd.join("agent.json");
    let env_example_path = cwd.join(".env.example");

    assert!(!agent_path.exists());
    assert!(!env_example_path.exists());

    let config =
        AppConfig::from_agent_file_path(&agent_path, EnvSecrets::default(), cwd.clone()).unwrap();

    assert!(agent_path.exists());
    assert!(env_example_path.exists());
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.provider, "openai");
    assert_eq!(config.model, "gpt-4o-mini");
    assert_eq!(config.temperature, 0.2);
    assert_eq!(config.max_tokens, 1024);
    assert_eq!(config.max_iterations, 6);
    assert_eq!(config.token_budget, 12_000);
    assert!(config.openai_api_key.is_none());
    assert!(config.anthropic_api_key.is_none());
    assert!(config.gemini_api_key.is_none());
    assert_eq!(config.workspace, home.join(".chaos-bot"));
    assert_eq!(config.log_level, "info");
    assert_eq!(config.log_retention_days, 7);
    assert_eq!(config.log_dir, home.join(".chaos-bot/logs"));
    assert_eq!(config.working_dir, home.join(".chaos-bot"));
    assert_eq!(config.personality_dir, home.join(".chaos-bot/personality"));
    assert_eq!(config.memory_dir, home.join(".chaos-bot/memory"));
    assert_eq!(config.memory_file, home.join(".chaos-bot/MEMORY.md"));
}

#[test]
#[serial]
fn agent_json_overrides_runtime_settings() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let agent_path = cwd.join("agent.json");
    let custom = AgentFileConfig {
        workspace: Some(PathBuf::from("./runtime")),
        logging: AgentLoggingConfig {
            level: Some("warning".to_string()),
            retention_days: Some(14),
            directory: Some(PathBuf::from("./app-logs")),
        },
        server: AgentServerConfig {
            host: Some("127.0.0.1".to_string()),
            port: Some(8080),
        },
        llm: AgentLlmConfig {
            provider: Some("mock".to_string()),
            model: Some("mock-model".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(2048),
            max_iterations: Some(10),
            token_budget: Some(20_000),
        },
        secrets: AgentSecretsConfig::default(),
    };
    std::fs::write(
        &agent_path,
        format!("{}\n", serde_json::to_string_pretty(&custom).unwrap()),
    )
    .unwrap();

    let config =
        AppConfig::from_agent_file_path(&agent_path, EnvSecrets::default(), cwd.clone()).unwrap();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
    assert_eq!(config.provider, "mock");
    assert_eq!(config.model, "mock-model");
    assert_eq!(config.temperature, 0.7);
    assert_eq!(config.max_tokens, 2048);
    assert_eq!(config.max_iterations, 10);
    assert_eq!(config.token_budget, 20_000);
    assert_eq!(config.workspace, home.join("runtime"));
    assert_eq!(config.log_level, "warn");
    assert_eq!(config.log_retention_days, 14);
    assert_eq!(config.log_dir, home.join("runtime/app-logs"));
    assert_eq!(config.working_dir, home.join("runtime"));
    assert_eq!(config.personality_dir, home.join("runtime/personality"));
    assert_eq!(config.memory_dir, home.join("runtime/memory"));
    assert_eq!(config.memory_file, home.join("runtime/MEMORY.md"));
}

#[test]
#[serial]
fn absolute_workspace_is_used_as_is() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let absolute_workspace = temp.path().join("absolute-workspace");
    let config = AppConfig::from_inputs(
        AgentFileConfig {
            workspace: Some(absolute_workspace.clone()),
            logging: AgentLoggingConfig::default(),
            server: AgentServerConfig::default(),
            llm: AgentLlmConfig::default(),
            secrets: AgentSecretsConfig::default(),
        },
        EnvSecrets::default(),
        home.clone(),
    );
    assert_eq!(config.workspace, absolute_workspace.clone());
    assert_eq!(config.working_dir, absolute_workspace);
}

#[test]
#[serial]
fn env_secrets_apply_when_json_secrets_missing() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let agent_path = cwd.join("agent.json");
    std::fs::write(
        &agent_path,
        r#"{
  "workspace": ".chaos-bot",
  "server": { "host": "0.0.0.0", "port": 3000 },
  "llm": { "provider": "openai", "model": "gpt-4o-mini" },
  "secrets": {}
}
"#,
    )
    .unwrap();

    let env_secrets = EnvSecrets {
        openai_api_key: Some("openai-env".to_string()),
        anthropic_api_key: Some("anthropic-env".to_string()),
        gemini_api_key: Some("gemini-env".to_string()),
    };
    let config = AppConfig::from_agent_file_path(&agent_path, env_secrets, cwd).unwrap();

    assert_eq!(config.openai_api_key.as_deref(), Some("openai-env"));
    assert_eq!(config.anthropic_api_key.as_deref(), Some("anthropic-env"));
    assert_eq!(config.gemini_api_key.as_deref(), Some("gemini-env"));
}

#[test]
#[serial]
fn json_secrets_override_env_secrets() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let agent_path = cwd.join("agent.json");
    std::fs::write(
        &agent_path,
        r#"{
  "llm": { "provider": "openai" },
  "secrets": {
    "openai_api_key": "openai-json",
    "anthropic_api_key": "anthropic-json",
    "gemini_api_key": "gemini-json"
  }
}
"#,
    )
    .unwrap();

    let env_secrets = EnvSecrets {
        openai_api_key: Some("openai-env".to_string()),
        anthropic_api_key: Some("anthropic-env".to_string()),
        gemini_api_key: Some("gemini-env".to_string()),
    };
    let config = AppConfig::from_agent_file_path(&agent_path, env_secrets, cwd).unwrap();

    assert_eq!(config.openai_api_key.as_deref(), Some("openai-json"));
    assert_eq!(config.anthropic_api_key.as_deref(), Some("anthropic-json"));
    assert_eq!(config.gemini_api_key.as_deref(), Some("gemini-json"));
}

#[test]
#[serial]
fn legacy_chaos_env_vars_are_ignored() {
    clear_legacy_and_secret_envs();
    std::env::set_var("CHAOS_HOST", "127.0.0.1");
    std::env::set_var("CHAOS_PORT", "9999");
    std::env::set_var("CHAOS_PROVIDER", "mock");

    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cfg");
    std::fs::create_dir_all(&cwd).unwrap();
    let config =
        AppConfig::from_agent_file_path(&cwd.join("agent.json"), EnvSecrets::default(), cwd)
            .unwrap();

    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.provider, "openai");
    assert_eq!(config.workspace, home.join(".chaos-bot"));

    std::env::remove_var("CHAOS_HOST");
    std::env::remove_var("CHAOS_PORT");
    std::env::remove_var("CHAOS_PROVIDER");
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
        secrets: AgentSecretsConfig {
            openai_api_key: Some("json-key".to_string()),
            anthropic_api_key: None,
            gemini_api_key: None,
        },
    };
    let env_secrets = EnvSecrets {
        openai_api_key: Some("env-key".to_string()),
        anthropic_api_key: None,
        gemini_api_key: None,
    };

    let config = AppConfig::from_inputs(file_config, env_secrets, home.clone());

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 4444);
    assert_eq!(config.provider, "mock");
    assert_eq!(config.model, "m");
    assert_eq!(config.openai_api_key.as_deref(), Some("json-key"));
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
fn load_uses_home_workspace_when_agent_config_path_is_external() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let home = temp.path().join("home");
    let _home_guard = setup_home(&home);

    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();
    let _cwd_guard = CurrentDirGuard::enter(&cwd);

    let external_dir = temp.path().join("external-config");
    std::fs::create_dir_all(&external_dir).unwrap();
    let external_config = external_dir.join("agent.custom.json");
    let _config_guard = EnvVarGuard::set("AGENT_CONFIG_PATH", external_config.to_str().unwrap());

    let config = AppConfig::load().expect("load config");

    assert!(external_config.exists());
    assert!(external_dir.join(".env.example").exists());
    assert_eq!(config.workspace, home.join(".chaos-bot"));
    assert_eq!(config.log_dir, home.join(".chaos-bot/logs"));
    assert_eq!(config.working_dir, home.join(".chaos-bot"));
}
