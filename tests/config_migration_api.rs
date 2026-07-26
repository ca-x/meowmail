use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use meowmail::{
    AppState, build_router,
    config::Config,
    db::entities::{mail_setting, notification_setting, user},
    security::{decrypt_archive, hash_secret},
};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn administrator_can_choose_personal_or_all_user_exports() {
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
    create_local_user(&state, "member", "member password", "user").await;
    let app = build_router(state);
    let admin = login(&app, "admin", "correct horse battery staple").await;

    let personal = export_archive(&app, &admin, "mine").await;
    assert_eq!(personal["scope"], "mine");
    let payload = decrypt_payload(&personal, "migration passphrase");
    assert_eq!(payload["scope"], "mine");
    assert_eq!(payload["users"].as_array().unwrap().len(), 1);
    assert!(payload["users"][0]["auth"].is_null());

    let all_users = export_archive(&app, &admin, "allUsers").await;
    assert_eq!(all_users["scope"], "allUsers");
    let payload = decrypt_payload(&all_users, "migration passphrase");
    assert_eq!(payload["users"].as_array().unwrap().len(), 2);
    assert!(payload["users"].as_array().unwrap().iter().all(|entry| {
        entry["auth"]["username"].is_string() && entry["auth"]["role"].is_string()
    }));
}

#[tokio::test]
async fn ordinary_user_cannot_export_or_import_all_users() {
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
    create_local_user(&state, "member", "member password", "user").await;
    let app = build_router(state);
    let admin = login(&app, "admin", "correct horse battery staple").await;
    let member = login(&app, "member", "member password").await;
    let all_users = export_archive(&app, &admin, "allUsers").await;

    let rejected_export = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users/migration/export",
            &member,
            json!({
                "passphrase": "migration passphrase",
                "scope": "allUsers",
                "sections": sections(),
            }),
        ))
        .await
        .unwrap();
    assert_eq!(rejected_export.status(), StatusCode::FORBIDDEN);

    let rejected_import = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users/migration/import",
            &member,
            json!({
                "passphrase": "migration passphrase",
                "sections": sections(),
                "archive": all_users,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(rejected_import.status(), StatusCode::FORBIDDEN);
}

struct LoginSession {
    cookie: String,
    csrf: String,
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
                    json!({ "username": username, "password": password }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    LoginSession {
        cookie,
        csrf: body["csrfToken"].as_str().unwrap().to_owned(),
    }
}

async fn export_archive(app: &Router, session: &LoginSession, scope: &str) -> Value {
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users/migration/export",
            session,
            json!({
                "passphrase": "migration passphrase",
                "scope": scope,
                "sections": sections(),
            }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

fn json_request(method: &str, path: &str, session: &LoginSession, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &session.cookie)
        .header("x-csrf-token", &session.csrf)
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn sections() -> Value {
    json!({
        "profile": true,
        "mailAccounts": true,
        "notifications": true,
        "cleanup": true,
    })
}

fn decrypt_payload(archive: &Value, passphrase: &str) -> Value {
    let plaintext =
        decrypt_archive(passphrase, archive["encryptedData"].as_str().unwrap()).unwrap();
    serde_json::from_slice(&plaintext).unwrap()
}

async fn create_local_user(state: &AppState, username: &str, password: &str, role: &str) {
    let id = Uuid::new_v4();
    let now = OffsetDateTime::now_utc().unix_timestamp();
    user::ActiveModel {
        id: Set(id.to_string()),
        username: Set(username.into()),
        nickname: Set(username.into()),
        email: Set(None),
        role: Set(role.into()),
        password_hash: Set(Some(hash_secret(password).unwrap())),
        pin_hash: Set(None),
        avatar_mime: Set(None),
        avatar_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
    notification_setting::ActiveModel {
        user_id: Set(id.to_string()),
        enabled: Set(false),
        message_template: Set("[{account}] {sender}: {subject}".into()),
        command_template: Set(None),
        http_url: Set(None),
        updated_at: Set(now),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
    mail_setting::ActiveModel {
        user_id: Set(id.to_string()),
        keep_local_after_server_delete: Set(true),
        updated_at: Set(now),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
}
