use axum::{
    Json, Router,
    extract::{FromRequestParts, Path, State},
    http::request::Parts,
    routing::get,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    auth::{AuthenticatedSession, MutationSession},
    error::AppError,
    messages::MessageRepository,
    users::UserRepository,
};

use super::{
    AiProvider, AiProviderInput, AiRepository, AiService, AiTextRequest, AiTextResponse,
    AutoLabelResult, AutoLabelRule, AutoLabelRuleFeed, AutoLabelRuleInput, AutoLabelSubscription,
    AutoLabelSubscriptionInput, AutoLabelSubscriptionService, AutoLabelSubscriptionSyncResult,
    Label, LabelInput,
};

struct AiSession {
    user_id: Uuid,
}

impl FromRequestParts<AppState> for AiSession {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session = AuthenticatedSession::from_request_parts(parts, state).await?;
        require_ai_enabled(state, session.user_id).await?;
        Ok(Self {
            user_id: session.user_id,
        })
    }
}

struct AiMutation {
    user_id: Uuid,
}

impl FromRequestParts<AppState> for AiMutation {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let mutation = MutationSession::from_request_parts(parts, state).await?;
        require_ai_enabled(state, mutation.0.user_id).await?;
        Ok(Self {
            user_id: mutation.0.user_id,
        })
    }
}

async fn require_ai_enabled(state: &AppState, user_id: Uuid) -> Result<(), AppError> {
    if UserRepository::new(state.db.clone())
        .get(user_id)
        .await?
        .ai_enabled
    {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/ai/providers", get(list_providers).post(create_provider))
        .route(
            "/ai/providers/{id}",
            axum::routing::patch(update_provider).delete(delete_provider),
        )
        .route(
            "/ai/providers/{id}/test",
            axum::routing::post(test_provider),
        )
        .route("/ai/translate", axum::routing::post(translate))
        .route("/ai/polish", axum::routing::post(polish))
        .route("/labels", get(list_labels).post(create_label))
        .route(
            "/labels/{id}",
            axum::routing::patch(update_label).delete(delete_label),
        )
        .route(
            "/auto-label-rules",
            get(list_auto_label_rules).post(create_auto_label_rule),
        )
        .route("/auto-label-rules/export", get(export_auto_label_rules))
        .route(
            "/auto-label-rules/{id}",
            axum::routing::patch(update_auto_label_rule).delete(delete_auto_label_rule),
        )
        .route(
            "/auto-label-subscriptions",
            get(list_auto_label_subscriptions).post(create_auto_label_subscription),
        )
        .route(
            "/auto-label-subscriptions/{id}",
            axum::routing::patch(update_auto_label_subscription)
                .delete(delete_auto_label_subscription),
        )
        .route(
            "/auto-label-subscriptions/{id}/sync",
            axum::routing::post(sync_auto_label_subscription),
        )
        .route(
            "/messages/{id}/auto-label",
            axum::routing::post(auto_label_message),
        )
}

async fn list_providers(
    State(state): State<AppState>,
    session: AiSession,
) -> Result<Json<Vec<AiProvider>>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .list_providers(session.user_id)
            .await?,
    ))
}

async fn create_provider(
    State(state): State<AppState>,
    mutation: AiMutation,
    Json(input): Json<AiProviderInput>,
) -> Result<Json<AiProvider>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .create_provider(mutation.user_id, input)
            .await?,
    ))
}

async fn update_provider(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
    Json(input): Json<AiProviderInput>,
) -> Result<Json<AiProvider>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .update_provider(mutation.user_id, id, input)
            .await?,
    ))
}

async fn delete_provider(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    AiRepository::new(state.db, state.vault)
        .delete_provider(mutation.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn test_provider(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
) -> Result<Json<AiTextResponse>, AppError> {
    Ok(Json(AiTextResponse {
        text: AiService::new(AiRepository::new(state.db, state.vault))
            .test_provider(mutation.user_id, id)
            .await?,
    }))
}

async fn translate(
    State(state): State<AppState>,
    mutation: AiMutation,
    Json(input): Json<AiTextRequest>,
) -> Result<Json<AiTextResponse>, AppError> {
    Ok(Json(AiTextResponse {
        text: AiService::new(AiRepository::new(state.db, state.vault))
            .translate(
                mutation.user_id,
                input.provider_id,
                &input.text,
                input.target_language.as_deref(),
            )
            .await?,
    }))
}

async fn polish(
    State(state): State<AppState>,
    mutation: AiMutation,
    Json(input): Json<AiTextRequest>,
) -> Result<Json<AiTextResponse>, AppError> {
    Ok(Json(AiTextResponse {
        text: AiService::new(AiRepository::new(state.db, state.vault))
            .polish(
                mutation.user_id,
                input.provider_id,
                &input.text,
                input.tone.as_deref(),
            )
            .await?,
    }))
}

async fn list_labels(
    State(state): State<AppState>,
    session: AiSession,
) -> Result<Json<Vec<Label>>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .list_labels(session.user_id)
            .await?,
    ))
}

