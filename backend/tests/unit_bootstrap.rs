use chaos_bot_backend::infrastructure::config::AppConfig;
use chaos_bot_backend::runtime::bootstrap::bootstrap_runtime_dirs;
use tempfile::tempdir;

fn make_config(root: &std::path::Path) -> AppConfig {
    AppConfig {
        provider: "mock".to_string(),
        model: "mock-model".to_string(),
        workspace: root.to_path_buf(),
        config_path: root.join("config.json"),
        log_level: "info".to_string(),
        log_retention_days: 7,
        log_dir: root.join("logs"),
        working_dir: root.to_path_buf(),
        personality_dir: root.join("personality"),
        memory_dir: root.join("memory"),
        memory_file: root.join("MEMORY.md"),
        sessions_dir: root.join("sessions"),
        skills_dir: root.join("skills"),
        ..AppConfig::default()
    }
}

#[tokio::test]
async fn bootstrap_creates_default_personality_and_sessions_dir() {
    let tmp = tempdir().unwrap();
    let config = make_config(tmp.path());

    bootstrap_runtime_dirs(&config).await.unwrap();

    for file in ["SOUL.md", "IDENTITY.md", "USER.md", "AGENTS.md"] {
        let path = config.personality_dir.join(file);
        assert!(path.exists(), "expected {} to exist", path.display());
        let content = tokio::fs::read_to_string(path).await.unwrap();
        assert!(!content.trim().is_empty());
    }

    assert!(config.sessions_dir.exists());
}

#[tokio::test]
async fn bootstrap_preserves_existing_files_and_fills_missing_defaults() {
    let tmp = tempdir().unwrap();
    let config = make_config(tmp.path());
    tokio::fs::create_dir_all(&config.personality_dir)
        .await
        .unwrap();
    let soul = config.personality_dir.join("SOUL.md");
    tokio::fs::write(&soul, "custom soul").await.unwrap();

    bootstrap_runtime_dirs(&config).await.unwrap();

    let soul_content = tokio::fs::read_to_string(soul).await.unwrap();
    assert_eq!(soul_content, "custom soul");
    assert!(config.personality_dir.join("IDENTITY.md").exists());
    assert!(config.personality_dir.join("USER.md").exists());
    assert!(config.personality_dir.join("AGENTS.md").exists());
    assert!(config.sessions_dir.exists());
}
