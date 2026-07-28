use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(InitialMigration),
            Box::new(McpAccessMigration),
            Box::new(McpHardeningMigration),
            Box::new(SyncFetchLimitMigration),
            Box::new(McpIntegrityMigration),
            Box::new(AttachmentPreviewMigration),
            Box::new(MailExperienceMigration),
            Box::new(ContactsDraftSchedulingMigration),
            Box::new(DraftAttachmentsMigration),
            Box::new(AiCalendarMigration),
            Box::new(AiAccessMigration),
        ]
    }
}

struct AiAccessMigration;

impl MigrationName for AiAccessMigration {
    fn name(&self) -> &str {
        "m20260728_000011_ai_access"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AiAccessMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE users ADD COLUMN ai_enabled BOOLEAN NOT NULL DEFAULT 0;

                CREATE TABLE auto_label_subscriptions (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    name VARCHAR(120) NOT NULL,
                    url TEXT NOT NULL,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    last_synced_at BIGINT,
                    last_error TEXT,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_auto_label_subscription_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(user_id, name)
                );
                CREATE INDEX idx_auto_label_subscriptions_user
                    ON auto_label_subscriptions(user_id, enabled, created_at);

                ALTER TABLE auto_label_rules ADD COLUMN source_subscription_id TEXT;
                CREATE INDEX idx_auto_label_rules_subscription
                    ON auto_label_rules(user_id, source_subscription_id, created_at);
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
                DROP INDEX IF EXISTS idx_auto_label_rules_subscription;
                ALTER TABLE auto_label_rules DROP COLUMN source_subscription_id;
                DROP INDEX IF EXISTS idx_auto_label_subscriptions_user;
                DROP TABLE IF EXISTS auto_label_subscriptions;
                ALTER TABLE users DROP COLUMN ai_enabled;
                "#,
            )
            .await?;
        Ok(())
    }
}

struct AiCalendarMigration;

