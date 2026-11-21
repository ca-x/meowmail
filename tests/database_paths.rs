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

#[tokio::test]
async fn every_pool_connection_uses_the_same_sqlite_settings() {
    let directory = tempfile::tempdir().unwrap();
    let db = Database::connect(&directory.path().join("mail.sqlite"))
        .await
        .unwrap();
    let pool = db.connection().get_sqlite_connection_pool();
    let mut held = Vec::new();
    for _ in 0..pool.options().get_max_connections() {
        let mut connection = pool.acquire().await.unwrap();
        for (pragma, expected) in [
            ("PRAGMA synchronous", 1),
            ("PRAGMA foreign_keys", 1),
            ("PRAGMA busy_timeout", 5000),
        ] {
            let actual: i64 = sea_orm::sqlx::query_scalar(pragma)
                .fetch_one(&mut *connection)
                .await
                .unwrap();
            assert_eq!(actual, expected, "{pragma} on connection {}", held.len());
        }
        held.push(connection);
    }
}
