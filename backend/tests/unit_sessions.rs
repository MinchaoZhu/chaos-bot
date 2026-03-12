use chaos_bot_backend::domain::types::{Message, SessionState};
use chaos_bot_backend::infrastructure::session_store::SessionStore;
use tempfile::tempdir;

fn store() -> SessionStore {
    let temp = tempdir().expect("tempdir");
    let root = temp.keep();
    SessionStore::new(root.join("sessions"))
}

#[tokio::test]
async fn create_returns_unique_session() {
    let store = store();
    let s1 = store.create().await.expect("create session");
    let s2 = store.create().await.expect("create session");
    assert_ne!(s1.id, s2.id);
    assert!(s1.messages.is_empty());
}

#[tokio::test]
async fn get_and_delete_roundtrip() {
    let store = store();
    let session = store.create().await.expect("create");
    let found = store.get(&session.id).await.expect("get");
    assert_eq!(found.expect("session").id, session.id);

    assert!(store.delete(&session.id).await.expect("delete"));
    assert!(store.get(&session.id).await.expect("get").is_none());
}

#[tokio::test]
async fn upsert_persists_messages_to_disk() {
    let store = store();
    let mut session = SessionState::new("custom-id");
    session.push_message(Message::user("hello"));
    store.upsert(session.clone()).await.expect("upsert");

    let found = store.get("custom-id").await.expect("get").expect("session");
    assert_eq!(found.messages.len(), 1);
    assert_eq!(found.messages[0].content, "hello");
}

#[tokio::test]
async fn list_returns_sessions_sorted_by_updated_at() {
    let store = store();
    let s1 = store.create().await.expect("create");
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let s2 = store.create().await.expect("create");

    let list = store.list().await.expect("list");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].id, s2.id);
    assert_eq!(list[1].id, s1.id);
}

#[tokio::test]
async fn invalid_session_id_is_rejected() {
    let store = store();
    assert!(store.get("../bad").await.expect("get").is_none());
    assert!(!store.delete("../bad").await.expect("delete"));
}
