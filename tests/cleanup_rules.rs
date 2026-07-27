use meowmail::{
    AppState,
    accounts::{AccountInput, AccountRepository, ConnectionSecurity, ProxyInput, ServerConfig},
    cleanup::{
        CleanupRepository, CleanupRule, CleanupRuleInput, MailSettings, RuleAction, RuleActionKind,
        RuleCondition, RuleField, RuleMatchMode, RuleOperator,
    },
    config::Config,
    error::AppError,
    mail::ParsedMail,
    messages::{MessageFilter, MessageRepository, NewMessage},
    users::UserRepository,
};
use uuid::Uuid;

#[tokio::test]
async fn cleanup_settings_and_rules_are_scoped_to_their_user() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let users = UserRepository::new(state.db.clone());
    let admin = users
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let other = users
        .provision_oidc(
            "https://issuer.example",
            "cleanup-user",
            Some("cleanup@example.com"),
            Some("cleanup-user"),
            false,
        )
        .await
        .unwrap();
    let accounts = AccountRepository::new(state.db.clone(), state.vault.clone());
    let other_account = accounts
        .create(other.id, account("Other", "other@example.com"))
        .await
        .unwrap();
    let cleanup = CleanupRepository::new(state.db.clone());

    cleanup
        .update_settings(
            admin.id,
            MailSettings {
                keep_local_after_server_delete: false,
                sync_fetch_limit: None,
            },
        )
        .await
        .unwrap();
    assert!(
        !cleanup
            .settings(admin.id)
            .await
            .unwrap()
            .keep_local_after_server_delete
    );
    assert!(
        cleanup
            .settings(other.id)
            .await
            .unwrap()
            .keep_local_after_server_delete
    );
    assert_eq!(
        cleanup.settings(admin.id).await.unwrap().sync_fetch_limit,
        None
    );
    assert_eq!(
        cleanup.settings(other.id).await.unwrap().sync_fetch_limit,
        Some(50)
    );

    let invalid = cleanup
        .update_settings(
            admin.id,
            MailSettings {
                keep_local_after_server_delete: false,
                sync_fetch_limit: Some(10_001),
            },
        )
        .await;
    assert!(matches!(invalid, Err(AppError::Validation(_))));

    let result = cleanup
        .create(
            admin.id,
            CleanupRuleInput {
                account_id: Some(other_account.id),
                name: "Cross-user rule".into(),
                match_mode: RuleMatchMode::All,
                conditions: Vec::new(),
                actions: Vec::new(),
                position: None,
                stop_processing: false,
                sender_contains: Some("alerts".into()),
                subject_contains: None,
                body_contains: None,
                older_than_days: None,
                delete_from_server: false,
                enabled: true,
            },
        )
        .await;
    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn server_cleanup_keeps_local_mail_until_server_success() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let admin = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(admin.id, account("Work", "me@example.com"))
        .await
        .unwrap();
    let messages = MessageRepository::new(state.db.clone());
    messages
        .insert_if_new(
            admin.id,
            &account,
            NewMessage {
                folder: "INBOX".into(),
                uid: 42,
                uid_validity: Some(1001),
                mail: ParsedMail {
                    message_id: Some("cleanup@example.com".into()),
                    reply_to_email: None,
                    references: Vec::new(),
                    sender_name: Some("Build Bot".into()),
                    sender_email: "build@example.com".into(),
                    recipients: vec!["me@example.com".into()],
                    cc_recipients: Vec::new(),
                    subject: "Delete after server success".into(),
                    thread_key: "delete after server success".into(),
                    preview: "cleanup".into(),
                    body_text: "cleanup".into(),
                    body_html: None,
                    received_at: 2_000_000_000,
                    attachment_count: 0,
                    attachments: Vec::new(),
                    raw_size: 128,
                    is_promotional: false,
                    auto_forward_allowed: true,
                    auto_response_allowed: true,
                },
                is_read: false,
                is_starred: false,
            },
        )
        .await
        .unwrap();
    let cached = CleanupRepository::new(state.db.clone())
        .apply_cached_rules(
            admin.id,
            account.id,
            Some(2002),
            &[CleanupRule {
                id: Uuid::new_v4(),
                account_id: Some(account.id),
                name: "Server cleanup".into(),
                match_mode: RuleMatchMode::All,
                conditions: vec![RuleCondition {
                    field: RuleField::Subject,
                    operator: RuleOperator::ContainsAny,
                    values: vec!["server success".into()],
                }],
                actions: vec![RuleAction {
                    kind: RuleActionKind::DeleteServer,
                    value: None,
                }],
                position: 0,
                stop_processing: false,
                sender_contains: None,
                subject_contains: None,
                body_contains: None,
                older_than_days: None,
                delete_from_server: true,
                enabled: true,
                created_at: 2_000_000_000,
                updated_at: 2_000_000_000,
            }],
            2_000_000_000,
        )
        .await
        .unwrap();
    assert!(cached.server_uids.is_empty());
    assert!(cached.server_message_ids.is_empty());
    let filter = MessageFilter {
        account_id: Some(account.id),
        folder: "INBOX".into(),
        unread: false,
        starred: false,
        has_attachment: false,
        query: None,
        limit: 10,
    };
    assert_eq!(
        messages.list(admin.id, filter.clone()).await.unwrap().len(),
        1
    );

    let cached = CleanupRepository::new(state.db.clone())
        .apply_cached_rules(
            admin.id,
            account.id,
            Some(1001),
            &[CleanupRule {
                id: Uuid::new_v4(),
                account_id: Some(account.id),
                name: "Server cleanup".into(),
                match_mode: RuleMatchMode::All,
                conditions: vec![RuleCondition {
                    field: RuleField::Subject,
                    operator: RuleOperator::ContainsAny,
                    values: vec!["server success".into()],
                }],
                actions: vec![RuleAction {
                    kind: RuleActionKind::DeleteServer,
                    value: None,
                }],
                position: 0,
                stop_processing: false,
                sender_contains: None,
                subject_contains: None,
                body_contains: None,
                older_than_days: None,
                delete_from_server: true,
                enabled: true,
                created_at: 2_000_000_000,
                updated_at: 2_000_000_000,
            }],
            2_000_000_000,
        )
        .await
        .unwrap();
    assert_eq!(cached.server_uids, vec![42]);
    assert_eq!(cached.server_message_ids.len(), 1);
    assert_eq!(
        messages.list(admin.id, filter.clone()).await.unwrap().len(),
        1
    );

    CleanupRepository::new(state.db.clone())
        .delete_cached_after_server_success(admin.id, &cached.server_message_ids)
        .await
        .unwrap();
    assert!(messages.list(admin.id, filter).await.unwrap().is_empty());
}

