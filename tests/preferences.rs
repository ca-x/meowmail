use meowmail::{
    AppState,
    accounts::{
        AccountIdentityInput, AccountInput, AccountRepository, ConnectionSecurity, ProxyInput,
        ServerConfig,
    },
    config::Config,
    error::AppError,
    preferences::{ListDensity, PreferencesRepository, ReadingMode, SignatureInput},
    users::UserRepository,
};

#[tokio::test]
async fn mail_preferences_and_signatures_are_isolated_per_user() {
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
            "preferences-user",
            Some("preferences@example.com"),
            Some("Preferences User"),
            false,
        )
        .await
        .unwrap();
    let preferences = PreferencesRepository::new(state.db.clone());

    let mut admin_preferences = preferences.mail(admin.id).await.unwrap();
    admin_preferences.reading_mode = ReadingMode::List;
    admin_preferences.list_density = ListDensity::Compact;
    admin_preferences.compose_font_size = 16;
    preferences
        .update_mail(admin.id, admin_preferences.clone())
        .await
        .unwrap();

    assert_eq!(preferences.mail(admin.id).await.unwrap(), admin_preferences);
    assert_eq!(
        preferences.mail(other.id).await.unwrap().reading_mode,
        ReadingMode::Preview
    );

    let admin_signature = preferences
        .create_signature(
            admin.id,
            SignatureInput {
                name: " Work ".into(),
                body_text: " Meowmail Team ".into(),
            },
        )
        .await
        .unwrap();
    let other_signature = preferences
        .create_signature(
            other.id,
            SignatureInput {
                name: "Work".into(),
                body_text: "Other Team".into(),
            },
        )
        .await
        .unwrap();

    assert_eq!(admin_signature.name, "Work");
    assert_eq!(admin_signature.body_text, "Meowmail Team");
    assert_eq!(
        preferences.list_signatures(admin.id).await.unwrap().len(),
        1
    );
    assert_eq!(
        preferences.list_signatures(other.id).await.unwrap().len(),
        1
    );
    assert!(matches!(
        preferences
            .update_signature(
                other.id,
                admin_signature.id,
                SignatureInput {
                    name: "Stolen".into(),
                    body_text: "Not allowed".into(),
                },
            )
            .await,
        Err(AppError::NotFound)
    ));

    let accounts = AccountRepository::new(state.db.clone(), state.vault.clone());
    let account = accounts
        .create(admin.id, account("Work", "admin@example.com"))
        .await
        .unwrap();
    assert!(matches!(
        accounts
            .update_identity(
                admin.id,
                account.id,
                AccountIdentityInput {
                    display_name: "Admin".into(),
                    signature_id: Some(other_signature.id),
                    is_default: true,
                },
            )
            .await,
        Err(AppError::Validation(_))
    ));

    let updated = accounts
        .update_identity(
            admin.id,
            account.id,
            AccountIdentityInput {
                display_name: "Admin".into(),
                signature_id: Some(admin_signature.id),
                is_default: true,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.signature_id, Some(admin_signature.id));

    preferences
        .delete_signature(admin.id, admin_signature.id)
        .await
        .unwrap();
    assert_eq!(
        accounts
            .get(admin.id, account.id)
            .await
            .unwrap()
            .signature_id,
        None
    );
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
