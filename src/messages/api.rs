use std::collections::HashSet;

use async_imap::types::Flag;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    AppState,
    accounts::AccountRepository,
    auth::{MutationSession, require_session},
    cleanup::{CleanupRepository, RuleOutcome},
    error::AppError,
    mail::{connect_imap_session, parse_message},
    preferences::{MailPreferences, PreferencesRepository},
};

use super::repository::{
    MessageDetail, MessageFilter, MessageRepository, MessageSummary, NewMessage,
};
use super::{AutomaticMessageKind, ComposeInput, ThreadingHeaders, send_outgoing};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/accounts/{id}/sync", post(sync_account))
        .route("/messages", get(list_messages))
        .route(
            "/messages/{id}",
            get(get_message)
                .patch(update_message)
                .delete(delete_message),
        )
        .route("/messages/{id}/thread", get(get_thread))
        .route(
            "/messages/{message_id}/attachments/{attachment_id}",
            get(get_attachment),
        )
        .route("/messages/send", post(send_message))
}

async fn get_thread(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<Vec<MessageDetail>>, AppError> {
    let session = require_session(&state, &headers)?;
    Ok(Json(
        MessageRepository::new(state.db)
            .thread(session.user_id, id)
            .await?,
    ))
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
struct AttachmentQuery {
    #[serde(default)]
    download: bool,
}

async fn get_attachment(
    State(state): State<AppState>,
    Path((message_id, attachment_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<AttachmentQuery>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    let session = require_session(&state, &headers)?;
    let attachment = MessageRepository::new(state.db)
        .get_attachment(session.user_id, message_id, attachment_id)
        .await?;
    let content_type = HeaderValue::from_str(&attachment.content_type)
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))?;
    let disposition = format!(
        "{}; filename=\"{}\"; filename*=UTF-8''{}",
        if query.download {
            "attachment"
        } else {
            "inline"
        },
        ascii_attachment_filename(&attachment.filename),
        percent_encode_filename(&attachment.filename),
    );
    let disposition = HeaderValue::from_str(&disposition)
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))?;
    let content_length = HeaderValue::from_str(&attachment.content.len().to_string())
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))?;
    let mut response = Body::from(attachment.content).into_response();
    let response_headers = response.headers_mut();
    response_headers.insert(header::CONTENT_TYPE, content_type);
    response_headers.insert(header::CONTENT_DISPOSITION, disposition);
    response_headers.insert(header::CONTENT_LENGTH, content_length);
    response_headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok(response)
}

fn percent_encode_filename(filename: &str) -> String {
    let mut encoded = String::with_capacity(filename.len());
    for byte in filename.as_bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            )
        {
            encoded.push(char::from(*byte));
        } else {
            use std::fmt::Write;
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

fn ascii_attachment_filename(filename: &str) -> String {
    let value = filename
        .chars()
        .take(120)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, ' ' | '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if value.trim_matches([' ', '.']).is_empty() {
        "attachment".into()
    } else {
        value
    }
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

async fn delete_message(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    mutation: MutationSession,
) -> Result<axum::http::StatusCode, AppError> {
    MessageRepository::new(state.db)
        .delete_local(mutation.0.user_id, id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
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
    let preferences = PreferencesRepository::new(state.db.clone())
        .mail(mutation.0.user_id)
        .await?;
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
    let mut automatic_actions = Vec::new();
    if mailbox.exists > 0 {
        let start = sync_sequence_start(mailbox.exists, settings.sync_fetch_limit);
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
                let outcome = CleanupRepository::match_new_mail(
                    &rules,
                    &parsed,
                    OffsetDateTime::now_utc().unix_timestamp(),
                );
                if outcome.delete_local || outcome.delete_server {
                    if outcome.delete_server {
                        server_delete_uids.push(i64::from(uid));
                    }
                    continue;
                }
                let flags = fetch.flags().collect::<Vec<_>>();
                let parsed_for_actions = parsed.clone();
                let result = repository
                    .insert_if_new(
                        mutation.0.user_id,
                        &account,
                        NewMessage {
                            folder: "INBOX".into(),
                            uid: i64::from(uid),
                            uid_validity: mailbox.uid_validity.map(i64::from),
                            mail: parsed,
                            is_read: outcome
                                .is_read
                                .unwrap_or_else(|| flags.contains(&Flag::Seen)),
                            is_starred: outcome
                                .is_starred
                                .unwrap_or_else(|| flags.contains(&Flag::Flagged)),
                        },
                    )
                    .await?;
                if let Some(event) = result.notification {
                    inserted += 1;
                    notifications.push(event);
                }
                if result.created {
                    automatic_actions.push((parsed_for_actions, outcome));
                }
            }
        }
        for event in notifications {
            state.notifications.dispatch(event);
        }
    }
    let mut automatic_failures = 0_u32;
    for (mail, outcome) in automatic_actions {
        automatic_failures += run_automatic_actions(
            &state,
            mutation.0.user_id,
            account.id,
            &account.email,
            &preferences,
            &mail,
            outcome,
        )
        .await;
    }
    let cached_cleanup = cleanup
        .apply_cached_rules(
            mutation.0.user_id,
            account.id,
            mailbox.uid_validity.map(i64::from),
            &rules,
            OffsetDateTime::now_utc().unix_timestamp(),
        )
        .await?;
    server_delete_uids.extend(cached_cleanup.server_uids.iter().copied());
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
        removed += cleanup
            .delete_cached_after_server_success(
                mutation.0.user_id,
                &cached_cleanup.server_message_ids,
            )
            .await?;
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
        automatic_failures,
        synced_at,
    }))
}

