use chaos_bot_backend::infrastructure::personality::PersonalityLoader;
use tempfile::tempdir;

#[tokio::test]
async fn load_sections_in_correct_order() {
    let temp = tempdir().unwrap();
    let dir = temp.path().join("personality");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SOUL.md"), "soul content").unwrap();
    std::fs::write(dir.join("IDENTITY.md"), "identity content").unwrap();
    std::fs::write(dir.join("USER.md"), "user content").unwrap();
    std::fs::write(dir.join("AGENTS.md"), "agents content").unwrap();

    let loader = PersonalityLoader::new(&dir);
    let sections = loader.load_sections().await.unwrap();

    assert_eq!(sections.len(), 4);
    assert_eq!(sections[0].0, "SOUL.md");
    assert_eq!(sections[1].0, "IDENTITY.md");
    assert_eq!(sections[2].0, "USER.md");
    assert_eq!(sections[3].0, "AGENTS.md");
}

#[tokio::test]
async fn load_sections_skips_missing_files() {
    let temp = tempdir().unwrap();
    let dir = temp.path().join("personality");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SOUL.md"), "soul only").unwrap();

    let loader = PersonalityLoader::new(&dir);
    let sections = loader.load_sections().await.unwrap();

    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].0, "SOUL.md");
    assert_eq!(sections[0].1, "soul only");
}

#[tokio::test]
async fn load_sections_empty_dir() {
    let temp = tempdir().unwrap();
    let dir = temp.path().join("personality");
    std::fs::create_dir_all(&dir).unwrap();

    let loader = PersonalityLoader::new(&dir);
    let sections = loader.load_sections().await.unwrap();
    assert!(sections.is_empty());
}

#[tokio::test]
async fn system_prompt_concatenates_with_headers() {
    let temp = tempdir().unwrap();
    let dir = temp.path().join("personality");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SOUL.md"), "You are helpful").unwrap();
    std::fs::write(dir.join("IDENTITY.md"), "Bot v1").unwrap();

    let loader = PersonalityLoader::new(&dir);
    let prompt = loader.system_prompt().await.unwrap();

    assert!(prompt.contains("## SOUL.md"));
    assert!(prompt.contains("You are helpful"));
    assert!(prompt.contains("## IDENTITY.md"));
    assert!(prompt.contains("Bot v1"));
}

#[tokio::test]
async fn system_prompt_empty_dir_returns_empty() {
    let temp = tempdir().unwrap();
    let dir = temp.path().join("personality");
    std::fs::create_dir_all(&dir).unwrap();

    let loader = PersonalityLoader::new(&dir);
    let prompt = loader.system_prompt().await.unwrap();
    assert!(prompt.is_empty());
}

#[tokio::test]
async fn dir_accessor() {
    let temp = tempdir().unwrap();
    let dir = temp.path().join("p");
    let loader = PersonalityLoader::new(&dir);
    assert_eq!(loader.dir(), dir);
}

#[tokio::test]
async fn system_prompt_trims_content() {
    let temp = tempdir().unwrap();
    let dir = temp.path().join("personality");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SOUL.md"), "  content with spaces  \n\n").unwrap();

    let loader = PersonalityLoader::new(&dir);
    let prompt = loader.system_prompt().await.unwrap();
    assert!(prompt.contains("content with spaces"));
}
