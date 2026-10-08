use meowmail::db::Database;

#[tokio::test]
async fn database_preserves_special_characters_in_paths() {
    let directory = tempfile::tempdir().unwrap();
    for name in ["mail %23 # + 空格.sqlite", "mail?mode=ro.sqlite"] {
        // Question marks are not valid Windows filenames.
        if cfg!(windows) && name.contains('?') {
            continue;
        }
        let path = directory.path().join(name);
        Database::connect(&path).await.unwrap();
        assert!(
            path.is_file(),
            "database must use the exact requested filename"
        );
    }
}

#[tokio::test]
async fn database_initializes_in_a_native_nested_path() {
    let directory = tempfile::tempdir().unwrap();
    let data = directory.path().join("妙邮 data %23").join("data");
    std::fs::create_dir_all(&data).unwrap();
    let path = data.join("meowmail.sqlite");
    Database::connect(&path).await.unwrap();
    assert!(path.is_file());
    Database::connect(&path).await.unwrap();
}