impl MigrationName for AiCalendarMigration {
    fn name(&self) -> &str {
        "m20260728_000010_ai_calendar"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AiCalendarMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE ai_providers (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    name VARCHAR(120) NOT NULL,
                    provider_kind TEXT NOT NULL CHECK(provider_kind IN ('openai','claude','gemini')),
                    api_type TEXT NOT NULL CHECK(api_type IN ('chat','responses','messages','generateContent')),
                    model VARCHAR(160) NOT NULL,
                    base_url TEXT,
                    api_key_cipher TEXT,
                    proxy_kind TEXT NOT NULL DEFAULT 'direct' CHECK(proxy_kind IN ('direct','http','socks5')),
                    proxy_host TEXT,
                    proxy_port INTEGER,
                    proxy_username TEXT,
                    proxy_password_cipher TEXT,
                    is_default BOOLEAN NOT NULL DEFAULT 0,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_ai_provider_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(user_id, name)
                );
                CREATE INDEX idx_ai_providers_user_default
                    ON ai_providers(user_id, enabled, is_default, created_at);

                CREATE TABLE labels (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    name VARCHAR(80) NOT NULL,
                    color VARCHAR(40) NOT NULL,
                    is_auto BOOLEAN NOT NULL DEFAULT 0,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_label_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(user_id, name)
                );
                CREATE INDEX idx_labels_user_auto
                    ON labels(user_id, is_auto, name COLLATE NOCASE);

                CREATE TABLE message_labels (
                    message_id TEXT NOT NULL,
                    label_id TEXT NOT NULL,
                    user_id TEXT NOT NULL,
                    created_at BIGINT NOT NULL,
                    PRIMARY KEY(message_id, label_id),
                    CONSTRAINT fk_message_label_message FOREIGN KEY(message_id)
                        REFERENCES messages(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_message_label_label FOREIGN KEY(label_id)
                        REFERENCES labels(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_message_label_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE
                );
                CREATE INDEX idx_message_labels_user_label
                    ON message_labels(user_id, label_id, created_at);

                CREATE TABLE auto_label_rules (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    account_id TEXT,
                    provider_id TEXT,
                    name VARCHAR(120) NOT NULL,
                    label_ids_json TEXT NOT NULL DEFAULT '[]',
                    instructions TEXT NOT NULL DEFAULT '',
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    apply_automatically BOOLEAN NOT NULL DEFAULT 0,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_auto_label_rule_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_auto_label_rule_account FOREIGN KEY(account_id)
                        REFERENCES mail_accounts(id) ON UPDATE CASCADE ON DELETE SET NULL,
                    CONSTRAINT fk_auto_label_rule_provider FOREIGN KEY(provider_id)
                        REFERENCES ai_providers(id) ON UPDATE CASCADE ON DELETE SET NULL
                );
                CREATE INDEX idx_auto_label_rules_user
                    ON auto_label_rules(user_id, enabled, created_at);

                CREATE TABLE calendar_accounts (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    name VARCHAR(120) NOT NULL,
                    base_url TEXT NOT NULL,
                    username VARCHAR(320) NOT NULL,
                    password_cipher TEXT NOT NULL,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    last_synced_at BIGINT,
                    last_error TEXT,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_calendar_account_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(user_id, name)
                );
                CREATE INDEX idx_calendar_accounts_user
                    ON calendar_accounts(user_id, enabled, created_at);

                CREATE TABLE calendars (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    account_id TEXT NOT NULL,
                    display_name VARCHAR(160) NOT NULL,
                    color VARCHAR(40) NOT NULL,
                    remote_href TEXT NOT NULL,
                    sync_token TEXT,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_calendar_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_calendar_account FOREIGN KEY(account_id)
                        REFERENCES calendar_accounts(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(account_id, remote_href)
                );
                CREATE INDEX idx_calendars_user_enabled
                    ON calendars(user_id, enabled, display_name COLLATE NOCASE);

                CREATE TABLE calendar_events (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    calendar_id TEXT NOT NULL,
                    uid VARCHAR(255) NOT NULL,
                    summary TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    location TEXT NOT NULL DEFAULT '',
                    starts_at BIGINT NOT NULL,
                    ends_at BIGINT NOT NULL,
                    all_day BOOLEAN NOT NULL DEFAULT 0,
                    timezone TEXT,
                    remote_href TEXT,
                    etag TEXT,
                    ics TEXT NOT NULL DEFAULT '',
                    deleted BOOLEAN NOT NULL DEFAULT 0,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_calendar_event_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_calendar_event_calendar FOREIGN KEY(calendar_id)
                        REFERENCES calendars(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(calendar_id, uid)
                );
                CREATE INDEX idx_calendar_events_user_range
                    ON calendar_events(user_id, deleted, starts_at, ends_at);
                CREATE INDEX idx_calendar_events_calendar_href
                    ON calendar_events(calendar_id, remote_href);
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
                DROP INDEX IF EXISTS idx_calendar_events_calendar_href;
                DROP INDEX IF EXISTS idx_calendar_events_user_range;
                DROP TABLE IF EXISTS calendar_events;
                DROP INDEX IF EXISTS idx_calendars_user_enabled;
                DROP TABLE IF EXISTS calendars;
                DROP INDEX IF EXISTS idx_calendar_accounts_user;
                DROP TABLE IF EXISTS calendar_accounts;
                DROP INDEX IF EXISTS idx_auto_label_rules_user;
                DROP TABLE IF EXISTS auto_label_rules;
                DROP INDEX IF EXISTS idx_message_labels_user_label;
                DROP TABLE IF EXISTS message_labels;
                DROP INDEX IF EXISTS idx_labels_user_auto;
                DROP TABLE IF EXISTS labels;
                DROP INDEX IF EXISTS idx_ai_providers_user_default;
                DROP TABLE IF EXISTS ai_providers;
                "#,
            )
            .await?;
        Ok(())
    }
}

struct DraftAttachmentsMigration;

impl MigrationName for DraftAttachmentsMigration {
    fn name(&self) -> &str {
        "m20260728_000009_draft_attachments"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for DraftAttachmentsMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE email_drafts
                    ADD COLUMN attachments_json TEXT NOT NULL DEFAULT '[]';
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE email_drafts DROP COLUMN attachments_json;")
            .await?;
        Ok(())
    }
}

struct ContactsDraftSchedulingMigration;

impl MigrationName for ContactsDraftSchedulingMigration {
    fn name(&self) -> &str {
        "m20260727_000008_contacts_draft_scheduling"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for ContactsDraftSchedulingMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE contacts (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    display_name VARCHAR(160) NOT NULL,
                    email VARCHAR(254) NOT NULL,
                    notes TEXT NOT NULL DEFAULT '',
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_contact_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(user_id, email)
                );
                CREATE INDEX idx_contacts_user_name
                    ON contacts(user_id, display_name COLLATE NOCASE, email COLLATE NOCASE);

                ALTER TABLE email_drafts ADD COLUMN html_body TEXT;
                ALTER TABLE email_drafts ADD COLUMN signature_id TEXT
                    REFERENCES signatures(id) ON UPDATE CASCADE ON DELETE SET NULL;
                ALTER TABLE email_drafts ADD COLUMN apply_signature BOOLEAN NOT NULL DEFAULT 1;
                ALTER TABLE email_drafts ADD COLUMN scheduled_at BIGINT;
                ALTER TABLE email_drafts ADD COLUMN last_error TEXT;
                CREATE INDEX idx_email_drafts_scheduled
                    ON email_drafts(status, scheduled_at, updated_at);
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
                DROP INDEX IF EXISTS idx_email_drafts_scheduled;
                ALTER TABLE email_drafts DROP COLUMN last_error;
                ALTER TABLE email_drafts DROP COLUMN scheduled_at;
                ALTER TABLE email_drafts DROP COLUMN apply_signature;
                ALTER TABLE email_drafts DROP COLUMN signature_id;
                ALTER TABLE email_drafts DROP COLUMN html_body;
                DROP INDEX IF EXISTS idx_contacts_user_name;
                DROP TABLE IF EXISTS contacts;
                "#,
            )
            .await?;
        Ok(())
    }
}

struct MailExperienceMigration;

impl MigrationName for MailExperienceMigration {
    fn name(&self) -> &str {
        "m20260727_000007_mail_experience"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for MailExperienceMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE signatures (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    name VARCHAR(120) NOT NULL,
                    body_text TEXT NOT NULL,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_signature_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE
                );
                CREATE INDEX idx_signatures_user ON signatures(user_id, created_at);

                ALTER TABLE mail_accounts ADD COLUMN signature_id TEXT
                    REFERENCES signatures(id) ON UPDATE CASCADE ON DELETE SET NULL;

                ALTER TABLE messages ADD COLUMN cc_recipients_json TEXT NOT NULL DEFAULT '[]';
                ALTER TABLE messages ADD COLUMN thread_key TEXT NOT NULL DEFAULT '';
                ALTER TABLE messages ADD COLUMN raw_size BIGINT NOT NULL DEFAULT 0 CHECK(raw_size >= 0);
                ALTER TABLE messages ADD COLUMN is_promotional BOOLEAN NOT NULL DEFAULT 0;
                ALTER TABLE messages ADD COLUMN auto_response_allowed BOOLEAN NOT NULL DEFAULT 1;
                UPDATE messages SET thread_key = 'legacy:' || id WHERE thread_key = '';
                CREATE INDEX idx_messages_user_thread
                    ON messages(user_id, account_id, folder, thread_key, received_at);
                CREATE INDEX idx_messages_user_promotional
                    ON messages(user_id, folder, is_promotional, received_at);

                ALTER TABLE cleanup_rules ADD COLUMN position INTEGER NOT NULL DEFAULT 0;
                ALTER TABLE cleanup_rules ADD COLUMN match_mode TEXT NOT NULL DEFAULT 'all'
                    CHECK(match_mode IN ('all', 'any'));
                ALTER TABLE cleanup_rules ADD COLUMN conditions_json TEXT NOT NULL DEFAULT '[]';
                ALTER TABLE cleanup_rules ADD COLUMN actions_json TEXT NOT NULL DEFAULT '[]';
                ALTER TABLE cleanup_rules ADD COLUMN stop_processing BOOLEAN NOT NULL DEFAULT 0;
                UPDATE cleanup_rules SET position = CAST(created_at AS INTEGER);
                CREATE INDEX idx_cleanup_rules_order
                    ON cleanup_rules(user_id, enabled, position, created_at);
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
                DROP INDEX IF EXISTS idx_cleanup_rules_order;
                ALTER TABLE cleanup_rules DROP COLUMN stop_processing;
                ALTER TABLE cleanup_rules DROP COLUMN actions_json;
                ALTER TABLE cleanup_rules DROP COLUMN conditions_json;
                ALTER TABLE cleanup_rules DROP COLUMN match_mode;
                ALTER TABLE cleanup_rules DROP COLUMN position;

                DROP INDEX IF EXISTS idx_messages_user_promotional;
                DROP INDEX IF EXISTS idx_messages_user_thread;
                ALTER TABLE messages DROP COLUMN auto_response_allowed;
                ALTER TABLE messages DROP COLUMN is_promotional;
                ALTER TABLE messages DROP COLUMN raw_size;
                ALTER TABLE messages DROP COLUMN thread_key;
                ALTER TABLE messages DROP COLUMN cc_recipients_json;

                ALTER TABLE mail_accounts DROP COLUMN signature_id;
                DROP INDEX IF EXISTS idx_signatures_user;
                DROP TABLE IF EXISTS signatures;
                "#,
            )
            .await?;
        Ok(())
    }
}

struct AttachmentPreviewMigration;

impl MigrationName for AttachmentPreviewMigration {
    fn name(&self) -> &str {
        "m20260727_000006_attachment_preview"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for AttachmentPreviewMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE message_attachments (
                    id TEXT PRIMARY KEY NOT NULL,
                    message_id TEXT NOT NULL,
                    position INTEGER NOT NULL CHECK(position >= 0),
                    filename VARCHAR(255) NOT NULL,
                    content_type VARCHAR(127) NOT NULL,
                    size BIGINT NOT NULL CHECK(size >= 0),
                    content BLOB,
                    created_at BIGINT NOT NULL,
                    CONSTRAINT fk_message_attachment_message FOREIGN KEY(message_id)
                        REFERENCES messages(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    UNIQUE(message_id, position)
                );
                CREATE INDEX idx_message_attachments_message
                    ON message_attachments(message_id, position);
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
                DROP INDEX IF EXISTS idx_message_attachments_message;
                DROP TABLE IF EXISTS message_attachments;
                "#,
            )
            .await?;
        Ok(())
    }
}

struct McpIntegrityMigration;

impl MigrationName for McpIntegrityMigration {
    fn name(&self) -> &str {
        "m20260727_000005_mcp_integrity"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for McpIntegrityMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE messages ADD COLUMN uid_validity BIGINT;")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE messages DROP COLUMN uid_validity;")
            .await?;
        Ok(())
    }
}

