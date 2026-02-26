use chaos_bot_backend::runtime::bootstrap::bootstrap_runtime_dirs;
use chaos_bot_backend::infrastructure::config::AppConfig;
use std::path::PathBuf;
use tempfile::tempdir;

fn make_config(root: PathBuf, personality_dir: PathBuf) -> AppConfig {
    let mut config = AppConfig::default();
    config.host = "127.0.0.1".to_string();
    config.port = 3000;
    config.provider = "mock".to_string();
    config.model = "mock-model".to_string();
    config.temperature = 0.2;
    config.max_tokens = 256;
    config.max_iterations = 3;
    config.token_budget = 2_000;
    config.workspace = root.clone();
    config.config_path = root.join("config.json");
    config.log_level = "info".to_string();
    config.log_retention_days = 7;
    config.log_dir = root.join("logs");
    config.working_dir = root.clone();
    config.personality_dir = personality_dir;
    config.memory_dir = root.join("memory");
    config.memory_file = root.join("MEMORY.md");
    config
}

#[tokio::test]
async fn bootstrap_creates_default_personality_and_sessions_dir() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let personality_dir = root.join("personality");
    let config = make_config(root.clone(), personality_dir.clone());

    bootstrap_runtime_dirs(&config).await.unwrap();

    for file in ["SOUL.md", "IDENTITY.md", "USER.md", "AGENTS.md"] {
        let path = personality_dir.join(file);
        assert!(path.exists(), "expected {} to exist", path.display());
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert!(!content.trim().is_empty());
    }

    assert!(root.join("data/sessions").exists());
}

#[tokio::test]
async fn bootstrap_preserves_existing_files_and_fills_missing_defaults() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let personality_dir = root.join("personality");
    tokio::fs::create_dir_all(&personality_dir).await.unwrap();
    let soul = personality_dir.join("SOUL.md");
    tokio::fs::write(&soul, "custom soul").await.unwrap();

    let config = make_config(root.clone(), personality_dir.clone());
    bootstrap_runtime_dirs(&config).await.unwrap();

    let soul_content = tokio::fs::read_to_string(soul).await.unwrap();
    assert_eq!(soul_content, "custom soul");

    assert!(personality_dir.join("IDENTITY.md").exists());
    assert!(personality_dir.join("USER.md").exists());
    assert!(personality_dir.join("AGENTS.md").exists());
    assert!(root.join("data/sessions").exists());
}