#[test]
fn legacy_mail_settings_default_to_recent_fifty_messages() {
    let settings: MailSettings = serde_json::from_value(serde_json::json!({
        "keepLocalAfterServerDelete": true
    }))
    .unwrap();
    assert_eq!(settings.sync_fetch_limit, Some(50));
}

#[test]
fn cleanup_rule_matches_sender_subject_body_and_age_together() {
    let now = 2_000_000_000;
    let rule = CleanupRule {
        id: Uuid::new_v4(),
        account_id: None,
        name: "Old build reports".into(),
        match_mode: RuleMatchMode::All,
        conditions: vec![
            RuleCondition {
                field: RuleField::Sender,
                operator: RuleOperator::ContainsAny,
                values: vec!["ci@example.com".into()],
            },
            RuleCondition {
                field: RuleField::Subject,
                operator: RuleOperator::ContainsAny,
                values: vec!["build".into()],
            },
            RuleCondition {
                field: RuleField::Body,
                operator: RuleOperator::ContainsAny,
                values: vec!["artifact".into()],
            },
            RuleCondition {
                field: RuleField::AgeDays,
                operator: RuleOperator::GreaterThan,
                values: vec!["30".into()],
            },
        ],
        actions: vec![RuleAction {
            kind: RuleActionKind::DeleteLocal,
            value: None,
        }],
        position: 0,
        stop_processing: false,
        sender_contains: Some("ci@example.com".into()),
        subject_contains: Some("build".into()),
        body_contains: Some("artifact".into()),
        older_than_days: Some(30),
        delete_from_server: false,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    let mail = ParsedMail {
        message_id: None,
        reply_to_email: None,
        references: Vec::new(),
        sender_name: Some("CI".into()),
        sender_email: "ci@example.com".into(),
        recipients: vec!["me@example.com".into()],
        cc_recipients: Vec::new(),
        subject: "Build completed".into(),
        thread_key: "build completed".into(),
        preview: "Artifact ready".into(),
        body_text: "The artifact can be downloaded.".into(),
        body_html: None,
        received_at: now - 31 * 86_400,
        attachment_count: 0,
        attachments: Vec::new(),
        raw_size: 128,
        is_promotional: false,
        auto_forward_allowed: true,
        auto_response_allowed: true,
    };
    assert!(CleanupRepository::match_new_mail(std::slice::from_ref(&rule), &mail, now).matched);

    let recent = ParsedMail {
        received_at: now - 2 * 86_400,
        ..mail
    };
    assert!(!CleanupRepository::match_new_mail(&[rule], &recent, now).matched);
}

#[test]
fn any_match_mode_and_stop_processing_preserve_rule_order() {
    let now = 2_000_000_000;
    let mail = ParsedMail {
        message_id: None,
        reply_to_email: None,
        references: Vec::new(),
        sender_name: Some("Release Bot".into()),
        sender_email: "release@example.com".into(),
        recipients: vec!["me@example.com".into()],
        cc_recipients: Vec::new(),
        subject: "Deployment completed".into(),
        thread_key: "deployment completed".into(),
        preview: "Production is ready".into(),
        body_text: "Production is ready".into(),
        body_html: None,
        received_at: now,
        attachment_count: 0,
        attachments: Vec::new(),
        raw_size: 128,
        is_promotional: false,
        auto_forward_allowed: true,
        auto_response_allowed: true,
    };
    let first = CleanupRule {
        id: Uuid::new_v4(),
        account_id: None,
        name: "Star deployments".into(),
        match_mode: RuleMatchMode::Any,
        conditions: vec![
            RuleCondition {
                field: RuleField::Sender,
                operator: RuleOperator::Equals,
                values: vec!["nobody@example.com".into()],
            },
            RuleCondition {
                field: RuleField::Subject,
                operator: RuleOperator::ContainsAny,
                values: vec!["deployment".into()],
            },
        ],
        actions: vec![RuleAction {
            kind: RuleActionKind::Star,
            value: None,
        }],
        position: 0,
        stop_processing: true,
        sender_contains: None,
        subject_contains: None,
        body_contains: None,
        older_than_days: None,
        delete_from_server: false,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    let second = CleanupRule {
        id: Uuid::new_v4(),
        account_id: None,
        name: "Delete everything".into(),
        match_mode: RuleMatchMode::All,
        conditions: vec![RuleCondition {
            field: RuleField::Subject,
            operator: RuleOperator::ContainsAny,
            values: vec!["deployment".into()],
        }],
        actions: vec![RuleAction {
            kind: RuleActionKind::DeleteLocal,
            value: None,
        }],
        position: 1,
        stop_processing: false,
        sender_contains: None,
        subject_contains: None,
        body_contains: None,
        older_than_days: None,
        delete_from_server: false,
        enabled: true,
        created_at: now,
        updated_at: now,
    };

    let outcome = CleanupRepository::match_new_mail(&[first, second], &mail, now);
    assert!(outcome.matched);
    assert_eq!(outcome.is_starred, Some(true));
    assert!(!outcome.delete_local);
}

fn account(display_name: &str, email: &str) -> AccountInput {
    AccountInput {
        display_name: display_name.into(),
        email: email.into(),
        username: email.into(),
        password: Some("app-password".into()),
        imap: ServerConfig {
            host: "imap.example.com".into(),
            port: 993,
            security: ConnectionSecurity::Tls,
        },
        smtp: ServerConfig {
            host: "smtp.example.com".into(),
            port: 587,
            security: ConnectionSecurity::Starttls,
        },
        proxy: ProxyInput::default(),
        is_default: true,
    }
}