struct SyncFetchLimitMigration;

impl MigrationName for SyncFetchLimitMigration {
    fn name(&self) -> &str {
        "m20260727_000004_sync_fetch_limit"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for SyncFetchLimitMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE mail_settings
                    ADD COLUMN sync_fetch_limit INTEGER DEFAULT 50
                    CHECK(sync_fetch_limit IS NULL OR sync_fetch_limit BETWEEN 1 AND 10000);
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("ALTER TABLE mail_settings DROP COLUMN sync_fetch_limit;")
            .await?;
        Ok(())
    }
}

struct McpHardeningMigration;

impl MigrationName for McpHardeningMigration {
    fn name(&self) -> &str {
        "m20260727_000003_mcp_hardening"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for McpHardeningMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE messages ADD COLUMN reply_to_email TEXT;
                ALTER TABLE messages ADD COLUMN references_header TEXT NOT NULL DEFAULT '[]';
                ALTER TABLE email_drafts ADD COLUMN status TEXT NOT NULL DEFAULT 'draft'
                    CHECK(status IN ('draft', 'sending', 'ambiguous', 'sent'));
                CREATE INDEX idx_email_drafts_send_status
                    ON email_drafts(user_id, id, status);
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
                DROP INDEX IF EXISTS idx_email_drafts_send_status;
                ALTER TABLE email_drafts DROP COLUMN status;
                ALTER TABLE messages DROP COLUMN references_header;
                ALTER TABLE messages DROP COLUMN reply_to_email;
                "#,
            )
            .await?;
        Ok(())
    }
}

