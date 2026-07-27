use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{
    AppState,
    auth::{MutationSession, require_session},
    error::AppError,
};

use super::{McpRepository, McpSettings};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct McpSettingsUpdate {
    allow_delete: bool,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mcp/settings", get(settings).patch(update_settings))
        .route("/mcp/token", post(generate_token).delete(revoke_token))
}

async fn settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<McpSettings>, AppError> {
    let session = require_session(&state, &headers)?;
    Ok(Json(
        McpRepository::new(state.db)
            .settings(session.user_id)
            .await?,
    ))
}

async fn generate_token(
    State(state): State<AppState>,
    mutation: MutationSession,
) -> Result<Response, AppError> {
    let mut response = Json(
        McpRepository::new(state.db)
            .generate(mutation.0.user_id)
            .await?,
    )
    .into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, private"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    Ok(response)
}

async fn update_settings(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<McpSettingsUpdate>,
) -> Result<Json<McpSettings>, AppError> {
    Ok(Json(
        McpRepository::new(state.db)
            .set_allow_delete(mutation.0.user_id, input.allow_delete)
            .await?,
    ))
}

async fn revoke_token(
    State(state): State<AppState>,
    mutation: MutationSession,
) -> Result<StatusCode, AppError> {
    McpRepository::new(state.db)
        .revoke(mutation.0.user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
