use meowmail::{
    AppState,
    accounts::{
        AccountInput, AccountRepository, ConnectionSecurity, ProxyInput, ProxyKind, ServerConfig,
    },
    config::Config,
    db::entities::mail_account,
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
    let repository = AccountRepository::new(state.db.clone(), state.vault.clone());
    let first = repository
        .create(account("Work", "work@example.com", true))
        .await
        .unwrap();
    let second = repository
        .create(account("Personal", "me@example.net", false))
        .await
        .unwrap();

    let accounts = repository.list().await.unwrap();
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

    repository.delete(first.id).await.unwrap();
    let remaining = repository.get(second.id).await.unwrap();
    assert!(remaining.is_default);
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
