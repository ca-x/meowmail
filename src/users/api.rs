use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, HeaderValue, header},
    response::IntoResponse,
    routing::get,
};

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
};

use super::migration::{
    ExportRequest, ImportReport, ImportRequest, MigrationArchive, MigrationService,
};
use super::{PublicUser, UserAiAccessInput, UserPasswordInput, UserProfile, UserRepository};

const MAX_AVATAR_SIZE: usize = 512 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users/me", get(profile).patch(update_profile))
        .route("/users/me/password", axum::routing::put(update_password))
        .route("/users/me/ai", axum::routing::put(update_ai_access))
        .route(
            "/users/me/avatar",
            get(avatar).put(update_avatar).delete(remove_avatar),
        )
        .route(
            "/users/migration/export",
            axum::routing::post(export_config),
        )
        .route(
            "/users/migration/import",
            axum::routing::post(import_config),
        )
}

async fn profile(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<PublicUser>, AppError> {
    Ok(Json(
        UserRepository::new(state.db).get(session.user_id).await?,
    ))
}

async fn update_profile(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<UserProfile>,
) -> Result<Json<PublicUser>, AppError> {
    Ok(Json(
        UserRepository::new(state.db)
            .update_profile(
                mutation.0.user_id,
                input.username.as_deref(),
                &input.nickname,
            )
            .await?,
    ))
}

async fn update_password(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<UserPasswordInput>,
) -> Result<Json<PublicUser>, AppError> {
    let _password_guard = state
        .password_locks
        .try_lock(mutation.0.user_id, mutation.0.user_id)
        .ok_or(AppError::RateLimited)?;
    let user = UserRepository::new(state.db)
        .update_password(
            mutation.0.user_id,
            input.current_password.as_deref(),
            &input.new_password,
        )
        .await?;
    state.sessions.revoke_user(mutation.0.user_id);
    Ok(Json(user))
}

async fn update_ai_access(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<UserAiAccessInput>,
) -> Result<Json<PublicUser>, AppError> {
    Ok(Json(
        UserRepository::new(state.db)
            .set_ai_enabled(mutation.0.user_id, input.enabled)
            .await?,
    ))
}

async fn avatar(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<impl IntoResponse, AppError> {
    let model = UserRepository::new(state.db)
        .get_model(session.user_id)
        .await?;
    let mime = model.avatar_mime.ok_or(AppError::NotFound)?;
    let data = model.avatar_data.ok_or(AppError::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&mime).map_err(AppError::internal)?,
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=300"),
    );
    Ok((headers, data))
}

async fn update_avatar(
    State(state): State<AppState>,
    mutation: MutationSession,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<PublicUser>, AppError> {
    if body.is_empty() || body.len() > MAX_AVATAR_SIZE {
        return Err(AppError::Validation("avatar size is invalid".into()));
    }
    let supplied = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .unwrap_or("");
    let detected = detect_avatar_mime(&body)
        .ok_or_else(|| AppError::Validation("avatar must be a PNG, JPEG, or WebP image".into()))?;
    if supplied != detected {
        return Err(AppError::Validation(
            "avatar content type does not match its data".into(),
        ));
    }
    Ok(Json(
        UserRepository::new(state.db)
            .set_avatar(
                mutation.0.user_id,
                Some(detected.into()),
                Some(body.to_vec()),
            )
            .await?,
    ))
}

async fn remove_avatar(
    State(state): State<AppState>,
    mutation: MutationSession,
) -> Result<Json<PublicUser>, AppError> {
    Ok(Json(
        UserRepository::new(state.db)
            .set_avatar(mutation.0.user_id, None, None)
            .await?,
    ))
}

fn detect_avatar_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if data.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

async fn export_config(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(request): Json<ExportRequest>,
) -> Result<Json<MigrationArchive>, AppError> {
    let user = UserRepository::new(state.db.clone())
        .get(mutation.0.user_id)
        .await?;
    if request.sections.ai && !user.ai_enabled {
        return Err(AppError::Forbidden);
    }
    Ok(Json(
        MigrationService::new(state.db, state.vault)
            .export(&user, request)
            .await?,
    ))
}

async fn import_config(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(request): Json<ImportRequest>,
) -> Result<Json<ImportReport>, AppError> {
    let user = UserRepository::new(state.db.clone())
        .get(mutation.0.user_id)
        .await?;
    if request.sections.ai && !user.ai_enabled {
        return Err(AppError::Forbidden);
    }
    Ok(Json(
        MigrationService::new(state.db, state.vault)
            .import(&user, request)
            .await?,
    ))
}
