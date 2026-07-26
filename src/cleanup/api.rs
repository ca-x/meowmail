use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
};

use super::{CleanupRepository, CleanupRule, CleanupRuleInput, MailSettings};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mail/settings", get(settings).patch(update_settings))
        .route("/cleanup/rules", get(list_rules).post(create_rule))
        .route(
            "/cleanup/rules/{id}",
            axum::routing::patch(update_rule).delete(delete_rule),
        )
}

async fn settings(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<MailSettings>, AppError> {
    Ok(Json(
        CleanupRepository::new(state.db)
            .settings(session.user_id)
            .await?,
    ))
}

async fn update_settings(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(settings): Json<MailSettings>,
) -> Result<Json<MailSettings>, AppError> {
    Ok(Json(
        CleanupRepository::new(state.db)
            .update_settings(mutation.0.user_id, settings)
            .await?,
    ))
}

async fn list_rules(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<CleanupRule>>, AppError> {
    Ok(Json(
        CleanupRepository::new(state.db)
            .list(session.user_id)
            .await?,
    ))
}

async fn create_rule(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<CleanupRuleInput>,
) -> Result<Json<CleanupRule>, AppError> {
    Ok(Json(
        CleanupRepository::new(state.db)
            .create(mutation.0.user_id, input)
            .await?,
    ))
}

async fn update_rule(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
    Json(input): Json<CleanupRuleInput>,
) -> Result<Json<CleanupRule>, AppError> {
    Ok(Json(
        CleanupRepository::new(state.db)
            .update(mutation.0.user_id, id, input)
            .await?,
    ))
}

async fn delete_rule(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    CleanupRepository::new(state.db)
        .delete(mutation.0.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
