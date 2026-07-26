use meowmail::{
    AppState,
    accounts::{AccountInput, AccountRepository, ConnectionSecurity, ProxyInput, ServerConfig},
    cleanup::{CleanupRepository, CleanupRule, CleanupRuleInput, MailSettings},
    config::Config,
    error::AppError,
    mail::ParsedMail,
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

    let result = cleanup
        .create(
            admin.id,
            CleanupRuleInput {
                account_id: Some(other_account.id),
                name: "Cross-user rule".into(),
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

#[test]
fn cleanup_rule_matches_sender_subject_body_and_age_together() {
    let now = 2_000_000_000;
    let rule = CleanupRule {
        id: Uuid::new_v4(),
        account_id: None,
        name: "Old build reports".into(),
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
        sender_name: Some("CI".into()),
        sender_email: "ci@example.com".into(),
        recipients: vec!["me@example.com".into()],
        subject: "Build completed".into(),
        preview: "Artifact ready".into(),
        body_text: "The artifact can be downloaded.".into(),
        body_html: None,
        received_at: now - 31 * 86_400,
        attachment_count: 0,
    };
    assert!(CleanupRepository::match_new_mail(std::slice::from_ref(&rule), &mail, now).is_some());

    let recent = ParsedMail {
        received_at: now - 2 * 86_400,
        ..mail
    };
    assert!(CleanupRepository::match_new_mail(&[rule], &recent, now).is_none());
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
