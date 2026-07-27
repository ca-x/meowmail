use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
};

use super::{CleanupRepository, CleanupRule, CleanupRuleInput, MailSettings};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MailSettingsUpdate {
    #[serde(default)]
    keep_local_after_server_delete: PatchField<bool>,
    #[serde(default)]
    sync_fetch_limit: PatchField<Option<u32>>,
}

#[derive(Default)]
enum PatchField<T> {
    #[default]
    Missing,
    Value(T),
}

impl<'de, T> Deserialize<'de> for PatchField<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::Value)
    }
}

impl MailSettingsUpdate {
    fn apply(self, current: MailSettings) -> MailSettings {
        MailSettings {
            keep_local_after_server_delete: match self.keep_local_after_server_delete {
                PatchField::Missing => current.keep_local_after_server_delete,
                PatchField::Value(value) => value,
            },
            sync_fetch_limit: match self.sync_fetch_limit {
                PatchField::Missing => current.sync_fetch_limit,
                PatchField::Value(value) => value,
            },
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/mail/settings", get(settings).patch(update_settings))
        .route("/cleanup/rules", get(list_rules).post(create_rule))
        .route("/cleanup/rules/reorder", axum::routing::put(reorder_rules))
        .route(
            "/cleanup/rules/{id}",
            axum::routing::patch(update_rule).delete(delete_rule),
        )
}

#[derive(Deserialize)]
struct ReorderRules {
    ids: Vec<Uuid>,
}

async fn reorder_rules(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<ReorderRules>,
) -> Result<Json<Vec<CleanupRule>>, AppError> {
    Ok(Json(
        CleanupRepository::new(state.db)
            .reorder(mutation.0.user_id, &input.ids)
            .await?,
    ))
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
    Json(update): Json<MailSettingsUpdate>,
) -> Result<Json<MailSettings>, AppError> {
    let repository = CleanupRepository::new(state.db);
    let current = repository.settings(mutation.0.user_id).await?;
    Ok(Json(
        repository
            .update_settings(mutation.0.user_id, update.apply(current))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{MailSettings, MailSettingsUpdate};

    #[test]
    fn omitted_sync_limit_preserves_the_existing_value() {
        let update: MailSettingsUpdate =
            serde_json::from_value(json!({ "keepLocalAfterServerDelete": false })).unwrap();
        let settings = update.apply(MailSettings {
            keep_local_after_server_delete: true,
            sync_fetch_limit: None,
        });
        assert!(!settings.keep_local_after_server_delete);
        assert_eq!(settings.sync_fetch_limit, None);
    }

    #[test]
    fn explicit_null_selects_full_mailbox_sync() {
        let update: MailSettingsUpdate =
            serde_json::from_value(json!({ "syncFetchLimit": null })).unwrap();
        let settings = update.apply(MailSettings {
            keep_local_after_server_delete: true,
            sync_fetch_limit: Some(50),
        });
        assert_eq!(settings.sync_fetch_limit, None);
    }
}
