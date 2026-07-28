use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use meowmail::{
    AppState,
    accounts::{AccountInput, AccountRepository, ConnectionSecurity, ProxyInput, ServerConfig},
    build_router,
    config::Config,
    db::entities::user,
    mail::parse_message,
    messages::{MessageRepository, NewMessage},
    security::hash_secret,
    users::UserRepository,
};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn attachment_metadata_and_content_are_scoped_to_the_message_owner() {
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
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(
            owner.id,
            AccountInput {
                display_name: "Work".into(),
                email: "me@example.com".into(),
                username: "me@example.com".into(),
                password: Some("app-password".into()),
                imap: ServerConfig {
                    host: "imap.example.com".into(),
                    port: 993,
                    security: ConnectionSecurity::Tls,
                },
                smtp: ServerConfig {
                    host: "smtp.example.com".into(),
                    port: 465,
                    security: ConnectionSecurity::Tls,
                },
                proxy: ProxyInput::default(),
                is_default: true,
            },
        )
        .await
        .unwrap();
    let repository = MessageRepository::new(state.db.clone());
    let initial = parse_message(
        b"From: Alice <alice@example.com>\r\nTo: me@example.com\r\nSubject: Handbook\r\n\r\nPlease review.\r\n",
        2_000_000_000,
    )
    .unwrap();
    repository
        .insert_if_new(
            owner.id,
            &account,
            NewMessage {
                folder: "INBOX".into(),
                uid: 1,
                uid_validity: Some(1001),
                mail: initial,
                is_read: false,
                is_starred: false,
            },
        )
        .await
        .unwrap();
    let parsed = parse_message(
        b"From: Alice <alice@example.com>\r\n\
To: me@example.com\r\n\
Subject: Handbook\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=meow\r\n\
\r\n\
--meow\r\n\
Content-Type: text/plain\r\n\
\r\n\
Please review.\r\n\
--meow\r\n\
Content-Type: application/pdf; name=\"handbook.pdf\"\r\n\
Content-Disposition: attachment; filename=\"handbook.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
JVBERi0xLjQK\r\n\
--meow--\r\n",
        2_000_000_000,
    )
    .unwrap();
    let parsed_again = parsed.clone();
    repository
        .insert_if_new(
            owner.id,
            &account,
            NewMessage {
                folder: "INBOX".into(),
                uid: 1,
                uid_validity: Some(1001),
                mail: parsed,
                is_read: false,
                is_starred: false,
            },
        )
        .await
        .unwrap();
    let message = MessageRepository::new(state.db.clone())
        .list(
            owner.id,
            meowmail::messages::MessageFilter {
                account_id: Some(account.id),
                folder: "INBOX".into(),
                unread: false,
                starred: false,
                has_attachment: false,
                query: None,
                limit: 10,
            },
        )
        .await
        .unwrap()
        .remove(0);
    create_local_user(&state, "member", "member password").await;
    let app = build_router(state);
    let owner_cookie = login(&app, "admin", "correct horse battery staple").await;
    let member_cookie = login(&app, "member", "member password").await;

    let detail = app
        .clone()
        .oneshot(get_request(
            &format!("/api/v1/messages/{}", message.id),
            &owner_cookie,
        ))
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail: Value =
        serde_json::from_slice(&detail.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let attachment = &detail["attachments"][0];
    assert_eq!(attachment["filename"], "handbook.pdf");
    assert_eq!(attachment["contentType"], "application/pdf");
    assert_eq!(attachment["size"], 9);
    assert_eq!(attachment["available"], true);
    let attachment_id = attachment["id"].as_str().unwrap().to_owned();
    repository
        .insert_if_new(
            owner.id,
            &account,
            NewMessage {
                folder: "INBOX".into(),
                uid: 1,
                uid_validity: Some(1001),
                mail: parsed_again,
                is_read: false,
                is_starred: false,
            },
        )
        .await
        .unwrap();
    let detail_after_sync = app
        .clone()
        .oneshot(get_request(
            &format!("/api/v1/messages/{}", message.id),
            &owner_cookie,
        ))
        .await
        .unwrap();
    assert_eq!(detail_after_sync.status(), StatusCode::OK);
    let detail_after_sync: Value = serde_json::from_slice(
        &detail_after_sync
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
    )
    .unwrap();
    assert_eq!(detail_after_sync["attachments"][0]["id"], attachment_id);
    let path = format!(
        "/api/v1/messages/{}/attachments/{attachment_id}",
        message.id
    );

    let content = app
        .clone()
        .oneshot(get_request(&path, &owner_cookie))
        .await
        .unwrap();
    assert_eq!(content.status(), StatusCode::OK);
    assert_eq!(content.headers()[header::CONTENT_TYPE], "application/pdf");
    assert_eq!(
        content.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert!(
        content.headers()[header::CONTENT_DISPOSITION]
            .to_str()
            .unwrap()
            .starts_with("inline;")
    );
    assert_eq!(
        content.into_body().collect().await.unwrap().to_bytes(),
        b"%PDF-1.4\n".as_slice()
    );

    let forbidden = app
        .oneshot(get_request(&path, &member_cookie))
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::NOT_FOUND);
}

struct LoginSession {
    cookie: String,
}

async fn login(app: &Router, username: &str, password: &str) -> LoginSession {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "username": username, "password": password }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    LoginSession {
        cookie: response.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned(),
    }
}

fn get_request(path: &str, session: &LoginSession) -> Request<Body> {
    Request::builder()
        .uri(path)
        .header(header::COOKIE, &session.cookie)
        .body(Body::empty())
        .unwrap()
}

async fn create_local_user(state: &AppState, username: &str, password: &str) {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    user::ActiveModel {
        id: Set(Uuid::new_v4().to_string()),
        username: Set(username.into()),
        nickname: Set(username.into()),
        email: Set(None),
        role: Set("user".into()),
        password_hash: Set(Some(hash_secret(password).unwrap())),
        pin_hash: Set(None),
        avatar_mime: Set(None),
        avatar_data: Set(None),
        ai_enabled: Set(false),
        auto_lock_minutes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
}
