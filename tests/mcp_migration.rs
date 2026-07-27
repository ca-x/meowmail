use meowmail::db::Database;
use sea_orm::{ConnectionTrait, Database as SeaDatabase, DatabaseBackend, Statement};

#[tokio::test]
async fn existing_0_2_database_receives_additive_0_3_schema() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("meowmail.sqlite3");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let connection = SeaDatabase::connect(url).await.unwrap();
    connection
        .execute_unprepared(
            r#"
            PRAGMA foreign_keys = ON;
            CREATE TABLE seaql_migrations (
                version TEXT PRIMARY KEY NOT NULL,
                applied_at BIGINT NOT NULL
            );
            INSERT INTO seaql_migrations(version, applied_at)
                VALUES('m20260726_000001_initial', 0);
            CREATE TABLE users (id TEXT PRIMARY KEY NOT NULL);
            INSERT INTO users(id) VALUES('user-1');
            CREATE TABLE mail_accounts (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT,
                CONSTRAINT fk_mail_account_user FOREIGN KEY(user_id)
                    REFERENCES users(id) ON DELETE CASCADE
            );
            INSERT INTO mail_accounts(id, user_id) VALUES('account-1', 'user-1');
            CREATE TABLE messages (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT,
                account_id TEXT,
                folder TEXT NOT NULL DEFAULT 'INBOX',
                received_at BIGINT NOT NULL DEFAULT 0,
                CONSTRAINT fk_message_account FOREIGN KEY(account_id)
                    REFERENCES mail_accounts(id) ON DELETE CASCADE
            );
            INSERT INTO messages(id, user_id, account_id)
                VALUES('message-1', 'user-1', 'account-1');
            CREATE TABLE cleanup_rules (
                id TEXT PRIMARY KEY NOT NULL,
                user_id TEXT NOT NULL,
                account_id TEXT,
                name TEXT NOT NULL,
                sender_contains TEXT,
                subject_contains TEXT,
                body_contains TEXT,
                older_than_days INTEGER,
                delete_from_server BOOLEAN NOT NULL DEFAULT 0,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at BIGINT NOT NULL,
                updated_at BIGINT NOT NULL
            );
            CREATE TABLE preferences (
                user_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY(user_id, key)
            );
            CREATE TABLE mail_settings (
                user_id TEXT PRIMARY KEY NOT NULL,
                keep_local_after_server_delete BOOLEAN NOT NULL DEFAULT 1,
                updated_at BIGINT NOT NULL,
                CONSTRAINT fk_mail_setting_user FOREIGN KEY(user_id)
                    REFERENCES users(id) ON DELETE CASCADE
            );
            INSERT INTO mail_settings(user_id, keep_local_after_server_delete, updated_at)
                VALUES('user-1', 1, 0);
            "#,
        )
        .await
        .unwrap();
    connection.close().await.unwrap();

    let database = Database::connect(&path).await.unwrap();
    for table in ["mcp_tokens", "email_drafts", "message_attachments"] {
        let row = database
            .connection()
            .query_one_raw(Statement::from_sql_and_values(
                DatabaseBackend::Sqlite,
                "SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'table' AND name = ?",
                [table.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    }
    for (table, column) in [
        ("messages", "reply_to_email"),
        ("messages", "references_header"),
        ("messages", "uid_validity"),
        ("email_drafts", "status"),
    ] {
        let row = database
            .connection()
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!(
                    "SELECT COUNT(*) AS count FROM pragma_table_info('{table}') WHERE name = '{column}'"
                ),
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    }
    let preserved = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM messages WHERE id = 'message-1'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(preserved.try_get::<i64>("", "count").unwrap(), 1);
    let legacy_thread = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT thread_key FROM messages WHERE id = 'message-1'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        legacy_thread.try_get::<String>("", "thread_key").unwrap(),
        "legacy:message-1"
    );
    let fetch_limit = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT sync_fetch_limit FROM mail_settings WHERE user_id = 'user-1'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fetch_limit
            .try_get::<Option<i32>>("", "sync_fetch_limit")
            .unwrap(),
        Some(50)
    );
    let migration = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version = 'm20260727_000002_mcp_access'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        migration.try_get::<String>("", "version").unwrap(),
        "m20260727_000002_mcp_access"
    );
    let hardening = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version = 'm20260727_000003_mcp_hardening'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        hardening.try_get::<String>("", "version").unwrap(),
        "m20260727_000003_mcp_hardening"
    );
    let fetch_limit_migration = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version = 'm20260727_000004_sync_fetch_limit'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        fetch_limit_migration
            .try_get::<String>("", "version")
            .unwrap(),
        "m20260727_000004_sync_fetch_limit"
    );
    let integrity = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version = 'm20260727_000005_mcp_integrity'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        integrity.try_get::<String>("", "version").unwrap(),
        "m20260727_000005_mcp_integrity"
    );
    let attachments = database
        .connection()
        .query_one_raw(Statement::from_string(
            DatabaseBackend::Sqlite,
            "SELECT version FROM seaql_migrations WHERE version = 'm20260727_000006_attachment_preview'",
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        attachments.try_get::<String>("", "version").unwrap(),
        "m20260727_000006_attachment_preview"
    );
}
