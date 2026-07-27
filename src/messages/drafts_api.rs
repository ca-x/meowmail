use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
    mcp::{DraftRepository, EmailDraft, EmailDraftStatus},
};

use super::{ComposeInput, ThreadingHeaders, send_outgoing};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/drafts", get(list_drafts).post(create_draft))
        .route(
            "/drafts/{id}",
            axum::routing::patch(update_draft).delete(delete_draft),
        )
        .route("/drafts/{id}/send", post(send_draft))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftInput {
    #[serde(flatten)]
    compose: ComposeInput,
    scheduled_at: Option<i64>,
}

async fn list_drafts(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<EmailDraft>>, AppError> {
    Ok(Json(
        DraftRepository::new(state.db)
            .list(session.user_id, 100)
            .await?,
    ))
}

async fn create_draft(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<DraftInput>,
) -> Result<Json<EmailDraft>, AppError> {
    let scheduled_at = normalize_schedule(input.scheduled_at)?;
    Ok(Json(
        DraftRepository::new(state.db)
            .create(
                mutation.0.user_id,
                input.compose,
                None,
                ThreadingHeaders::default(),
                scheduled_at,
            )
            .await?,
    ))
}

async fn update_draft(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
    Json(input): Json<DraftInput>,
) -> Result<Json<EmailDraft>, AppError> {
    Ok(Json(
        DraftRepository::new(state.db)
            .update(
                mutation.0.user_id,
                id,
                input.compose,
                normalize_schedule(input.scheduled_at)?,
            )
            .await?,
    ))
}

async fn delete_draft(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    DraftRepository::new(state.db)
        .delete(mutation.0.user_id, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn send_draft(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let repository = DraftRepository::new(state.db.clone());
    let stored = repository.claim_for_send(mutation.0.user_id, id).await?;
    if let Err(error) = send_outgoing(
        &state,
        mutation.0.user_id,
        stored.clone().into_compose(),
        stored.threading,
    )
    .await
    {
        repository
            .mark_after_send_failure(mutation.0.user_id, id, EmailDraftStatus::Draft)
            .await?;
        return Err(error);
    }
    repository.finish_sent(mutation.0.user_id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn normalize_schedule(value: Option<i64>) -> Result<Option<i64>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if value <= now || value > now + 366 * 24 * 60 * 60 {
        return Err(AppError::Validation(
            "scheduled send time is invalid".into(),
        ));
    }
    Ok(Some(value))
}
