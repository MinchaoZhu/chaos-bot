use chaos_bot_backend::infrastructure::memory::{MemoryBackend, MemoryStore};
use chaos_bot_backend::infrastructure::tooling::*;
use serde_json::json;
use std::sync::Arc;
use tempfile::tempdir;

fn make_context() -> (tempfile::TempDir, ToolContext) {
    let temp = tempdir().unwrap();
    let memory: Arc<dyn MemoryBackend> = Arc::new(MemoryStore::new(
        temp.path().join("memory"),
        temp.path().join("MEMORY.md"),
    ));
    let ctx = ToolContext::new(temp.path().to_path_buf(), memory);
    (temp, ctx)
}

// -------------------------------------------------------------------------
// ToolRegistry
// -------------------------------------------------------------------------

#[test]
fn registry_new_is_empty() {
    let reg = ToolRegistry::new();
    assert!(reg.specs().is_empty());
}

#[test]
fn registry_register_and_specs() {
    let mut reg = ToolRegistry::new();
    reg.register(ReadTool);
    let specs = reg.specs();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].name, "read");
}

#[test]
fn registry_register_default_tools_count() {
    let mut reg = ToolRegistry::new();
    reg.register_default_tools();
    // ReadTool registered by both coding and read-only, but HashMap deduplicates
    // coding: read, write, edit, bash
    // read-only: read, grep, find, ls
    // memory: memory_get, memory_search
    // unique: read, write, edit, bash, grep, find, ls, memory_get, memory_search = 9
    assert_eq!(reg.specs().len(), 9);
}

#[tokio::test]
async fn registry_dispatch_existing_tool() {
    let (_temp, ctx) = make_context();
    // Create a file in root_dir
    std::fs::write(ctx.root_dir.join("hello.txt"), "world").unwrap();

    let mut reg = ToolRegistry::new();
    reg.register(ReadTool);

    let result = reg
        .dispatch("tc_1", "read", json!({"path": "hello.txt"}), &ctx)
        .await
        .unwrap();

    assert_eq!(result.name, "read");
    assert!(result.output.contains("world"));
    assert!(!result.is_error);
}

#[tokio::test]
async fn registry_dispatch_unknown_tool_errors() {
    let (_temp, ctx) = make_context();
    let reg = ToolRegistry::new();
    let result = reg.dispatch("tc_1", "nonexistent", json!({}), &ctx).await;
    assert!(result.is_err());
}

// -------------------------------------------------------------------------
// slice_lines
// -------------------------------------------------------------------------

#[test]
fn slice_lines_no_range() {
    let content = "a\nb\nc";
    assert_eq!(slice_lines(content, None, None), "a\nb\nc");
}

#[test]
fn slice_lines_start_and_end() {
    let content = "line1\nline2\nline3\nline4";
    assert_eq!(slice_lines(content, Some(2), Some(3)), "line2\nline3");
}

#[test]
fn slice_lines_start_only() {
    let content = "line1\nline2\nline3";
    assert_eq!(slice_lines(content, Some(2), None), "line2\nline3");
}

#[test]
fn slice_lines_out_of_range_returns_empty() {
    let content = "line1";
    assert_eq!(slice_lines(content, Some(10), Some(20)), "");
}

#[test]
fn slice_lines_start_zero_treated_as_beginning() {
    let content = "line1\nline2";
    // start_line 0 -> saturating_sub(1) = 0
    assert_eq!(slice_lines(content, Some(0), Some(1)), "line1");
}

// -------------------------------------------------------------------------
// ensure_within_root
// -------------------------------------------------------------------------

#[test]
fn ensure_within_root_allows_child_path() {
    let temp = tempdir().unwrap();
    let child = temp.path().join("child.txt");
    std::fs::write(&child, "").unwrap();
    assert!(ensure_within_root(temp.path(), &child).is_ok());
}

#[test]
fn ensure_within_root_rejects_escape() {
    let temp = tempdir().unwrap();
    let other = tempdir().unwrap();
    let file = other.path().join("escape.txt");
    std::fs::write(&file, "").unwrap();
    assert!(ensure_within_root(temp.path(), &file).is_err());
}

