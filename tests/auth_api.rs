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
