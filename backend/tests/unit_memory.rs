use chaos_bot_backend::memory::MemoryStore;
use tempfile::tempdir;

fn make_store() -> (tempfile::TempDir, MemoryStore) {
    let temp = tempdir().unwrap();
    let memory_dir = temp.path().join("memory");
    let memory_file = temp.path().join("MEMORY.md");
    (temp, MemoryStore::new(memory_dir, memory_file))
}

#[tokio::test]
async fn ensure_layout_creates_dir_and_file() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    assert!(store.memory_dir().exists());
    assert!(store.curated_file().exists());
}

#[tokio::test]
async fn ensure_layout_idempotent() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store.ensure_layout().await.unwrap();
    assert!(store.curated_file().exists());
}

#[tokio::test]
async fn read_curated_returns_default_content() {
    let (_temp, store) = make_store();
    let content = store.read_curated().await.unwrap();
    assert!(content.contains("Long-Term Memory"));
}

#[tokio::test]
async fn write_curated_and_read_back() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store.write_curated("custom content").await.unwrap();
    let content = store.read_curated().await.unwrap();
    assert_eq!(content, "custom content");
}

#[tokio::test]
async fn append_daily_log_creates_file() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    let path = store.append_daily_log("test log entry").await.unwrap();
    assert!(path.exists());
    let content = tokio::fs::read_to_string(path).await.unwrap();
    assert!(content.contains("test log entry"));
}

#[tokio::test]
async fn append_daily_log_appends() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    let path = store.append_daily_log("first").await.unwrap();
    store.append_daily_log("second").await.unwrap();
    let content = tokio::fs::read_to_string(path).await.unwrap();
    assert!(content.contains("first"));
    assert!(content.contains("second"));
}

#[tokio::test]
async fn search_finds_matching_lines() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store
        .write_curated("hello world\ngoodbye world\n")
        .await
        .unwrap();
    let hits = store.search("hello").await.unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0].snippet.contains("hello"));
    assert_eq!(hits[0].line, 1);
}

#[tokio::test]
async fn search_case_insensitive() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store.write_curated("Hello World\n").await.unwrap();
    let hits = store.search("hello").await.unwrap();
    assert_eq!(hits.len(), 1);
}

#[tokio::test]
async fn search_empty_keyword_returns_empty() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    let hits = store.search("").await.unwrap();
    assert!(hits.is_empty());
}

#[tokio::test]
async fn search_whitespace_keyword_returns_empty() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    let hits = store.search("   ").await.unwrap();
    assert!(hits.is_empty());
}

#[tokio::test]
async fn search_across_daily_logs_and_curated() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store
        .write_curated("topic: Rust programming\n")
        .await
        .unwrap();
    store
        .append_daily_log("discussed topic today")
        .await
        .unwrap();
    let hits = store.search("topic").await.unwrap();
    assert!(hits.len() >= 2);
}

#[tokio::test]
async fn get_file_curated() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store.write_curated("line1\nline2\nline3\n").await.unwrap();
    let content = store.get_file("MEMORY.md", None, None).await.unwrap();
    assert!(content.contains("line1"));
}

#[tokio::test]
async fn get_file_with_line_range() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store
        .write_curated("line1\nline2\nline3\nline4\n")
        .await
        .unwrap();
    let content = store.get_file("MEMORY.md", Some(2), Some(3)).await.unwrap();
    assert!(content.contains("line2"));
    assert!(content.contains("line3"));
    assert!(!content.contains("line1"));
    assert!(!content.contains("line4"));
}

#[tokio::test]
async fn get_file_missing_returns_error() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    let result = store.get_file("nonexistent.md", None, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn get_file_daily_log() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    let path = store.append_daily_log("daily entry").await.unwrap();
    let filename = path.file_name().unwrap().to_string_lossy().to_string();
    let content = store.get_file(&filename, None, None).await.unwrap();
    assert!(content.contains("daily entry"));
}

#[tokio::test]
async fn get_file_out_of_range_returns_empty() {
    let (_temp, store) = make_store();
    store.ensure_layout().await.unwrap();
    store.write_curated("line1\n").await.unwrap();
    let content = store
        .get_file("MEMORY.md", Some(100), Some(200))
        .await
        .unwrap();
    assert!(content.is_empty());
}

#[tokio::test]
async fn memory_dir_and_curated_file_accessors() {
    let temp = tempdir().unwrap();
    let memory_dir = temp.path().join("mem");
    let curated_file = temp.path().join("MEM.md");
    let store = MemoryStore::new(&memory_dir, &curated_file);
    assert_eq!(store.memory_dir(), memory_dir);
    assert_eq!(store.curated_file(), curated_file);
}
