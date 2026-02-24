use chaos_bot_backend::bootstrap::bootstrap_runtime_dirs;
use chaos_bot_backend::config::AppConfig;
use std::path::PathBuf;
use tempfile::tempdir;

fn make_config(root: PathBuf, personality_dir: PathBuf) -> AppConfig {
    AppConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        provider: "mock".to_string(),
        model: "mock-model".to_string(),
        openai_api_key: None,
        anthropic_api_key: None,
        gemini_api_key: None,
        temperature: 0.2,
        max_tokens: 256,
        max_iterations: 3,
        token_budget: 2_000,
        working_dir: root.clone(),
        personality_dir,
        memory_dir: root.join("memory"),
        memory_file: root.join("MEMORY.md"),
    }
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
