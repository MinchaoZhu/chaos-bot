use chaos_bot_backend::config::{
    AgentFileConfig, AgentLlmConfig, AgentPathsConfig, AgentSecretsConfig, AgentServerConfig,
    AppConfig, EnvSecrets,
};
use serial_test::serial;
use tempfile::tempdir;

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
    ] {
        std::env::remove_var(key);
    }
}

#[test]
#[serial]
fn creates_default_agent_json_when_missing() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let cwd = temp.path().to_path_buf();
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
    assert_eq!(config.working_dir, cwd);
}

#[test]
#[serial]
fn agent_json_overrides_runtime_settings() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let cwd = temp.path().to_path_buf();
    let agent_path = cwd.join("agent.json");
    let custom = AgentFileConfig {
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
        paths: AgentPathsConfig {
            working_dir: Some(std::path::PathBuf::from("./runtime")),
            personality_dir: Some(std::path::PathBuf::from("./personality-x")),
            memory_dir: Some(std::path::PathBuf::from("./memory-x")),
            memory_file: Some(std::path::PathBuf::from("./MEMX.md")),
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
    assert_eq!(config.working_dir, cwd.join("runtime"));
    assert_eq!(config.personality_dir, cwd.join("personality-x"));
    assert_eq!(config.memory_dir, cwd.join("memory-x"));
    assert_eq!(config.memory_file, cwd.join("MEMX.md"));
}

#[test]
#[serial]
fn env_secrets_apply_when_json_secrets_missing() {
    clear_legacy_and_secret_envs();
    let temp = tempdir().unwrap();
    let cwd = temp.path().to_path_buf();
    let agent_path = cwd.join("agent.json");
    std::fs::write(
        &agent_path,
        r#"{
  "server": { "host": "0.0.0.0", "port": 3000 },
  "llm": { "provider": "openai", "model": "gpt-4o-mini" },
  "paths": { "working_dir": ".", "personality_dir": "./personality", "memory_dir": "./memory", "memory_file": "./MEMORY.md" },
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
    let cwd = temp.path().to_path_buf();
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
    let cwd = temp.path().to_path_buf();
    let config =
        AppConfig::from_agent_file_path(&cwd.join("agent.json"), EnvSecrets::default(), cwd)
            .unwrap();

    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.provider, "openai");

    std::env::remove_var("CHAOS_HOST");
    std::env::remove_var("CHAOS_PORT");
    std::env::remove_var("CHAOS_PROVIDER");
}

#[test]
fn from_inputs_supports_injected_config_source() {
    let cwd = std::path::PathBuf::from("/tmp/project-root");
    let file_config = AgentFileConfig {
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
        paths: AgentPathsConfig {
            working_dir: Some(std::path::PathBuf::from("./wd")),
            personality_dir: Some(std::path::PathBuf::from("./p")),
            memory_dir: Some(std::path::PathBuf::from("./m")),
            memory_file: Some(std::path::PathBuf::from("./M.md")),
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

    let config = AppConfig::from_inputs(file_config, env_secrets, cwd.clone());

    assert_eq!(config.host, "localhost");
    assert_eq!(config.port, 4444);
    assert_eq!(config.provider, "mock");
    assert_eq!(config.model, "m");
    assert_eq!(config.openai_api_key.as_deref(), Some("json-key"));
    assert_eq!(config.working_dir, cwd.join("wd"));
}
