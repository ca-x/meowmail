use meowmail::{
    AppState,
    config::Config,
    contacts::{ContactInput, ContactRepository},
    users::UserRepository,
};

#[tokio::test]
async fn contact_search_supports_pinyin_and_english_initials() {
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
    let user = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let contacts = ContactRepository::new(state.db);

    contacts
        .create(
            user.id,
            ContactInput {
                display_name: "张三".into(),
                email: "zhangsan@example.com".into(),
                notes: "设计团队".into(),
            },
        )
        .await
        .unwrap();
    contacts
        .create(
            user.id,
            ContactInput {
                display_name: "John Smith".into(),
                email: "john@example.com".into(),
                notes: "Sales".into(),
            },
        )
        .await
        .unwrap();

    assert_eq!(
        contacts.list(user.id, Some("zs".into()), 10).await.unwrap()[0].display_name,
        "张三"
    );
    assert_eq!(
        contacts
            .list(user.id, Some("zhangsan".into()), 10)
            .await
            .unwrap()[0]
            .display_name,
        "张三"
    );
    assert_eq!(
        contacts.list(user.id, Some("js".into()), 10).await.unwrap()[0].display_name,
        "John Smith"
    );
    assert_eq!(
        contacts
            .list(user.id, Some("设计".into()), 10)
            .await
            .unwrap()[0]
            .display_name,
        "张三"
    );
}
