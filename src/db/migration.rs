use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(InitialMigration)]
    }
}

struct InitialMigration;

impl MigrationName for InitialMigration {
    fn name(&self) -> &str {
        "m20260726_000001_initial"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for InitialMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE users (
                    id TEXT PRIMARY KEY NOT NULL,
                    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
                    nickname TEXT NOT NULL,
                    email TEXT,
                    role TEXT NOT NULL CHECK(role IN ('admin', 'user')),
                    password_hash TEXT,
                    pin_hash TEXT,
                    avatar_mime TEXT,
                    avatar_data BLOB,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    last_login_at BIGINT
                );

                CREATE TABLE user_identities (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    issuer TEXT NOT NULL,
                    subject TEXT NOT NULL,
                    created_at BIGINT NOT NULL,
                    last_login_at BIGINT NOT NULL,
                    CONSTRAINT fk_identity_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(issuer, subject)
                );
                CREATE INDEX idx_user_identities_user ON user_identities(user_id);

                CREATE TABLE system_state (
                    key TEXT PRIMARY KEY NOT NULL,
                    value TEXT NOT NULL
                );

                CREATE TABLE mail_accounts (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    display_name VARCHAR(80) NOT NULL,
                    email VARCHAR(254) NOT NULL,
                    username VARCHAR(320) NOT NULL,
                    password_cipher TEXT NOT NULL,
                    imap_host VARCHAR(253) NOT NULL,
                    imap_port INTEGER NOT NULL,
                    imap_security VARCHAR(16) NOT NULL,
                    smtp_host VARCHAR(253) NOT NULL,
                    smtp_port INTEGER NOT NULL,
                    smtp_security VARCHAR(16) NOT NULL,
                    proxy_kind VARCHAR(16) NOT NULL DEFAULT 'direct',
                    proxy_host VARCHAR(253),
                    proxy_port INTEGER,
                    proxy_username VARCHAR(255),
                    proxy_password_cipher TEXT,
                    is_default BOOLEAN NOT NULL DEFAULT 0,
                    last_synced_at BIGINT,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_mail_account_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(user_id, email)
                );
                CREATE INDEX idx_mail_accounts_user ON mail_accounts(user_id, created_at);

                CREATE TABLE messages (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    account_id TEXT NOT NULL,
                    folder VARCHAR(160) NOT NULL,
                    uid BIGINT NOT NULL,
                    message_id VARCHAR(998),
                    sender_name VARCHAR(320),
                    sender_email VARCHAR(320) NOT NULL,
                    recipients_json TEXT NOT NULL,
                    subject TEXT NOT NULL,
                    preview TEXT NOT NULL,
                    body_text TEXT NOT NULL,
                    body_html TEXT,
                    received_at BIGINT NOT NULL,
                    is_read BOOLEAN NOT NULL DEFAULT 0,
                    is_starred BOOLEAN NOT NULL DEFAULT 0,
                    attachment_count INTEGER NOT NULL DEFAULT 0,
                    created_at BIGINT NOT NULL,
                    CONSTRAINT fk_message_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_message_account FOREIGN KEY(account_id)
                        REFERENCES mail_accounts(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(account_id, folder, uid)
                );
                CREATE INDEX idx_messages_user_list
                    ON messages(user_id, account_id, folder, received_at);

                CREATE TABLE preferences (
                    user_id TEXT NOT NULL,
                    key TEXT NOT NULL,
                    value TEXT NOT NULL,
                    PRIMARY KEY(user_id, key),
                    CONSTRAINT fk_preference_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE
                );

                CREATE TABLE notification_settings (
                    user_id TEXT PRIMARY KEY NOT NULL,
                    enabled BOOLEAN NOT NULL DEFAULT 0,
                    message_template TEXT NOT NULL,
                    command_template TEXT,
                    http_url TEXT,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_notification_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE
                );

                CREATE TABLE mail_settings (
                    user_id TEXT PRIMARY KEY NOT NULL,
                    keep_local_after_server_delete BOOLEAN NOT NULL DEFAULT 1,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_mail_setting_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE
                );

                CREATE TABLE cleanup_rules (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    account_id TEXT,
                    name VARCHAR(120) NOT NULL,
                    sender_contains VARCHAR(320),
                    subject_contains VARCHAR(998),
                    body_contains TEXT,
                    older_than_days INTEGER,
                    delete_from_server BOOLEAN NOT NULL DEFAULT 0,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_cleanup_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_cleanup_account FOREIGN KEY(account_id)
                        REFERENCES mail_accounts(id) ON UPDATE CASCADE ON DELETE CASCADE
                );
                CREATE INDEX idx_cleanup_rules_user ON cleanup_rules(user_id, enabled);
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TABLE IF EXISTS cleanup_rules;
                DROP TABLE IF EXISTS mail_settings;
                DROP TABLE IF EXISTS notification_settings;
                DROP TABLE IF EXISTS preferences;
                DROP TABLE IF EXISTS messages;
                DROP TABLE IF EXISTS mail_accounts;
                DROP TABLE IF EXISTS user_identities;
                DROP TABLE IF EXISTS system_state;
                DROP TABLE IF EXISTS users;
                "#,
            )
            .await?;
        Ok(())
    }
}
