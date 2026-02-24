use chaos_bot_backend::config::{
    AgentFileConfig, AgentLlmConfig, AgentLoggingConfig, AgentSecretsConfig, AgentServerConfig,
    AppConfig, EnvSecrets,
};
use chaos_bot_backend::logging::{cleanup_old_logs_at, init_logging};
use chrono::NaiveDate;
use tempfile::tempdir;

#[test]
fn cleanup_old_logs_removes_entries_outside_retention() {
    let temp = tempdir().expect("tempdir");
    let logs = temp.path().join("logs");
    std::fs::create_dir_all(&logs).expect("create logs dir");
    std::fs::write(logs.join("2026-02-10.log"), "old").expect("write old");
    std::fs::write(logs.join("2026-02-20.log"), "recent").expect("write recent");
    std::fs::write(logs.join("notes.txt"), "skip").expect("write notes");

    let removed = cleanup_old_logs_at(
        &logs,
        7,
        NaiveDate::from_ymd_opt(2026, 2, 24).expect("date"),
    )
    .expect("cleanup logs");

    assert_eq!(removed, 1);
    assert!(!logs.join("2026-02-10.log").exists());
    assert!(logs.join("2026-02-20.log").exists());
    assert!(logs.join("notes.txt").exists());
}

#[test]
fn init_logging_writes_to_workspace_log_file() {
    let temp = tempdir().expect("tempdir");
    let workspace_base = temp.path().join("home");
    std::fs::create_dir_all(&workspace_base).expect("create home");

    let config = AppConfig::from_inputs(
        AgentFileConfig {
            workspace: Some(std::path::PathBuf::from("./workspace")),
            logging: AgentLoggingConfig {
                level: Some("info".to_string()),
                retention_days: Some(7),
                directory: Some(std::path::PathBuf::from("./logs")),
            },
            server: AgentServerConfig::default(),
            llm: AgentLlmConfig::default(),
            secrets: AgentSecretsConfig::default(),
        },
        EnvSecrets::default(),
        workspace_base.clone(),
    );

    let runtime = init_logging(&config).expect("init logging");
    tracing::info!("unit logging smoke");
    let log_path = runtime.log_file.clone();
    drop(runtime);

    let content = std::fs::read_to_string(log_path).expect("read log");
    assert!(content.contains("unit logging smoke"));
}
