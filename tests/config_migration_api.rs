use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use meowmail::{
    AppState, build_router,
    cleanup::{CleanupRepository, CleanupRuleInput, MailSettings, RuleMatchMode},
    config::Config,
    db::entities::{mail_setting, notification_setting, user},
    preferences::{ListDensity, PreferencesRepository, SignatureInput},
    security::{decrypt_archive, encrypt_archive, hash_secret},
    users::UserRepository,
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
    let cleanup = CleanupRepository::new(state.db.clone());
    let admin_user = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    cleanup
        .update_settings(
            admin_user.id,
            MailSettings {
                keep_local_after_server_delete: true,
                sync_fetch_limit: None,
            },
        )
        .await
        .unwrap();
    let preferences = PreferencesRepository::new(state.db.clone());
    let mut mail_preferences = preferences.mail(admin_user.id).await.unwrap();
    mail_preferences.list_density = ListDensity::Compact;
    preferences
        .update_mail(admin_user.id, mail_preferences)
        .await
        .unwrap();
    let signature = preferences
        .create_signature(
            admin_user.id,
            SignatureInput {
                name: "Work".into(),
                body_text: "Meowmail Team".into(),
            },
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
    assert!(payload["users"][0]["mail_settings"]["syncFetchLimit"].is_null());
    assert_eq!(
        payload["users"][0]["preferences"]["mail"]["listDensity"],
        "compact"
    );
    assert_eq!(
        payload["users"][0]["preferences"]["signatures"][0]["name"],
        "Work"
    );

    cleanup
        .update_settings(
            admin_user.id,
            MailSettings {
                keep_local_after_server_delete: true,
                sync_fetch_limit: Some(125),
            },
        )
        .await
        .unwrap();
    preferences
        .update_mail(admin_user.id, Default::default())
        .await
        .unwrap();
    preferences
        .delete_signature(admin_user.id, signature.id)
        .await
        .unwrap();
    let imported = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users/migration/import",
            &admin,
            json!({
                "passphrase": "migration passphrase",
                "sections": sections(),
                "archive": personal,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(imported.status(), StatusCode::OK);
    let report: Value =
        serde_json::from_slice(&imported.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(report["signaturesImported"], 1);
    assert_eq!(report["preferencesImported"], 1);
    assert_eq!(
        cleanup
            .settings(admin_user.id)
            .await
            .unwrap()
            .sync_fetch_limit,
        None
    );
    assert_eq!(
        preferences.mail(admin_user.id).await.unwrap().list_density,
        ListDensity::Compact
    );
    assert_eq!(
        preferences.list_signatures(admin_user.id).await.unwrap()[0].name,
        "Work"
    );

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

#[tokio::test]
async fn version_one_archives_from_zero_two_import_with_legacy_defaults() {
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
    let admin_user = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let cleanup = CleanupRepository::new(state.db.clone());
    cleanup
        .create(
            admin_user.id,
            CleanupRuleInput {
                account_id: None,
                name: "Legacy newsletters".into(),
                match_mode: RuleMatchMode::All,
                conditions: Vec::new(),
                actions: Vec::new(),
                position: None,
                stop_processing: false,
                sender_contains: Some("newsletter@example.com".into()),
                subject_contains: None,
                body_contains: None,
                older_than_days: None,
                delete_from_server: false,
                enabled: true,
            },
        )
        .await
        .unwrap();
    let app = build_router(state);
    let admin = login(&app, "admin", "correct horse battery staple").await;
    let mut archive = export_archive(&app, &admin, "mine").await;
    let mut payload = decrypt_payload(&archive, "migration passphrase");
    payload["sections"]
        .as_object_mut()
        .unwrap()
        .remove("preferences");
    let user = payload["users"][0].as_object_mut().unwrap();
    user.remove("preferences");
    let rule = user["cleanup_rules"][0].as_object_mut().unwrap();
    for field in [
        "match_mode",
        "conditions",
        "actions",
        "position",
        "stop_processing",
    ] {
        rule.remove(field);
    }
    archive["sections"] = payload["sections"].clone();
    archive["encryptedData"] = Value::String(
        encrypt_archive(
            "migration passphrase",
            &serde_json::to_vec(&payload).unwrap(),
        )
        .unwrap(),
    );

    let imported = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users/migration/import",
            &admin,
            json!({
                "passphrase": "migration passphrase",
                "sections": archive["sections"].clone(),
                "archive": archive,
            }),
        ))
        .await
        .unwrap();
    assert_eq!(imported.status(), StatusCode::OK);
    let report: Value =
        serde_json::from_slice(&imported.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(report["rulesImported"], 1);
    assert_eq!(report["preferencesImported"], 0);
    let rules = cleanup.list(admin_user.id).await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].conditions.len(), 1);
    assert_eq!(rules[0].actions.len(), 1);
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
        "preferences": true,
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
        sync_fetch_limit: Set(Some(50)),
        updated_at: Set(now),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
}