async fn create_label(
    State(state): State<AppState>,
    mutation: AiMutation,
    Json(input): Json<LabelInput>,
) -> Result<Json<Label>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .create_label(mutation.user_id, input)
            .await?,
    ))
}

async fn update_label(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
    Json(input): Json<LabelInput>,
) -> Result<Json<Label>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .update_label(mutation.user_id, id, input)
            .await?,
    ))
}

async fn delete_label(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    AiRepository::new(state.db, state.vault)
        .delete_label(mutation.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn list_auto_label_rules(
    State(state): State<AppState>,
    session: AiSession,
) -> Result<Json<Vec<AutoLabelRule>>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .list_auto_label_rules(session.user_id)
            .await?,
    ))
}

async fn create_auto_label_rule(
    State(state): State<AppState>,
    mutation: AiMutation,
    Json(input): Json<AutoLabelRuleInput>,
) -> Result<Json<AutoLabelRule>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .create_auto_label_rule(mutation.user_id, input)
            .await?,
    ))
}

async fn export_auto_label_rules(
    State(state): State<AppState>,
    session: AiSession,
) -> Result<Json<AutoLabelRuleFeed>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .export_auto_label_feed(session.user_id)
            .await?,
    ))
}

async fn update_auto_label_rule(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
    Json(input): Json<AutoLabelRuleInput>,
) -> Result<Json<AutoLabelRule>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .update_auto_label_rule(mutation.user_id, id, input)
            .await?,
    ))
}

async fn delete_auto_label_rule(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    AiRepository::new(state.db, state.vault)
        .delete_auto_label_rule(mutation.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn list_auto_label_subscriptions(
    State(state): State<AppState>,
    session: AiSession,
) -> Result<Json<Vec<AutoLabelSubscription>>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .list_auto_label_subscriptions(session.user_id)
            .await?,
    ))
}

async fn create_auto_label_subscription(
    State(state): State<AppState>,
    mutation: AiMutation,
    Json(input): Json<AutoLabelSubscriptionInput>,
) -> Result<Json<AutoLabelSubscription>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .create_auto_label_subscription(mutation.user_id, input)
            .await?,
    ))
}

async fn update_auto_label_subscription(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
    Json(input): Json<AutoLabelSubscriptionInput>,
) -> Result<Json<AutoLabelSubscription>, AppError> {
    Ok(Json(
        AiRepository::new(state.db, state.vault)
            .update_auto_label_subscription(mutation.user_id, id, input)
            .await?,
    ))
}

async fn delete_auto_label_subscription(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    AiRepository::new(state.db, state.vault)
        .delete_auto_label_subscription(mutation.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn sync_auto_label_subscription(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
) -> Result<Json<AutoLabelSubscriptionSyncResult>, AppError> {
    Ok(Json(
        AutoLabelSubscriptionService::new(AiRepository::new(state.db, state.vault))
            .sync(mutation.user_id, id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AutoLabelMessageInput {
    #[serde(default)]
    rule_id: Option<Uuid>,
}

async fn auto_label_message(
    State(state): State<AppState>,
    mutation: AiMutation,
    Path(id): Path<Uuid>,
    Json(input): Json<AutoLabelMessageInput>,
) -> Result<Json<AutoLabelResult>, AppError> {
    let user_id = mutation.user_id;
    let repository = AiRepository::new(state.db.clone(), state.vault.clone());
    let message = MessageRepository::new(state.db.clone())
        .get(user_id, id)
        .await?;
    let rules = repository.list_auto_label_rules(user_id).await?;
    let enabled_subscriptions = repository
        .list_auto_label_subscriptions(user_id)
        .await?
        .into_iter()
        .filter(|subscription| subscription.enabled)
        .map(|subscription| subscription.id)
        .collect::<std::collections::HashSet<_>>();
    let Some(rule) = rules.into_iter().find(|rule| {
        rule.enabled
            && rule
                .source_subscription_id
                .is_none_or(|id| enabled_subscriptions.contains(&id))
            && input.rule_id.is_none_or(|id| id == rule.id)
    }) else {
        return Err(AppError::Validation(
            "auto-label rule is not configured".into(),
        ));
    };
    let labels = repository.labels_by_ids(user_id, &rule.label_ids).await?;
    let label_ids = AiService::new(repository.clone())
        .classify_message(
            user_id,
            rule.provider_id,
            &message,
            &labels,
            &rule.instructions,
        )
        .await?;
    Ok(Json(
        repository.apply_labels(user_id, id, &label_ids).await?,
    ))
}