// -------------------------------------------------------------------------
// ReadTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn read_tool_reads_file() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("test.txt"), "hello\nworld").unwrap();

    let tool = ReadTool;
    let result = tool
        .execute(json!({"path": "test.txt"}), &ctx)
        .await
        .unwrap();
    assert_eq!(result.output, "hello\nworld");
    assert!(!result.is_error);
}

#[tokio::test]
async fn read_tool_with_line_range() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("test.txt"), "a\nb\nc\nd").unwrap();

    let tool = ReadTool;
    let result = tool
        .execute(
            json!({"path": "test.txt", "start_line": 2, "end_line": 3}),
            &ctx,
        )
        .await
        .unwrap();
    assert_eq!(result.output, "b\nc");
}

#[tokio::test]
async fn read_tool_missing_file_errors() {
    let (_temp, ctx) = make_context();
    let tool = ReadTool;
    let result = tool.execute(json!({"path": "nope.txt"}), &ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn read_tool_missing_path_arg_errors() {
    let (_temp, ctx) = make_context();
    let tool = ReadTool;
    let result = tool.execute(json!({}), &ctx).await;
    assert!(result.is_err());
}

// -------------------------------------------------------------------------
// WriteTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn write_tool_creates_file() {
    let (_temp, ctx) = make_context();
    let tool = WriteTool;
    let result = tool
        .execute(json!({"path": "out.txt", "content": "data"}), &ctx)
        .await
        .unwrap();
    assert!(result.output.contains("4 bytes"));
    let content = std::fs::read_to_string(ctx.root_dir.join("out.txt")).unwrap();
    assert_eq!(content, "data");
}

#[tokio::test]
async fn write_tool_append_mode() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("out.txt"), "first").unwrap();

    let tool = WriteTool;
    tool.execute(
        json!({"path": "out.txt", "content": "second", "append": true}),
        &ctx,
    )
    .await
    .unwrap();

    let content = std::fs::read_to_string(ctx.root_dir.join("out.txt")).unwrap();
    assert_eq!(content, "firstsecond");
}

#[tokio::test]
async fn write_tool_creates_parent_dirs() {
    let (_temp, ctx) = make_context();
    let tool = WriteTool;
    tool.execute(json!({"path": "sub/dir/file.txt", "content": "deep"}), &ctx)
        .await
        .unwrap();

    let content = std::fs::read_to_string(ctx.root_dir.join("sub/dir/file.txt")).unwrap();
    assert_eq!(content, "deep");
}

// -------------------------------------------------------------------------
// EditTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn edit_tool_replaces_string() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("edit.txt"), "hello world").unwrap();

    let tool = EditTool;
    let result = tool
        .execute(
            json!({"path": "edit.txt", "find": "world", "replace": "rust"}),
            &ctx,
        )
        .await
        .unwrap();
    assert!(result.output.contains("updated"));

    let content = std::fs::read_to_string(ctx.root_dir.join("edit.txt")).unwrap();
    assert_eq!(content, "hello rust");
}

#[tokio::test]
async fn edit_tool_string_not_found_errors() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("edit.txt"), "hello").unwrap();

    let tool = EditTool;
    let result = tool
        .execute(
            json!({"path": "edit.txt", "find": "missing", "replace": "x"}),
            &ctx,
        )
        .await;
    assert!(result.is_err());
}

// -------------------------------------------------------------------------
// BashTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn bash_tool_allowed_command() {
    let (_temp, ctx) = make_context();
    let tool = BashTool;
    let result = tool
        .execute(json!({"command": "echo hello"}), &ctx)
        .await
        .unwrap();
    assert!(result.output.contains("hello"));
    assert!(!result.is_error);
}