fn sync_sequence_start(exists: u32, limit: Option<u32>) -> u32 {
    match limit {
        None => 1,
        Some(limit) => exists.saturating_sub(limit.saturating_sub(1)).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::sync_sequence_start;

    #[test]
    fn sync_sequence_uses_configured_recent_count() {
        assert_eq!(sync_sequence_start(100, Some(50)), 51);
        assert_eq!(sync_sequence_start(20, Some(50)), 1);
    }

    #[test]
    fn sync_sequence_can_include_the_entire_mailbox() {
        assert_eq!(sync_sequence_start(100, None), 1);
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncResponse {
    inserted: u32,
    removed: u64,
    automatic_failures: u32,
    synced_at: i64,
}

async fn run_automatic_actions(
    state: &AppState,
    user_id: Uuid,
    account_id: Uuid,
    account_email: &str,
    preferences: &MailPreferences,
    mail: &crate::mail::ParsedMail,
    mut outcome: RuleOutcome,
) -> u32 {
    if !mail.auto_forward_allowed {
        outcome.forwards.clear();
    }
    if preferences.auto_forward_enabled
        && mail.auto_forward_allowed
        && let Some(address) = preferences.auto_forward_address.clone()
    {
        outcome.forwards.push(address);
    }
    if preferences.auto_reply_enabled
        && mail.auto_response_allowed
        && !mail.sender_email.eq_ignore_ascii_case(account_email)
    {
        outcome
            .auto_replies
            .push(preferences.auto_reply_text.clone());
    }
    let mut failures = 0_u32;
    let mut forwards = HashSet::new();
    for address in outcome.forwards {
        let address = address.to_ascii_lowercase();
        if !forwards.insert(address.clone()) || address.eq_ignore_ascii_case(account_email) {
            continue;
        }
        let subject = prefixed_subject(preferences.forward_prefix(), &mail.subject);
        let sender = mail
            .sender_name
            .as_ref()
            .map(|name| format!("{name} <{}>", mail.sender_email))
            .unwrap_or_else(|| mail.sender_email.clone());
        let body = format!(
            "\n\n---------- Forwarded message ----------\nFrom: {sender}\nSubject: {}\n\n{}",
            mail.subject, mail.body_text
        );
        if let Err(error) = send_outgoing(
            state,
            user_id,
            ComposeInput {
                account_id,
                to: vec![address],
                cc: Vec::new(),
                bcc: Vec::new(),
                subject,
                text_body: body,
                html_body: None,
            },
            ThreadingHeaders {
                automatic: Some(AutomaticMessageKind::Forward),
                ..ThreadingHeaders::default()
            },
        )
        .await
        {
            failures = failures.saturating_add(1);
            tracing::warn!(error = %error, "automatic mail forward failed");
        }
    }
    let mut replies = HashSet::new();
    for body in outcome.auto_replies {
        if !mail.auto_response_allowed || !replies.insert(body.clone()) {
            continue;
        }
        let mut references = mail.references.clone();
        if let Some(message_id) = mail.message_id.clone()
            && !references.contains(&message_id)
        {
            references.push(message_id);
        }
        if let Err(error) = send_outgoing(
            state,
            user_id,
            ComposeInput {
                account_id,
                to: vec![
                    mail.reply_to_email
                        .clone()
                        .unwrap_or_else(|| mail.sender_email.clone()),
                ],
                cc: Vec::new(),
                bcc: Vec::new(),
                subject: prefixed_subject(preferences.reply_prefix(), &mail.subject),
                text_body: body,
                html_body: None,
            },
            ThreadingHeaders {
                in_reply_to: mail.message_id.clone(),
                references,
                automatic: Some(AutomaticMessageKind::Reply),
            },
        )
        .await
        {
            failures = failures.saturating_add(1);
            tracing::warn!(error = %error, "automatic mail reply failed");
        }
    }
    failures
}

fn prefixed_subject(prefix: &str, subject: &str) -> String {
    let value = subject.trim();
    if value.is_empty() {
        return prefix.into();
    }
    let lower = value.to_ascii_lowercase();
    let already_prefixed = lower.starts_with("re:")
        || lower.starts_with("fw:")
        || lower.starts_with("fwd:")
        || value.starts_with("回复：")
        || value.starts_with("转发：");
    if already_prefixed {
        value.into()
    } else if prefix.ends_with('：') {
        format!("{prefix}{value}")
    } else {
        format!("{prefix} {value}")
    }
}

async fn send_message(
    State(state): State<AppState>,
    mutation: MutationSession,
    Json(input): Json<ComposeInput>,
) -> Result<axum::http::StatusCode, AppError> {
    send_outgoing(
        &state,
        mutation.0.user_id,
        input,
        ThreadingHeaders::default(),
    )
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
