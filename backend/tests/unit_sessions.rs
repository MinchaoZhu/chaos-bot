use chaos_bot_backend::sessions::SessionStore;
use chaos_bot_backend::types::Message;

#[tokio::test]
async fn create_returns_unique_session() {
    let store = SessionStore::new();
    let s1 = store.create().await;
    let s2 = store.create().await;
    assert_ne!(s1.id, s2.id);
    assert!(s1.messages.is_empty());
}

#[tokio::test]
async fn get_existing_session() {
    let store = SessionStore::new();
    let s = store.create().await;
    let found = store.get(&s.id).await;
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, s.id);
}

#[tokio::test]
async fn get_missing_session_returns_none() {
    let store = SessionStore::new();
    assert!(store.get("nonexistent").await.is_none());
}

#[tokio::test]
async fn delete_existing_session() {
    let store = SessionStore::new();
    let s = store.create().await;
    assert!(store.delete(&s.id).await);
    assert!(store.get(&s.id).await.is_none());
}

#[tokio::test]
async fn delete_missing_session_returns_false() {
    let store = SessionStore::new();
    assert!(!store.delete("nonexistent").await);
}

#[tokio::test]
async fn list_returns_all_sessions_sorted_by_updated_at() {
    let store = SessionStore::new();
    let s1 = store.create().await;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let s2 = store.create().await;

    let list = store.list().await;
    assert_eq!(list.len(), 2);
    // Most recently updated first
    assert_eq!(list[0].id, s2.id);
    assert_eq!(list[1].id, s1.id);
}

#[tokio::test]
async fn upsert_updates_existing_session() {
    let store = SessionStore::new();
    let mut s = store.create().await;
    s.push_message(Message::user("hello"));
    store.upsert(s.clone()).await;

    let found = store.get(&s.id).await.unwrap();
    assert_eq!(found.messages.len(), 1);
}

#[tokio::test]
async fn upsert_inserts_new_session() {
    let store = SessionStore::new();
    let s = chaos_bot_backend::types::SessionState::new("custom-id");
    store.upsert(s).await;
    let found = store.get("custom-id").await;
    assert!(found.is_some());
}

#[tokio::test]
async fn list_empty_store() {
    let store = SessionStore::new();
    assert!(store.list().await.is_empty());
}

#[tokio::test]
async fn concurrent_create() {
    let store = SessionStore::new();
    let mut handles = Vec::new();
    for _ in 0..10 {
        let s = store.clone();
        handles.push(tokio::spawn(async move { s.create().await }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(store.list().await.len(), 10);
}
