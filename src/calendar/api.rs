use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
};

use super::{
    Calendar, CalendarAccount, CalendarAccountInput, CalendarDayInfo, CalendarEvent,
    CalendarFeature, CalendarPreferences, CalendarRepository, CalendarUpdate, caldav, lunar,
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
        .route(
            "/calendar/preferences",
            get(get_preferences).put(update_preferences),
        )
        .route("/calendar/day-info", get(list_day_info))
        .route("/calendar/events", get(list_events))
}

async fn get_preferences(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<CalendarPreferences>, AppError> {
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .preferences(session.user_id)
            .await?,
    ))
}

async fn update_preferences(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(preferences): Json<CalendarPreferences>,
) -> Result<Json<CalendarPreferences>, AppError> {
    Ok(Json(
        CalendarRepository::new(state.db, state.vault)
            .update_preferences(mutation.0.user_id, preferences)
            .await?,
    ))
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DayInfoQuery {
    start: String,
    end: String,
    #[serde(default)]
    detail: bool,
}

async fn list_day_info(
    State(state): State<AppState>,
    session: AuthenticatedSession,
    Query(query): Query<DayInfoQuery>,
) -> Result<Json<Vec<CalendarDayInfo>>, AppError> {
    let format = time::format_description::parse_borrowed::<3>("[year]-[month]-[day]")
        .map_err(AppError::internal)?;
    let start = Date::parse(&query.start, &format)
        .map_err(|_| AppError::Validation("calendar date range is invalid".into()))?;
    let end = Date::parse(&query.end, &format)
        .map_err(|_| AppError::Validation("calendar date range is invalid".into()))?;
    if end < start
        || (end - start).whole_days() > 62
        || (query.detail && end != start)
        || !(1900..=2100).contains(&start.year())
        || !(1900..=2100).contains(&end.year())
    {
        return Err(AppError::Validation(
            "calendar date range is invalid".into(),
        ));
    }

    let preferences = CalendarRepository::new(state.db, state.vault)
        .preferences(session.user_id)
        .await?;
    let enabled_features = if query.detail {
        preferences.enabled_features
    } else {
        preferences
            .enabled_features
            .into_iter()
            .filter(|feature| {
                matches!(
                    feature,
                    CalendarFeature::LunarDate
                        | CalendarFeature::SolarFestival
                        | CalendarFeature::SolarOtherFestival
                        | CalendarFeature::HolidayAdjustment
                        | CalendarFeature::SolarTerm
                        | CalendarFeature::LunarFestival
                        | CalendarFeature::LunarOtherFestival
                )
            })
            .collect()
    };
    let mut date = start;
    let mut days = Vec::with_capacity(((end - start).whole_days() + 1) as usize);
    loop {
        days.push(lunar::day_info(date, &enabled_features));
        if date == end {
            break;
        }
        date = date
            .next_day()
            .ok_or_else(|| AppError::Validation("calendar date range is invalid".into()))?;
    }
    Ok(Json(days))
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
