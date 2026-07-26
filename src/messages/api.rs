use async_imap::types::Flag;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, post},
};
use futures_util::TryStreamExt;
use mail_builder::MessageBuilder;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    AppState,
    accounts::AccountRepository,
    auth::{MutationSession, require_session},
    cleanup::CleanupRepository,
    error::AppError,
    mail::{connect_imap_session, parse_message, send_smtp},
};

use super::repository::{
    MessageDetail, MessageFilter, MessageRepository, MessageSummary, NewMessage,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/accounts/{id}/sync", post(sync_account))
        .route("/messages", get(list_messages))
        .route("/messages/{id}", get(get_message).patch(update_message))
        .route("/messages/send", post(send_message))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    account_id: Option<Uuid>,
    folder: Option<String>,
    #[serde(default)]
    unread: bool,
    #[serde(default)]
    starred: bool,
    #[serde(default)]
    has_attachment: bool,
    q: Option<String>,
    limit: Option<u64>,
}

async fn list_messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<MessageSummary>>, AppError> {
    let session = require_session(&state, &headers)?;
    let folder = query.folder.unwrap_or_else(|| "INBOX".into());
    if folder.is_empty() || folder.len() > 160 {
        return Err(AppError::Validation("folder is invalid".into()));
    }
    let search = query
        .q
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if search.as_ref().is_some_and(|value| value.len() > 200) {
        return Err(AppError::Validation("search query is too long".into()));
    }
    Ok(Json(
        MessageRepository::new(state.db)
            .list(
                session.user_id,
                MessageFilter {
                    account_id: query.account_id,
                    folder,
                    unread: query.unread,
                    starred: query.starred,
                    has_attachment: query.has_attachment,
                    query: search,
                    limit: query.limit.unwrap_or(80),
                },
            )
            .await?,
    ))
}

