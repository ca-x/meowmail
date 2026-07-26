use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    routing::{get, post},
};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use time::OffsetDateTime;

use crate::{
    AppState,
    auth::{MutationSession, require_session},
    db::entities::notification_setting,
    error::AppError,
};

use super::{NotificationSettings, runner::validate_settings};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/notifications/settings",
            get(get_settings).patch(update_settings),
        )
        .route("/notifications/test", post(test_settings))
}

async fn get_settings(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<NotificationSettings>, AppError> {
    require_session(&state, &headers)?;
    Ok(Json(load(&state).await?))
}

async fn update_settings(
    State(state): State<AppState>,
    _mutation: MutationSession,
    Json(mut settings): Json<NotificationSettings>,
) -> Result<Json<NotificationSettings>, AppError> {
    normalize(&mut settings);
    validate_settings(&settings)?;
    let model = notification_setting::Entity::find_by_id(1)
        .one(state.db.connection())
        .await?
        .ok_or_else(|| AppError::internal(anyhow::anyhow!("notification settings are missing")))?;
    let mut active = model.into_active_model();
    active.enabled = Set(settings.enabled);
    active.message_template = Set(settings.message_template.clone());
    active.command_template = Set(settings.command_template.clone());
    active.http_url = Set(settings.http_url.clone());
    active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
    active.update(state.db.connection()).await?;
    Ok(Json(settings))
}

async fn test_settings(
    State(state): State<AppState>,
    _mutation: MutationSession,
    Json(mut settings): Json<NotificationSettings>,
) -> Result<axum::http::StatusCode, AppError> {
    normalize(&mut settings);
    state.notifications.test(&settings).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn load(state: &AppState) -> Result<NotificationSettings, AppError> {
    let model = notification_setting::Entity::find_by_id(1)
        .one(state.db.connection())
        .await?
        .ok_or_else(|| AppError::internal(anyhow::anyhow!("notification settings are missing")))?;
    Ok(NotificationSettings {
        enabled: model.enabled,
        message_template: model.message_template,
        command_template: model.command_template,
        http_url: model.http_url,
    })
}

fn normalize(settings: &mut NotificationSettings) {
    settings.message_template = settings.message_template.trim().to_owned();
    settings.command_template = settings
        .command_template
        .take()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    settings.http_url = settings
        .http_url
        .take()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
}