struct McpAccessMigration;

impl MigrationName for McpAccessMigration {
    fn name(&self) -> &str {
        "m20260727_000002_mcp_access"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for McpAccessMigration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TABLE mcp_tokens (
                    user_id TEXT PRIMARY KEY NOT NULL,
                    token_id TEXT NOT NULL UNIQUE,
                    token_digest BLOB NOT NULL,
                    allow_delete BOOLEAN NOT NULL DEFAULT 0,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    last_used_at BIGINT,
                    CONSTRAINT fk_mcp_token_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE
                );

                CREATE TABLE email_drafts (
                    id TEXT PRIMARY KEY NOT NULL,
                    user_id TEXT NOT NULL,
                    account_id TEXT NOT NULL,
                    reply_to_message_id TEXT,
                    to_json TEXT NOT NULL,
                    cc_json TEXT NOT NULL,
                    bcc_json TEXT NOT NULL,
                    subject TEXT NOT NULL,
                    text_body TEXT NOT NULL,
                    in_reply_to TEXT,
                    references_header TEXT,
                    created_at BIGINT NOT NULL,
                    updated_at BIGINT NOT NULL,
                    CONSTRAINT fk_email_draft_user FOREIGN KEY(user_id)
                        REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_email_draft_account FOREIGN KEY(account_id)
                        REFERENCES mail_accounts(id) ON UPDATE CASCADE ON DELETE CASCADE,
                    CONSTRAINT fk_email_draft_reply_message FOREIGN KEY(reply_to_message_id)
                        REFERENCES messages(id) ON UPDATE CASCADE ON DELETE SET NULL
                );
                CREATE INDEX idx_email_drafts_user
                    ON email_drafts(user_id, updated_at);
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
                DROP TABLE IF EXISTS email_drafts;
                DROP TABLE IF EXISTS mcp_tokens;
                "#,
            )
            .await?;
        Ok(())
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