async fn get_message(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<MessageDetail>, AppError> {
    let session = require_session(&state, &headers)?;
    Ok(Json(
        MessageRepository::new(state.db)
            .get(session.user_id, id)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlagUpdate {
    is_read: Option<bool>,
    is_starred: Option<bool>,
}

async fn update_message(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    mutation: MutationSession,
    Json(update): Json<FlagUpdate>,
) -> Result<Json<MessageSummary>, AppError> {
    if update.is_read.is_none() && update.is_starred.is_none() {
        return Err(AppError::Validation(
            "no message change was supplied".into(),
        ));
    }
    Ok(Json(
        MessageRepository::new(state.db)
            .update_flags(mutation.0.user_id, id, update.is_read, update.is_starred)
            .await?,
    ))
}

async fn sync_account(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    mutation: MutationSession,
) -> Result<Json<SyncResponse>, AppError> {
    let accounts = AccountRepository::new(state.db.clone(), state.vault.clone());
    let (account, secrets, proxy) = accounts.get_with_secrets(mutation.0.user_id, id).await?;
    let mut session = connect_imap_session(&account, &secrets, &proxy)
        .await
        .map_err(|error| AppError::Mail(error.to_string()))?;
    let mailbox = session
        .select("INBOX")
        .await
        .map_err(|error| AppError::Mail(error.to_string()))?;
    let cleanup = CleanupRepository::new(state.db.clone());
    let settings = cleanup.settings(mutation.0.user_id).await?;
    let rules = cleanup
        .enabled_for_account(mutation.0.user_id, account.id)
        .await?;
    let server_uids = if settings.keep_local_after_server_delete {
        None
    } else {
        Some(
            session
                .uid_search("ALL")
                .await
                .map_err(|error| AppError::Mail(error.to_string()))?,
        )
    };
    let mut inserted = 0_u32;
    let mut removed = 0_u64;
    let mut server_delete_uids = Vec::new();
    if mailbox.exists > 0 {
        let start = mailbox.exists.saturating_sub(49).max(1);
        let sequence = format!("{start}:{}", mailbox.exists);
        let repository = MessageRepository::new(state.db.clone());
        let mut notifications = Vec::new();
        {
            let mut fetches = session
                .fetch(sequence, "(UID FLAGS BODY.PEEK[])")
                .await
                .map_err(|error| AppError::Mail(error.to_string()))?;
            while let Some(fetch) = fetches
                .try_next()
                .await
                .map_err(|error| AppError::Mail(error.to_string()))?
            {
                let Some(uid) = fetch.uid else { continue };
                let Some(raw) = fetch.body() else { continue };
                let Some(parsed) = parse_message(raw, OffsetDateTime::now_utc().unix_timestamp())
                else {
                    continue;
                };
                if let Some(rule) = CleanupRepository::match_new_mail(
                    &rules,
                    &parsed,
                    OffsetDateTime::now_utc().unix_timestamp(),
                ) {
                    if rule.delete_from_server {
                        server_delete_uids.push(i64::from(uid));
                    }
                    continue;
                }
                let flags = fetch.flags().collect::<Vec<_>>();
                if let Some(event) = repository
                    .insert_if_new(
                        mutation.0.user_id,
                        &account,
                        NewMessage {
                            folder: "INBOX".into(),
                            uid: i64::from(uid),
                            mail: parsed,
                            is_read: flags.contains(&Flag::Seen),
                            is_starred: flags.contains(&Flag::Flagged),
                        },
                    )
                    .await?
                {
                    inserted += 1;
                    notifications.push(event);
                }
            }
        }
        for event in notifications {
            state.notifications.dispatch(event);
        }
    }
    server_delete_uids.extend(
        cleanup
            .apply_cached_rules(
                mutation.0.user_id,
                account.id,
                &rules,
                OffsetDateTime::now_utc().unix_timestamp(),
            )
            .await?,
    );
    server_delete_uids.sort_unstable();
    server_delete_uids.dedup();
    if !server_delete_uids.is_empty() {
        let uid_set = server_delete_uids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        session
            .uid_store(&uid_set, "+FLAGS.SILENT (\\Deleted)")
            .await
            .map_err(|error| AppError::Mail(error.to_string()))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|error| AppError::Mail(error.to_string()))?;
        session
            .uid_expunge(&uid_set)
            .await
            .map_err(|error| AppError::Mail(error.to_string()))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|error| AppError::Mail(error.to_string()))?;
    }
    if let Some(server_uids) = server_uids.as_ref() {
        removed += cleanup
            .reconcile_server_uids(mutation.0.user_id, account.id, server_uids)
            .await?;
    }
    let _ = session.logout().await;
    let synced_at = OffsetDateTime::now_utc().unix_timestamp();
    accounts
        .mark_synced(mutation.0.user_id, id, synced_at)
        .await?;
    Ok(Json(SyncResponse {
        inserted,
        removed,
        synced_at,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncResponse {
    inserted: u32,
    removed: u64,
    synced_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComposeRequest {
    account_id: Uuid,
    to: Vec<String>,
    #[serde(default)]
    cc: Vec<String>,
    #[serde(default)]
    bcc: Vec<String>,
    subject: String,
    text_body: String,
    html_body: Option<String>,
}

async fn send_message(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(mut input): Json<ComposeRequest>,
) -> Result<axum::http::StatusCode, AppError> {
    validate_compose(&mut input)?;
    let accounts = AccountRepository::new(state.db, state.vault);
    let (account, secrets, proxy) = accounts
        .get_with_secrets(mutation.0.user_id, input.account_id)
        .await?;
    let mut builder = MessageBuilder::new()
        .from((account.display_name.clone(), account.email.clone()))
        .to(input.to.clone())
        .subject(input.subject.clone())
        .text_body(input.text_body.clone());
    if !input.cc.is_empty() {
        builder = builder.cc(input.cc.clone());
    }
    if let Some(html) = input.html_body {
        builder = builder.html_body(html);
    }
    let message = builder.write_to_vec().map_err(AppError::internal)?;
    let mut recipients = input.to;
    recipients.extend(input.cc);
    recipients.extend(input.bcc);
    send_smtp(
        &account,
        &secrets,
        &proxy,
        &account.email,
        &recipients,
        &message,
    )
    .await
    .map_err(|error| AppError::Mail(error.to_string()))?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn validate_compose(input: &mut ComposeRequest) -> Result<(), AppError> {
    if input.to.is_empty() || input.to.len() + input.cc.len() + input.bcc.len() > 100 {
        return Err(AppError::Validation("recipient count is invalid".into()));
    }
    for address in input
        .to
        .iter_mut()
        .chain(&mut input.cc)
        .chain(&mut input.bcc)
    {
        *address = address.trim().to_ascii_lowercase();
        if address.len() > 254 || address.contains(['\r', '\n']) || !address.contains('@') {
            return Err(AppError::Validation("recipient address is invalid".into()));
        }
    }
    input.subject = input.subject.trim().to_owned();
    if input.subject.len() > 998 || input.subject.contains(['\r', '\n']) {
        return Err(AppError::Validation("subject is invalid".into()));
    }
    if input.text_body.len() > 2 * 1024 * 1024
        || input
            .html_body
            .as_ref()
            .is_some_and(|value| value.len() > 2 * 1024 * 1024)
    {
        return Err(AppError::Validation("message body is too large".into()));
    }
    Ok(())
}
