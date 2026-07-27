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

use super::{MailPreferences, PreferencesRepository, Signature, SignatureInput};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/preferences/mail", get(get_mail).put(update_mail))
        .route("/signatures", get(list_signatures).post(create_signature))
        .route(
            "/signatures/{id}",
            axum::routing::patch(update_signature).delete(delete_signature),
        )
}

async fn get_mail(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<MailPreferences>, AppError> {
    Ok(Json(
        PreferencesRepository::new(state.db)
            .mail(session.user_id)
            .await?,
    ))
}

async fn update_mail(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(preferences): Json<MailPreferences>,
) -> Result<Json<MailPreferences>, AppError> {
    Ok(Json(
        PreferencesRepository::new(state.db)
            .update_mail(mutation.0.user_id, preferences)
            .await?,
    ))
}

async fn list_signatures(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<Signature>>, AppError> {
    Ok(Json(
        PreferencesRepository::new(state.db)
            .list_signatures(session.user_id)
            .await?,
    ))
}

async fn create_signature(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<SignatureInput>,
) -> Result<Json<Signature>, AppError> {
    Ok(Json(
        PreferencesRepository::new(state.db)
            .create_signature(mutation.0.user_id, input)
            .await?,
    ))
}

async fn update_signature(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
    Json(input): Json<SignatureInput>,
) -> Result<Json<Signature>, AppError> {
    Ok(Json(
        PreferencesRepository::new(state.db)
            .update_signature(mutation.0.user_id, id, input)
            .await?,
    ))
}

async fn delete_signature(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    PreferencesRepository::new(state.db)
        .delete_signature(mutation.0.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
