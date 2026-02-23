use chaos_bot_backend::config::AppConfig;
use serial_test::serial;

#[test]
#[serial]
fn defaults_when_no_env_vars() {
    // Clear all CHAOS_ vars
    for key in &[
        "CHAOS_HOST", "CHAOS_PORT", "CHAOS_PROVIDER", "CHAOS_MODEL",
        "CHAOS_TEMPERATURE", "CHAOS_MAX_TOKENS", "CHAOS_MAX_ITERATIONS",
        "CHAOS_TOKEN_BUDGET", "CHAOS_WORKING_DIR", "CHAOS_PERSONALITY_DIR",
        "CHAOS_MEMORY_DIR", "CHAOS_MEMORY_FILE", "OPENAI_API_KEY",
    ] {
        std::env::remove_var(key);
    }

    let config = AppConfig::from_env().unwrap();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
    assert_eq!(config.provider, "openai");
    assert_eq!(config.model, "gpt-4o-mini");
    assert_eq!(config.temperature, 0.2);
    assert_eq!(config.max_tokens, 1024);
    assert_eq!(config.max_iterations, 6);
    assert_eq!(config.token_budget, 12_000);
    assert!(config.openai_api_key.is_none());
}

#[test]
#[serial]
fn env_var_overrides() {
    std::env::set_var("CHAOS_HOST", "127.0.0.1");
    std::env::set_var("CHAOS_PORT", "8080");
    std::env::set_var("CHAOS_PROVIDER", "anthropic");
    std::env::set_var("CHAOS_MODEL", "claude-3");
    std::env::set_var("CHAOS_TEMPERATURE", "0.7");
    std::env::set_var("CHAOS_MAX_TOKENS", "2048");
    std::env::set_var("CHAOS_MAX_ITERATIONS", "10");
    std::env::set_var("CHAOS_TOKEN_BUDGET", "20000");
    std::env::set_var("OPENAI_API_KEY", "sk-test");

    let config = AppConfig::from_env().unwrap();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
    assert_eq!(config.provider, "anthropic");
    assert_eq!(config.model, "claude-3");
    assert_eq!(config.temperature, 0.7);
    assert_eq!(config.max_tokens, 2048);
    assert_eq!(config.max_iterations, 10);
    assert_eq!(config.token_budget, 20_000);
    assert_eq!(config.openai_api_key.as_deref(), Some("sk-test"));

    // Cleanup
    for key in &[
        "CHAOS_HOST", "CHAOS_PORT", "CHAOS_PROVIDER", "CHAOS_MODEL",
        "CHAOS_TEMPERATURE", "CHAOS_MAX_TOKENS", "CHAOS_MAX_ITERATIONS",
        "CHAOS_TOKEN_BUDGET", "OPENAI_API_KEY",
    ] {
        std::env::remove_var(key);
    }
}

#[test]
#[serial]
fn path_overrides() {
    std::env::set_var("CHAOS_WORKING_DIR", "/tmp/work");
    std::env::set_var("CHAOS_PERSONALITY_DIR", "/tmp/personality");
    std::env::set_var("CHAOS_MEMORY_DIR", "/tmp/memory");
    std::env::set_var("CHAOS_MEMORY_FILE", "/tmp/MEM.md");

    let config = AppConfig::from_env().unwrap();

    assert_eq!(config.working_dir.to_string_lossy(), "/tmp/work");
    assert_eq!(config.personality_dir.to_string_lossy(), "/tmp/personality");
    assert_eq!(config.memory_dir.to_string_lossy(), "/tmp/memory");
    assert_eq!(config.memory_file.to_string_lossy(), "/tmp/MEM.md");

    for key in &[
        "CHAOS_WORKING_DIR", "CHAOS_PERSONALITY_DIR",
        "CHAOS_MEMORY_DIR", "CHAOS_MEMORY_FILE",
    ] {
        std::env::remove_var(key);
    }
}

#[test]
#[serial]
fn invalid_port_uses_default() {
    std::env::set_var("CHAOS_PORT", "not_a_number");
    let config = AppConfig::from_env().unwrap();
    assert_eq!(config.port, 3000);
    std::env::remove_var("CHAOS_PORT");
}

#[test]
#[serial]
fn invalid_temperature_uses_default() {
    std::env::set_var("CHAOS_TEMPERATURE", "abc");
    let config = AppConfig::from_env().unwrap();
    assert_eq!(config.temperature, 0.2);
    std::env::remove_var("CHAOS_TEMPERATURE");
}
