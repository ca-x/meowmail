use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use meowmail::{AppState, build_router, config::Config};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn login_creates_a_protected_session_and_csrf_guards_mutations() {
    let directory = tempfile::tempdir().unwrap();
    let config = Config::new("2468-meowmail".into(), directory.path().to_path_buf()).unwrap();
    let app = build_router(AppState::initialize(config).await.unwrap());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"2468-meowmail"}"#,
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
        .to_owned();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    let cookie_pair = cookie.split(';').next().unwrap();
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let csrf = body["csrfToken"].as_str().unwrap().to_owned();

    let session = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/session")
                .header(header::COOKIE, cookie_pair)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(session.status(), StatusCode::OK);

    let rejected = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/accounts")
                .header(header::COOKIE, cookie_pair)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);

    let logout = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::COOKIE, cookie_pair)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let config = Config::new("2468-meowmail".into(), directory.path().to_path_buf()).unwrap();
    let app = build_router(AppState::initialize(config).await.unwrap());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"wrong-password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn profile_username_change_is_validated_and_updates_local_login() {
    let directory = tempfile::tempdir().unwrap();
    let config = Config::new("2468-meowmail".into(), directory.path().to_path_buf()).unwrap();
    let app = build_router(AppState::initialize(config).await.unwrap());
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"2468-meowmail"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body: Value =
        serde_json::from_slice(&login.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let csrf = body["csrfToken"].as_str().unwrap();

    let legacy_profile = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/users/me")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf)
                .body(Body::from(r#"{"nickname":"Legacy client"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(legacy_profile.status(), StatusCode::OK);
    let legacy_profile: Value = serde_json::from_slice(
        &legacy_profile
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
    )
    .unwrap();
    assert_eq!(legacy_profile["username"], "admin");
    assert_eq!(legacy_profile["nickname"], "Legacy client");

    let too_short = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/users/me")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf)
                .body(Body::from(r#"{"username":"a","nickname":"Admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(too_short.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/v1/users/me")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf)
                .body(Body::from(r#"{"username":"New.Admin","nickname":"Admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(updated.status(), StatusCode::OK);
    let updated: Value =
        serde_json::from_slice(&updated.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(updated["username"], "new.admin");

    let wrong_current_password = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/users/me/password")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf)
                .body(Body::from(
                    r#"{"currentPassword":"wrong","newPassword":"new secure password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_current_password.status(), StatusCode::UNAUTHORIZED);

    let oversized_password = "猫".repeat(2_000);
    let oversized_password = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/users/me/password")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf)
                .body(Body::from(
                    serde_json::json!({
                        "currentPassword": "2468-meowmail",
                        "newPassword": oversized_password,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        oversized_password.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let second_login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"new.admin","password":"2468-meowmail"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second_login.status(), StatusCode::OK);
    let second_cookie = second_login.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();

    let password_updated = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/users/me/password")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", csrf)
                .body(Body::from(
                    r#"{"currentPassword":"2468-meowmail","newPassword":"new secure password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(password_updated.status(), StatusCode::OK);

    let revoked_session = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/session")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoked_session.status(), StatusCode::UNAUTHORIZED);

    let revoked_second_session = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/session")
                .header(header::COOKIE, &second_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoked_second_session.status(), StatusCode::UNAUTHORIZED);

    let old_login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"2468-meowmail"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(old_login.status(), StatusCode::UNAUTHORIZED);

    let new_login = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"new.admin","password":"new secure password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(new_login.status(), StatusCode::OK);

    let restarted = build_router(
        AppState::initialize(
            Config::new("2468-meowmail".into(), directory.path().to_path_buf()).unwrap(),
        )
        .await
        .unwrap(),
    );
    let bootstrap_login = restarted
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"2468-meowmail"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bootstrap_login.status(), StatusCode::UNAUTHORIZED);

    let persisted_login = restarted
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"new.admin","password":"new secure password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(persisted_login.status(), StatusCode::OK);
}

#[tokio::test]
async fn personal_pin_locks_and_unlocks_an_authenticated_session() {
    let directory = tempfile::tempdir().unwrap();
    let config = Config::new("2468-meowmail".into(), directory.path().to_path_buf()).unwrap();
    let app = build_router(AppState::initialize(config).await.unwrap());
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"2468-meowmail"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login
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
        serde_json::from_slice(&login.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let csrf = body["csrfToken"].as_str().unwrap().to_owned();

    let set_pin = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/auth/pin")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf)
                .body(Body::from(r#"{"pin":"135790"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(set_pin.status(), StatusCode::OK);

    let locked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/lock")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(locked.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&locked.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["locked"], true);

    let protected = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/accounts")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(protected.status(), StatusCode::LOCKED);

    let rejected = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/unlock")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf)
                .body(Body::from(r#"{"pin":"wrong"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);

    let unlocked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/unlock")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf)
                .body(Body::from(r#"{"pin":"135790"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unlocked.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&unlocked.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["locked"], false);
}

#[tokio::test]
async fn locked_session_can_logout_and_auto_lock_is_pin_guarded() {
    let directory = tempfile::tempdir().unwrap();
    let config = Config::new("2468-meowmail".into(), directory.path().to_path_buf()).unwrap();
    let app = build_router(AppState::initialize(config).await.unwrap());
    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"2468-meowmail"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = login
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
        serde_json::from_slice(&login.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let csrf = body["csrfToken"].as_str().unwrap().to_owned();

    let rejected = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/users/me/auto-lock")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf)
                .body(Body::from(r#"{"minutes":15}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let set_pin = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/auth/pin")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf)
                .body(Body::from(r#"{"pin":"135790"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(set_pin.status(), StatusCode::OK);

    let auto_lock = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/v1/users/me/auto-lock")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-csrf-token", &csrf)
                .body(Body::from(r#"{"minutes":15}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(auto_lock.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&auto_lock.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["autoLockMinutes"], 15);

    let locked = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/lock")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(locked.status(), StatusCode::OK);

    let logout = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::COOKIE, &cookie)
                .header("x-csrf-token", &csrf)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::OK);
    assert!(
        logout
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );

    let expired = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/session")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);
}
