use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
};

use super::{
    Calendar, CalendarAccount, CalendarAccountInput, CalendarEvent, CalendarRepository,
    CalendarUpdate, caldav,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/calendar/accounts",
            get(list_accounts).post(create_account),
        )
        .route(
            "/calendar/accounts/{id}",
            axum::routing::patch(update_account).delete(delete_account),
        )
        .route(
            "/calendar/accounts/{id}/discover",
            axum::routing::post(discover_account),
        )
        .route(
            "/calendar/accounts/{id}/sync",
            axum::routing::post(sync_account),
        )
        .route("/calendars", get(list_calendars))
        .route("/calendars/{id}", axum::routing::patch(update_calendar))
        .route("/calendar/events", get(list_events))
}

async fn list_accounts(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<CalendarAccount>>, AppError> {
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .list_accounts(session.user_id)
            .await?,
    ))
}

async fn create_account(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<CalendarAccountInput>,
) -> Result<Json<CalendarAccount>, AppError> {
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .create_account(mutation.0.user_id, input)
            .await?,
    ))
}

async fn update_account(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
    Json(input): Json<CalendarAccountInput>,
) -> Result<Json<CalendarAccount>, AppError> {
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .update_account(mutation.0.user_id, id, input)
            .await?,
    ))
}

async fn delete_account(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    CalendarRepository::new(state.db, state.vault)
        .delete_account(mutation.0.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn list_calendars(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<Calendar>>, AppError> {
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .list_calendars(session.user_id)
            .await?,
    ))
}

async fn update_calendar(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
    Json(input): Json<CalendarUpdate>,
) -> Result<Json<Calendar>, AppError> {
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .update_calendar(mutation.0.user_id, id, input)
            .await?,
    ))
}

async fn discover_account(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Calendar>>, AppError> {
    let repository = CalendarRepository::new(state.db, state.vault);
    let (account, secrets) = repository
        .get_account_with_secrets(mutation.0.user_id, id)
        .await?;
    let remote = caldav::discover(&account.base_url, &account.username, secrets.password()).await?;
    Ok(Json(
        repository
            .upsert_remote_calendars(mutation.0.user_id, id, remote)
            .await?,
    ))
}

async fn sync_account(
    State(state): State<AppState>,
    mutation: MutationSession,
    Path(id): Path<Uuid>,
) -> Result<Json<CalendarSyncResponse>, AppError> {
    let repository = CalendarRepository::new(state.db, state.vault);
    let imported = match caldav::sync_account(&repository, mutation.0.user_id, id).await {
        Ok(imported) => imported,
        Err(error) => {
            let message = error.to_string();
            let _ = repository
                .mark_account_synced(mutation.0.user_id, id, Some(message.clone()))
                .await;
            return Err(AppError::Calendar(message));
        }
    };
    Ok(Json(CalendarSyncResponse { imported }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CalendarSyncResponse {
    imported: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventQuery {
    start: Option<i64>,
    end: Option<i64>,
}

async fn list_events(
    State(state): State<AppState>,
    session: AuthenticatedSession,
    Query(query): Query<EventQuery>,
) -> Result<Json<Vec<CalendarEvent>>, AppError> {
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let start = query.start.unwrap_or(now.saturating_sub(30 * 86_400));
    let end = query.end.unwrap_or(now.saturating_add(90 * 86_400));
    if end <= start || end.saturating_sub(start) > 370 * 86_400 {
        return Err(AppError::Validation(
            "calendar event range is invalid".into(),
        ));
    }
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .list_events(session.user_id, start, end)
            .await?,
    ))
}
