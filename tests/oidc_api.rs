use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use meowmail::{
    AppState, build_router,
    config::{AuthMode, Config, OidcConfig},
};
use openidconnect::{
    AccessToken, Audience, IssuerUrl, JsonWebKeyId, Nonce, PrivateSigningKey, StandardClaims,
    SubjectIdentifier,
    core::{CoreIdToken, CoreIdTokenClaims, CoreJwsSigningAlgorithm, CoreRsaPrivateSigningKey},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use url::Url;

const RSA_KEY: &str = concat!(
    "-----BEGIN RSA ",
    "PRIVATE KEY-----\n",
    "\
MIIEowIBAAKCAQEAn4EPtAOCc9AlkeQHPzHStgAbgs7bTZLwUBZdR8/KuKPEHLd4\n\
rHVTeT+O+XV2jRojdNhxJWTDvNd7nqQ0VEiZQHz/AJmSCpMaJMRBSFKrKb2wqVwG\n\
U/NsYOYL+QtiWN2lbzcEe6XC0dApr5ydQLrHqkHHig3RBordaZ6Aj+oBHqFEHYpP\n\
e7Tpe+OfVfHd1E6cS6M1FZcD1NNLYD5lFHpPI9bTwJlsde3uhGqC0ZCuEHg8lhzw\n\
OHrtIQbS0FVbb9k3+tVTU4fg/3L/vniUFAKwuCLqKnS2BYwdq/mzSnbLY7h/qixo\n\
R7jig3//kRhuaxwUkRz5iaiQkqgc5gHdrNP5zwIDAQABAoIBAG1lAvQfhBUSKPJK\n\
Rn4dGbshj7zDSr2FjbQf4pIh/ZNtHk/jtavyO/HomZKV8V0NFExLNi7DUUvvLiW7\n\
0PgNYq5MDEjJCtSd10xoHa4QpLvYEZXWO7DQPwCmRofkOutf+NqyDS0QnvFvp2d+\n\
Lov6jn5C5yvUFgw6qWiLAPmzMFlkgxbtjFAWMJB0zBMy2BqjntOJ6KnqtYRMQUxw\n\
TgXZDF4rhYVKtQVOpfg6hIlsaoPNrF7dofizJ099OOgDmCaEYqM++bUlEHxgrIVk\n\
wZz+bg43dfJCocr9O5YX0iXaz3TOT5cpdtYbBX+C/5hwrqBWru4HbD3xz8cY1TnD\n\
qQa0M8ECgYEA3Slxg/DwTXJcb6095RoXygQCAZ5RnAvZlno1yhHtnUex/fp7AZ/9\n\
nRaO7HX/+SFfGQeutao2TDjDAWU4Vupk8rw9JR0AzZ0N2fvuIAmr/WCsmGpeNqQn\n\
ev1T7IyEsnh8UMt+n5CafhkikzhEsrmndH6LxOrvRJlsPp6Zv8bUq0kCgYEAuKE2\n\
dh+cTf6ERF4k4e/jy78GfPYUIaUyoSSJuBzp3Cubk3OCqs6grT8bR/cu0Dm1MZwW\n\
mtdqDyI95HrUeq3MP15vMMON8lHTeZu2lmKvwqW7anV5UzhM1iZ7z4yMkuUwFWoB\n\
vyY898EXvRD+hdqRxHlSqAZ192zB3pVFJ0s7pFcCgYAHw9W9eS8muPYv4ZhDu/fL\n\
2vorDmD1JqFcHCxZTOnX1NWWAj5hXzmrU0hvWvFC0P4ixddHf5Nqd6+5E9G3k4E5\n\
2IwZCnylu3bqCWNh8pT8T3Gf5FQsfPT5530T2BcsoPhUaeCnP499D+rb2mTnFYeg\n\
mnTT1B/Ue8KGLFFfn16GKQKBgAiw5gxnbocpXPaO6/OKxFFZ+6c0OjxfN2PogWce\n\
TU/k6ZzmShdaRKwDFXisxRJeNQ5Rx6qgS0jNFtbDhW8E8WFmQ5urCOqIOYk28EBi\n\
At4JySm4v+5P7yYBh8B8YD2l9j57z/s8hJAxEbn/q8uHP2ddQqvQKgtsni+pHSk9\n\
XGBfAoGBANz4qr10DdM8DHhPrAb2YItvPVz/VwkBd1Vqj8zCpyIEKe/07oKOvjWQ\n\
SgkLDH9x2hBgY01SbP43CvPk0V72invu2TGkI/FXwXWJLLG7tDSgw4YyfhrYrHmg\n\
1Vre3XB9HH8MYBVB6UIexaAq4xSeoemRKTBesZro7OKjKT8/GmiO\n\
",
    "-----END RSA ",
    "PRIVATE KEY-----"
);

const ROTATED_RSA_KEY: &str = concat!(
    "-----BEGIN RSA ",
    "PRIVATE KEY-----\n",
    "\
MIIEogIBAAKCAQEAhZwRizyITM5gBuhqDnFZwNdE08BN+qNApIi597T7Ti5yzLNn\n\
hIAYCcrZXg/kYl2fOh2wugmI/t7JCW4mGHXewGkjQED3FMqrlqXy82Ust2voMOZA\n\
64z3mazQ/VtOvBJlQznI4z6GFm1ET0SISCbFCShBGKDsukhes2jxpqBWb/lkAMkr\n\
LAfBKzoetNSYERDqQZhxEHEJbrroazP+x4zSQXKHlsVCmNCAfll54xLyoLceALQM\n\
g+74dS6yEKh6uYGvpg0dHD35/zGn/DYl+7GkUZ3M23ceM/pd6F4qRxZQvrJhh2Bs\n\
lZZ+aTIP4facYhIkFIfn0N+yID8eOUPeitXw1QIDAQABAoIBABrY9omE+1p7qb4Z\n\
m54VVtSyLQljvgecIFQviTbmLg1Stgy+DBIK70mgcjc9eEXvzBwQdT+cxON5/umf\n\
MZZ+sOj293dk1oFeDEa0R/JypR6iV0DkM61hYSuHF2OholuWUrTEesJ3ANim0jAf\n\
dEcTS0qAxTveslLoUec5Mj2qQFQ1fXa9/W9q1A5T/aqekJfybZq1VUeYES9nBUM5\n\
SvcUXcPc/IS90tC37Nx+Q76wgbrOttJYb/2hDQF6Y1KxDQYLC+igGjG82vcJeBC6\n\
7gtdHv/Hjjnpa8LXN/K9P70AcYzmw9+19sM+sDKUI6lHnT2AllmuTeBtDpdPVGzE\n\
JShR8kECgYEAu1UTPQ+YCc7c+xr6WdIaNX+je3TYVzFr3F8BUyj5KiBUVAivhWR6\n\
glIYKAkG+6/jNL1ssqTQQrrIr2fFQm7zV1YHOJmel0OpA4mLyj08ZR+imq4nPJtE\n\
TnKASKXlO7ti3Wfq87waWS3TR99jg47VTh+jxIFkZMvafSDmTZGSwQUCgYEAtpXB\n\
KaqD56BS4ARggh56zo0kBdxXz6lyffCBUb4y1McUH89HVo2HoVww1JCAejXPbYdU\n\
6dIJS2cXmE3dN/V8rOGWI02hGiWvWDitYDqTmrz7pJvaOEDORExNEQm3ApjQe5Rk\n\
JGZf1TGk1kBKOi5kq5ypXHfW66hzkGULKdFyuZECgYBAkaFQ8ZgJAWk/j9vsq7Nm\n\
7zitK+gJnbo8ue5d+IhxUbVfRaMiCjEDzEIRQpNKmyRoIEZgCNjTt0fG1bCzJkTv\n\
vHI+uwxjvOl7k6RAL/0qKc5FHHPfuvC/TU4UPEIX5Y29HL1qB1LZnCbv5fqJ9Ohm\n\
xhcPez3cVDtZ18YpxFxd9QKBgATx82RcgOwSFIyKsc50YuEbQ4GBIUO3lClDDU2Y\n\
eCn5JltiMs1uUeEV7SCktUYaFP8jbjJTBPts1F/EpBwy4uiPx5A3NwjNQn2CM3fq\n\
vjqvqaUgr31ci/mfk2rFt5Yza1odf8TYnPnaOVuwLBJ9VS6stI2TlVeWnWKoye+d\n\
A51xAoGAF3xLpBZ2dQHilTwuDXawoOxPsBbzByhHqe2RpVY49YnIpr/E/rnRjR1p\n\
R0tXAQdHJJi/zdERe8HHUQNMVdS8Ve/9pgPveFy9iArgdm0bM2i/Gp9WpHSvJXJo\n\
WABXfXCkPYzmH/WS3fmzzXMf54OUumtTHZvyf9KpvIvFsXOLg9E=\n\
",
    "-----END RSA ",
    "PRIVATE KEY-----"
);

#[derive(Clone)]
struct ProviderState {
    issuer: String,
    kid: Arc<Mutex<&'static str>>,
    key: Arc<Mutex<&'static str>>,
    token_kid: &'static str,
    token_key: &'static str,
    nonce: Arc<Mutex<Option<String>>>,
}

async fn discovery(State(state): State<ProviderState>) -> Json<Value> {
    Json(json!({
        "issuer": state.issuer,
        "authorization_endpoint": format!("{}authorize", state.issuer),
        "token_endpoint": format!("{}token", state.issuer),
        "jwks_uri": format!("{}jwks", state.issuer),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "token_endpoint_auth_methods_supported": ["client_secret_basic"]
    }))
}

async fn jwks(State(state): State<ProviderState>) -> Json<Value> {
    let kid = *state.kid.lock().unwrap();
    let pem = *state.key.lock().unwrap();
    let key = CoreRsaPrivateSigningKey::from_pem(pem, Some(JsonWebKeyId::new(kid.into())))
        .unwrap()
        .as_verification_key();
    Json(json!({ "keys": [key] }))
}

async fn token(State(state): State<ProviderState>) -> Json<Value> {
    let nonce = state.nonce.lock().unwrap().clone().unwrap();
    let access_token = AccessToken::new("access-token".into());
    let signing_key = CoreRsaPrivateSigningKey::from_pem(
        state.token_key,
        Some(JsonWebKeyId::new(state.token_kid.into())),
    )
    .unwrap();
    let id_token = CoreIdToken::new(
        CoreIdTokenClaims::new(
            IssuerUrl::new(state.issuer).unwrap(),
            vec![Audience::new("meowmail-test".into())],
            Utc::now() + Duration::minutes(5),
            Utc::now(),
            StandardClaims::new(SubjectIdentifier::new("rotated-key-user".into())),
            Default::default(),
        )
        .set_nonce(Some(Nonce::new(nonce))),
        &signing_key,
        CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
        Some(&access_token),
        None,
    )
    .unwrap();
    Json(json!({
        "access_token": "access-token",
        "token_type": "Bearer",
        "expires_in": 300,
        "id_token": id_token.to_string()
    }))
}

#[tokio::test]
async fn oidc_callback_refreshes_rotated_signing_keys_without_a_restart() {
    assert_oidc_key_rotation("old-key", "new-key", RSA_KEY, RSA_KEY).await;
}

#[tokio::test]
async fn oidc_callback_refreshes_a_rotated_key_that_reuses_the_same_kid() {
    assert_oidc_key_rotation("stable-key", "stable-key", RSA_KEY, ROTATED_RSA_KEY).await;
}

async fn assert_oidc_key_rotation(
    initial_kid: &'static str,
    rotated_kid: &'static str,
    initial_key: &'static str,
    rotated_key: &'static str,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let issuer = format!(
        "http://localhost:{}/",
        listener.local_addr().unwrap().port()
    );
    let provider = ProviderState {
        issuer: issuer.clone(),
        kid: Arc::new(Mutex::new(initial_kid)),
        key: Arc::new(Mutex::new(initial_key)),
        token_kid: rotated_kid,
        token_key: rotated_key,
        nonce: Arc::new(Mutex::new(None)),
    };
    let provider_app = Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks", get(jwks))
        .route("/token", post(token))
        .with_state(provider.clone());
    tokio::spawn(async move { axum::serve(listener, provider_app).await.unwrap() });

    let directory = tempfile::tempdir().unwrap();
    let config = Config {
        bind: "127.0.0.1:0".parse().unwrap(),
        data_dir: directory.path().to_path_buf(),
        auth_mode: AuthMode::Oidc,
        bootstrap_admin: None,
        oidc: Some(OidcConfig {
            issuer: Url::parse(&issuer).unwrap(),
            client_id: "meowmail-test".into(),
            client_secret: Some("test-secret".into()),
            redirect_url: Url::parse("http://localhost/api/v1/auth/oidc/callback").unwrap(),
            scopes: vec!["openid".into()],
            first_user_admin: true,
        }),
        vault_secret: Some("test-vault-secret".into()),
    };
    let app = build_router(AppState::initialize(config).await.unwrap());
    let start = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/oidc/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(start.status(), StatusCode::TEMPORARY_REDIRECT);
    let authorization = Url::parse(start.headers()[header::LOCATION].to_str().unwrap()).unwrap();
    let parameters = authorization
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    let returned_state = parameters["state"].to_string();
    *provider.nonce.lock().unwrap() = Some(parameters["nonce"].to_string());
    *provider.kid.lock().unwrap() = rotated_kid;
    *provider.key.lock().unwrap() = rotated_key;

    let callback = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/auth/oidc/callback?code=test-code&state={returned_state}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(callback.status(), StatusCode::SEE_OTHER);
    assert!(callback.headers().contains_key(header::SET_COOKIE));
}
