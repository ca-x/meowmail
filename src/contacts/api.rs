use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
};

use super::{Contact, ContactInput, ContactRepository};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/contacts", get(list_contacts).post(create_contact))
        .route(
            "/contacts/{id}",
            axum::routing::patch(update_contact).delete(delete_contact),
        )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContactQuery {
    q: Option<String>,
    limit: Option<u64>,
}

async fn list_contacts(
    State(state): State<AppState>,
    session: AuthenticatedSession,
    Query(query): Query<ContactQuery>,
) -> Result<Json<Vec<Contact>>, AppError> {
    Ok(Json(
        ContactRepository::new(state.db)
            .list(session.user_id, query.q, query.limit.unwrap_or(50))
            .await?,
    ))
}

async fn create_contact(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<ContactInput>,
) -> Result<Json<Contact>, AppError> {
    Ok(Json(
        ContactRepository::new(state.db)
            .create(mutation.0.user_id, input)
            .await?,
    ))
}

async fn update_contact(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
    Json(input): Json<ContactInput>,
) -> Result<Json<Contact>, AppError> {
    Ok(Json(
        ContactRepository::new(state.db)
            .update(mutation.0.user_id, id, input)
            .await?,
    ))
}

async fn delete_contact(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    ContactRepository::new(state.db)
        .delete(mutation.0.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