#[tokio::test]
async fn bash_tool_disallowed_command_errors() {
    let (_temp, ctx) = make_context();
    let tool = BashTool;
    let result = tool.execute(json!({"command": "rm -rf /"}), &ctx).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn bash_tool_pwd_runs_in_root_dir() {
    let (_temp, ctx) = make_context();
    let tool = BashTool;
    let result = tool.execute(json!({"command": "pwd"}), &ctx).await.unwrap();
    // The output should contain the temp dir path
    let canonical_root = std::fs::canonicalize(&ctx.root_dir).unwrap();
    assert!(result
        .output
        .contains(&canonical_root.to_string_lossy().to_string()));
}

// -------------------------------------------------------------------------
// GrepTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn grep_tool_finds_matches() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("file.txt"), "foo\nbar\nbaz foo").unwrap();

    let tool = GrepTool;
    let result = tool.execute(json!({"pattern": "foo"}), &ctx).await.unwrap();
    // Should find 2 matches
    let lines: Vec<&str> = result.output.lines().collect();
    assert_eq!(lines.len(), 2);
}

#[tokio::test]
async fn grep_tool_case_insensitive() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("file.txt"), "FOO\nfoo").unwrap();

    let tool = GrepTool;
    let result = tool.execute(json!({"pattern": "foo"}), &ctx).await.unwrap();
    let lines: Vec<&str> = result.output.lines().collect();
    assert_eq!(lines.len(), 2);
}

// -------------------------------------------------------------------------
// FindTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn find_tool_finds_files() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("test.rs"), "").unwrap();
    std::fs::write(ctx.root_dir.join("test.txt"), "").unwrap();

    let tool = FindTool;
    let result = tool.execute(json!({"pattern": ".rs"}), &ctx).await.unwrap();
    assert!(result.output.contains("test.rs"));
}

// -------------------------------------------------------------------------
// LsTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn ls_tool_lists_directory() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("a.txt"), "").unwrap();
    std::fs::create_dir(ctx.root_dir.join("subdir")).unwrap();

    let tool = LsTool;
    let result = tool.execute(json!({}), &ctx).await.unwrap();
    assert!(result.output.contains("a.txt"));
    assert!(result.output.contains("subdir/"));
}

#[tokio::test]
async fn ls_tool_sorted_output() {
    let (_temp, ctx) = make_context();
    std::fs::write(ctx.root_dir.join("z.txt"), "").unwrap();
    std::fs::write(ctx.root_dir.join("a.txt"), "").unwrap();

    let tool = LsTool;
    let result = tool.execute(json!({}), &ctx).await.unwrap();
    let lines: Vec<&str> = result.output.lines().collect();
    let a_pos = lines.iter().position(|l| l.contains("a.txt"));
    let z_pos = lines.iter().position(|l| l.contains("z.txt"));
    assert!(a_pos < z_pos);
}

// -------------------------------------------------------------------------
// MemoryGetTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn memory_get_tool_reads_curated() {
    let (_temp, ctx) = make_context();
    ctx.memory.ensure_layout().await.unwrap();
    ctx.memory.write_curated("memory content").await.unwrap();

    let tool = MemoryGetTool;
    let result = tool
        .execute(json!({"path": "MEMORY.md"}), &ctx)
        .await
        .unwrap();
    assert!(result.output.contains("memory content"));
}

// -------------------------------------------------------------------------
// MemorySearchTool
// -------------------------------------------------------------------------

#[tokio::test]
async fn memory_search_tool_finds_hits() {
    let (_temp, ctx) = make_context();
    ctx.memory.ensure_layout().await.unwrap();
    ctx.memory
        .write_curated("important: project notes\n")
        .await
        .unwrap();

    let tool = MemorySearchTool;
    let result = tool
        .execute(json!({"keyword": "important"}), &ctx)
        .await
        .unwrap();
    assert!(result.output.contains("important"));
}

// -------------------------------------------------------------------------
// Tool trait metadata
// -------------------------------------------------------------------------

#[test]
fn tool_names_and_descriptions() {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ReadTool),
        Box::new(WriteTool),
        Box::new(EditTool),
        Box::new(BashTool),
        Box::new(GrepTool),
        Box::new(FindTool),
        Box::new(LsTool),
        Box::new(MemoryGetTool),
        Box::new(MemorySearchTool),
    ];

    for tool in &tools {
        assert!(!tool.name().is_empty());
        assert!(!tool.description().is_empty());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
    }
}
