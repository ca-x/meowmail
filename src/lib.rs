pub mod accounts;
pub mod auth;
pub mod cleanup;
pub mod config;
pub mod db;
pub mod error;
pub mod mail;
pub mod messages;
pub mod notifications;
pub mod security;
pub mod users;
pub mod web;

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, header},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use tower_http::{
    catch_panic::CatchPanicLayer, compression::CompressionLayer, limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::{
    auth::{OidcService, SessionStore},
    config::Config,
    db::Database,
    notifications::NotificationRunner,
    security::CredentialVault,
    users::UserRepository,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Database,
    pub vault: CredentialVault,
    pub sessions: SessionStore,
    pub oidc: Option<OidcService>,
    pub notifications: NotificationRunner,
}

impl AppState {
    pub async fn initialize(config: Config) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&config.data_dir).await?;
        let db = Database::connect(&config.database_path()).await?;
        let vault = CredentialVault::load(
            config.vault_secret.as_ref(),
            &config.vault_salt_path(),
            &config.vault_key_path(),
        )?;
        let users = UserRepository::new(db.clone());
        users.bootstrap(config.bootstrap_admin.as_ref()).await?;
        if config.auth_mode.local_enabled() && !users.has_local_user().await? {
            anyhow::bail!(
                "local authentication is enabled but no local user exists; set MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME and MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD"
            );
        }
        let oidc = match config.oidc.as_ref() {
            Some(oidc) => Some(OidcService::discover(oidc).await?),
            None => None,
        };
        let notifications = NotificationRunner::new(db.clone());
        Ok(Self {
            config: Arc::new(config),
            db,
            vault,
            sessions: SessionStore::default(),
            oidc,
            notifications,
        })
    }
}

pub fn build_router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(health))
        .merge(auth::routes())
        .merge(cleanup::routes())
        .merge(accounts::routes())
        .merge(messages::routes())
        .merge(notifications::routes())
        .merge(users::routes());

    Router::new()
        .nest("/api/v1", api)
        .fallback(web::serve)
        .layer(middleware::from_fn(security_headers))
        .layer(RequestBodyLimitLayer::new(16 * 1024 * 1024))
        .layer(CompressionLayer::new())
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "name": "meowmail",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=(), payment=()"),
    );
    response
}
