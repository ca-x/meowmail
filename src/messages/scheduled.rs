use std::time::Duration;

use tokio::time::MissedTickBehavior;

use crate::{
    AppState,
    error::AppError,
    mcp::{DraftRepository, EmailDraftStatus},
};

use super::send_outgoing;

pub fn spawn_runner(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            interval.tick().await;
            if let Err(error) = send_due(&state).await {
                tracing::warn!(error = %error, "scheduled draft runner failed");
            }
        }
    });
}

async fn send_due(state: &AppState) -> Result<(), AppError> {
    let repository = DraftRepository::new(state.db.clone());
    for due in repository.list_due_scheduled(20).await? {
        let draft_id = due.stored.draft.id;
        let user_id = due.user_id;
        let claimed = match repository.claim_for_send(user_id, draft_id).await {
            Ok(claimed) => claimed,
            Err(error) => {
                tracing::debug!(draft_id = %draft_id, error = %error, "scheduled draft was no longer claimable");
                continue;
            }
        };
        if let Err(error) = send_outgoing(
            state,
            user_id,
            claimed.clone().into_compose(),
            claimed.threading,
        )
        .await
        {
            tracing::warn!(draft_id = %draft_id, error = %error, "scheduled draft send failed");
            repository
                .mark_after_send_failure(user_id, draft_id, EmailDraftStatus::Draft)
                .await?;
            continue;
        }
        repository.finish_sent(user_id, draft_id).await?;
    }
    Ok(())
}
