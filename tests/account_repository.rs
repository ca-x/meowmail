use meowmail::{
    AppState,
    accounts::{
        AccountInput, AccountRepository, ConnectionSecurity, ProxyInput, ProxyKind, ServerConfig,
    },
    config::Config,
    db::entities::mail_account,
    error::AppError,
    mail::parse_message,
    messages::{MessageFilter, MessageRepository, NewMessage},
    users::UserRepository,
};
use sea_orm::EntityTrait;

#[tokio::test]
async fn multiple_mail_accounts_are_isolated_and_secrets_are_encrypted() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new("a secure local pin".into(), directory.path().to_path_buf()).unwrap(),
    )
    .await
    .unwrap();
    let users = UserRepository::new(state.db.clone());
    let owner = users
        .authenticate_local("admin", "a secure local pin")
        .await
        .unwrap();
    let other = users
        .provision_oidc(
            "https://issuer.example",
            "other-user",
            Some("other@example.com"),
            Some("other"),
            false,
        )
        .await
        .unwrap();
    let repository = AccountRepository::new(state.db.clone(), state.vault.clone());
    let first = repository
        .create(owner.id, account("Work", "work@example.com", true))
        .await
        .unwrap();
    let second = repository
        .create(owner.id, account("Personal", "me@example.net", false))
        .await
        .unwrap();
    repository
        .create(other.id, account("Other work", "work@example.com", true))
        .await
        .unwrap();

    let accounts = repository.list(owner.id).await.unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(
        accounts.iter().filter(|account| account.is_default).count(),
        1
    );
    let stored = mail_account::Entity::find_by_id(first.id.to_string())
        .one(state.db.connection())
        .await
        .unwrap()
        .unwrap();
    assert_ne!(stored.password_cipher, "app-password");
    assert_ne!(
        stored.proxy_password_cipher.as_deref(),
        Some("proxy-secret")
    );

    assert!(matches!(
        repository.get(other.id, first.id).await,
        Err(AppError::NotFound)
    ));
    assert_eq!(repository.list(other.id).await.unwrap().len(), 1);

    repository.delete(owner.id, first.id).await.unwrap();
    let remaining = repository.get(owner.id, second.id).await.unwrap();
    assert!(remaining.is_default);
}

#[tokio::test]
async fn changing_imap_identity_clears_cached_messages() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new("a secure local pin".into(), directory.path().to_path_buf()).unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "a secure local pin")
        .await
        .unwrap();
    let accounts = AccountRepository::new(state.db.clone(), state.vault.clone());
    let saved_account = accounts
        .create(owner.id, account("Work", "work@example.com", true))
        .await
        .unwrap();
    let messages = MessageRepository::new(state.db.clone());
    messages
        .insert_if_new(
            owner.id,
            &saved_account,
            NewMessage {
                folder: "INBOX".into(),
                uid: 42,
                uid_validity: Some(1001),
                mail: parse_message(
                    b"From: Alice <alice@example.com>\r\nTo: work@example.com\r\nSubject: Old mailbox\r\n\r\nBody\r\n",
                    2_000_000_000,
                )
                .unwrap(),
                is_read: false,
                is_starred: false,
            },
        )
        .await
        .unwrap();

    let mut changed = account("Work", "new-work@example.com", true);
    changed.imap.host = "imap.new.example.com".into();
    changed.password = None;
    accounts
        .update(owner.id, saved_account.id, changed)
        .await
        .unwrap();

    let cached = messages
        .list(
            owner.id,
            MessageFilter {
                account_id: Some(saved_account.id),
                folder: "INBOX".into(),
                unread: false,
                starred: false,
                has_attachment: false,
                query: None,
                limit: 10,
            },
        )
        .await
        .unwrap();
    assert!(cached.is_empty());
}

fn account(display_name: &str, email: &str, is_default: bool) -> AccountInput {
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
        proxy: ProxyInput {
            kind: ProxyKind::Socks5,
            host: Some("127.0.0.1".into()),
            port: Some(1080),
            username: Some("proxy-user".into()),
            password: Some("proxy-secret".into()),
        },
        is_default,
    }
}
