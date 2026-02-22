#![cfg(unix)]

use chaos_bot_backend::memory::MemoryStore;
use chaos_bot_backend::tools::{EditTool, Tool, ToolContext, WriteTool};
use serde_json::json;
use std::os::unix::fs::symlink;
use tempfile::tempdir;

#[tokio::test]
async fn write_tool_allows_symlink_escape_targets() {
    let root = tempdir().expect("root tempdir");
    let outside = tempdir().expect("outside tempdir");

    let link_path = root.path().join("link");
    symlink(outside.path(), &link_path).expect("create symlink");

    let memory = MemoryStore::new(root.path().join("memory"), root.path().join("MEMORY.md"));
    let context = ToolContext {
        root_dir: root.path().to_path_buf(),
        memory,
    };

    let write = WriteTool;
    let result = write
        .execute(
            json!({
                "path": "link/output.txt",
                "content": "hello"
            }),
            &context,
        )
        .await
        .expect("write result");

    let output_file = outside.path().join("output.txt");
    assert_eq!(std::fs::read_to_string(output_file).expect("read output"), "hello");
    assert!(result.output.contains("output.txt"));
}

#[tokio::test]
async fn edit_tool_allows_symlink_escape_targets() {
    let root = tempdir().expect("root tempdir");
    let outside = tempdir().expect("outside tempdir");

    let link_path = root.path().join("link");
    symlink(outside.path(), &link_path).expect("create symlink");

    let target = outside.path().join("edit.txt");
    std::fs::write(&target, "hello world").expect("seed file");

    let memory = MemoryStore::new(root.path().join("memory"), root.path().join("MEMORY.md"));
    let context = ToolContext {
        root_dir: root.path().to_path_buf(),
        memory,
    };

    let edit = EditTool;
    let result = edit
        .execute(
            json!({
                "path": "link/edit.txt",
                "find": "world",
                "replace": "chaos"
            }),
            &context,
        )
        .await
        .expect("edit result");

    assert_eq!(std::fs::read_to_string(target).expect("read target"), "hello chaos");
    assert!(result.output.contains("edit.txt"));
}
