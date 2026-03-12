use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

fn cli_bin() -> PathBuf {
    let exe = std::env::current_exe().expect("current exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .map(|debug_dir| debug_dir.join("chaos-bot"))
        .expect("target/debug")
}

fn write_mock_config(home: &Path) {
    let config_dir = home.join(".chaos-bot");
    fs::create_dir_all(&config_dir).expect("create config dir");
    fs::write(
        config_dir.join("config.json"),
        r#"{
  "llm": {
    "provider": "mock",
    "model": "mock-model"
  },
  "logging": {
    "level": "error"
  }
}
"#,
    )
    .expect("write config");
}

fn run_cli(home: &Path, args: &[&str]) -> Output {
    Command::new(cli_bin())
        .args(args)
        .env("HOME", home)
        .current_dir(home)
        .output()
        .expect("run cli")
}

fn run_cli_ok(home: &Path, args: &[&str]) -> String {
    let output = run_cli(home, args);
    if !output.status.success() {
        panic!(
            "command failed: {:?}\nstatus: {}\nstdout:\n{}\nstderr:\n{}",
            args,
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).expect("utf8 stdout")
}

fn run_cli_json(home: &Path, args: &[&str]) -> Value {
    serde_json::from_str(&run_cli_ok(home, args)).expect("json stdout")
}

fn run_cli_with_stdin(home: &Path, args: &[&str], stdin: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(cli_bin())
        .args(args)
        .env("HOME", home)
        .current_dir(home)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn cli");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("wait output")
}

fn init_skill_repo(root: &Path) -> PathBuf {
    let repo_dir = root.join("skill-repo.git");
    let skill_dir = repo_dir.join("hello-skill");
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: hello-skill
description: Local smoke-test skill
---

# Hello Skill

Used by CLI integration tests.
"#,
    )
    .expect("write skill");

    for args in [
        ["init", "--initial-branch=main"].as_slice(),
        ["config", "user.email", "cli-tests@example.com"].as_slice(),
        ["config", "user.name", "CLI Tests"].as_slice(),
        ["add", "."].as_slice(),
        ["commit", "-m", "Feature: add smoke skill"].as_slice(),
    ] {
        let output = Command::new("git")
            .args(args)
            .current_dir(&repo_dir)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    repo_dir
}

#[test]
fn cli_chat_sessions_config_and_stdin_flow() {
    let home = TempDir::new().expect("tempdir");
    write_mock_config(home.path());

    let help = run_cli_ok(home.path(), &["--help"]);
    assert!(help.contains("chat"));
    assert!(help.contains("sessions"));
    assert!(!help.contains("serve"));

    let initial_config = run_cli_json(home.path(), &["--output", "json", "config", "get"]);
    assert_eq!(initial_config["running"]["llm"]["provider"], "mock");

    let chat = run_cli_json(
        home.path(),
        &["--output", "json", "chat", "hello from cli integration"],
    );
    let session_id = chat["session_id"].as_str().expect("session id").to_string();
    assert_eq!(chat["finish_reason"], "stop");
    assert!(chat["assistant_message"]
        .as_str()
        .expect("assistant message")
        .contains("Mock response to: hello from cli integration"));

    let sessions = run_cli_json(home.path(), &["--output", "json", "sessions", "list"]);
    assert_eq!(sessions.as_array().expect("sessions").len(), 1);
    assert_eq!(sessions[0]["id"], session_id);
    assert_eq!(sessions[0]["message_count"], 2);

    let continued = run_cli_json(
        home.path(),
        &[
            "--output",
            "json",
            "chat",
            &session_id,
            "follow up from cli integration",
        ],
    );
    assert_eq!(continued["session_id"], session_id);

    let session = run_cli_json(
        home.path(),
        &["--output", "json", "sessions", "get", &session_id],
    );
    assert_eq!(session["messages"].as_array().expect("messages").len(), 4);

    let stdin_chat = run_cli_with_stdin(
        home.path(),
        &[
            "--output",
            "json",
            "chat",
            "--session",
            &session_id,
            "--stdin",
        ],
        "stdin follow up\n",
    );
    assert!(stdin_chat.status.success());
    let stdin_json: Value = serde_json::from_slice(&stdin_chat.stdout).expect("stdin json");
    assert_eq!(stdin_json["session_id"], session_id);

    let stream = run_cli_ok(
        home.path(),
        &[
            "--output",
            "jsonl",
            "chat",
            "--stream",
            "stream from cli integration",
        ],
    );
    assert!(stream
        .lines()
        .any(|line| line.contains(r#""event":"session""#)));
    assert!(stream
        .lines()
        .any(|line| line.contains(r#""event":"delta""#)));
    assert!(stream
        .lines()
        .any(|line| line.contains(r#""event":"done""#)));

    let apply = run_cli_json(
        home.path(),
        &[
            "--output",
            "json",
            "config",
            "apply",
            "--raw",
            r#"{"llm":{"provider":"mock","model":"mock-model-v2","temperature":0.1},"logging":{"level":"error"}}"#,
        ],
    );
    assert_eq!(apply["ok"], true);

    let updated_config = run_cli_json(home.path(), &["--output", "json", "config", "get"]);
    assert_eq!(updated_config["running"]["llm"]["model"], "mock-model-v2");

    let delete = run_cli_json(
        home.path(),
        &["--output", "json", "sessions", "delete", &session_id],
    );
    assert_eq!(delete["ok"], true);

    let get_missing = run_cli(
        home.path(),
        &["--output", "json", "sessions", "get", &session_id],
    );
    assert!(!get_missing.status.success());
    let stderr = String::from_utf8(get_missing.stderr).expect("stderr utf8");
    assert!(stderr.contains("\"code\": \"not_found\""));
}

#[test]
fn cli_skills_upgrade_and_workspace_flag_flow() {
    let home = TempDir::new().expect("tempdir");
    let workspace = home.path().join("custom-workspace");
    fs::create_dir_all(&workspace).expect("workspace dir");
    write_mock_config(home.path());

    let chat = run_cli_json(
        home.path(),
        &[
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            "json",
            "chat",
            "workspace override",
        ],
    );
    let session_id = chat["session_id"].as_str().expect("session id");
    assert!(workspace
        .join("sessions")
        .join(format!("{session_id}.json"))
        .exists());

    let repo_root = TempDir::new().expect("tempdir");
    let repo_dir = init_skill_repo(repo_root.path());

    let install = run_cli_json(
        home.path(),
        &[
            "--output",
            "json",
            "skills",
            "install",
            repo_dir.to_str().expect("repo path"),
        ],
    );
    assert_eq!(install["ok"], true);
    assert_eq!(install["installed"][0]["id"], "hello-skill");

    let skills = run_cli_json(home.path(), &["--output", "json", "skills", "list"]);
    assert!(skills
        .as_array()
        .expect("skills array")
        .iter()
        .any(|skill| skill["id"] == "hello-skill"));

    let upgrade = run_cli_json(home.path(), &["--output", "json", "upgrade", "status"]);
    assert_eq!(upgrade["supported"], Value::Bool(false));
    assert!(upgrade["reason"]
        .as_str()
        .expect("upgrade reason")
        .contains("installed release bundles"));
}
