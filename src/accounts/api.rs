use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch, post},
};
use secrecy::SecretString;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{MutationSession, require_session},
    error::AppError,
    mail,
};

use super::{AccountInput, AccountRepository, AccountSecrets, MailAccount, ProxyConfig};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/accounts", get(list).post(create))
        .route("/accounts/test", post(test_draft))
        .route("/accounts/{id}", patch(update).delete(remove))
        .route("/accounts/{id}/test", post(test_saved))
}

async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<MailAccount>>, AppError> {
    require_session(&state, &headers)?;
    Ok(Json(repository(&state).list().await?))
}

async fn create(
    State(state): State<AppState>,
    _mutation: MutationSession,
    Json(input): Json<AccountInput>,
) -> Result<Json<MailAccount>, AppError> {
    Ok(Json(repository(&state).create(input).await?))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _mutation: MutationSession,
    Json(input): Json<AccountInput>,
) -> Result<Json<MailAccount>, AppError> {
    Ok(Json(repository(&state).update(id, input).await?))
}

async fn remove(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _mutation: MutationSession,
) -> Result<axum::http::StatusCode, AppError> {
    repository(&state).delete(id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn test_saved(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    _mutation: MutationSession,
) -> Result<Json<ConnectionTestResponse>, AppError> {
    let (account, secrets, proxy) = repository(&state).get_with_secrets(id).await?;
    test_connection(account, secrets, proxy).await
}

async fn test_draft(
    _mutation: MutationSession,
    Json(mut input): Json<AccountInput>,
) -> Result<Json<ConnectionTestResponse>, AppError> {
    input.validate(true)?;
    let password = SecretString::from(input.password.take().expect("validated password exists"));
    let proxy_password = input
        .proxy
        .password
        .take()
        .filter(|value| !value.is_empty())
        .map(SecretString::from);
    let proxy = ProxyConfig {
        kind: input.proxy.kind,
        host: input.proxy.host.clone(),
        port: input.proxy.port,
        username: input.proxy.username.clone(),
        password: proxy_password.clone(),
    };
    let account = MailAccount {
        id: Uuid::nil(),
        display_name: input.display_name,
        email: input.email,
        username: input.username,
        imap: input.imap,
        smtp: input.smtp,
        proxy: super::model::PublicProxyConfig {
            kind: input.proxy.kind,
            host: input.proxy.host,
            port: input.proxy.port,
            username: input.proxy.username,
            has_password: proxy_password.is_some(),
        },
        is_default: false,
        last_synced_at: None,
        created_at: 0,
        updated_at: 0,
        has_password: true,
    };
    test_connection(
        account,
        AccountSecrets {
            password,
            proxy_password,
        },
        proxy,
    )
    .await
}

async fn test_connection(
    account: MailAccount,
    secrets: AccountSecrets,
    proxy: ProxyConfig,
) -> Result<Json<ConnectionTestResponse>, AppError> {
    let imap = mail::test_imap(&account, &secrets, &proxy).await;
    let smtp = mail::test_smtp(&account, &secrets, &proxy).await;
    let response = ConnectionTestResponse {
        imap: imap.is_ok(),
        smtp: smtp.is_ok(),
    };
    if let Err(error) = imap {
        tracing::warn!(account_id = %account.id, protocol = "imap", error = %error, "mail connection test failed");
        return Err(AppError::Mail("IMAP connection failed".into()));
    }
    if let Err(error) = smtp {
        tracing::warn!(account_id = %account.id, protocol = "smtp", error = %error, "mail connection test failed");
        return Err(AppError::Mail("SMTP connection failed".into()));
    }
    Ok(Json(response))
}

fn repository(state: &AppState) -> AccountRepository {
    AccountRepository::new(state.db.clone(), state.vault.clone())
}

#[derive(Serialize)]
struct ConnectionTestResponse {
    imap: bool,
    smtp: bool,
}
