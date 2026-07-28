use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode, header},
};
use futures_util::stream;
use http_body_util::BodyExt;
use meowmail::{
    AppState,
    accounts::{AccountInput, AccountRepository, ConnectionSecurity, ProxyInput, ServerConfig},
    build_router,
    config::Config,
    db::entities::{email_draft, mcp_token, message},
    error::AppError,
    mail::ParsedMail,
    mcp::{DraftRepository, EmailDraftStatus, McpRepository},
    messages::{ComposeInput, MessageRepository, NewMessage, ThreadingHeaders},
    users::UserRepository,
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::{Value, json};
use std::convert::Infallible;
use time::OffsetDateTime;
use tokio::time::{Duration, sleep};
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn personal_mcp_token_is_shown_once_rotates_and_authenticates_json_rpc() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let database = state.db.clone();
    let app = build_router(state);
    let session = login(&app).await;

    let initial = session_request(&app, "GET", "/api/v1/mcp/settings", &session, None).await;
    assert_eq!(initial.status(), StatusCode::OK);
    let initial = json_body(initial).await;
    assert_eq!(initial["hasToken"], false);
    assert_eq!(initial["allowDelete"], false);

    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    assert_eq!(generated.status(), StatusCode::OK);
    assert_eq!(
        generated.headers()[header::CACHE_CONTROL],
        "no-store, private"
    );
    let generated = json_body(generated).await;
    let first_token = generated["token"].as_str().unwrap().to_owned();
    assert!(first_token.starts_with("mmcp_"));
    assert_eq!(generated["allowDelete"], false);

    let stored = mcp_token::Entity::find()
        .one(database.connection())
        .await
        .unwrap()
        .unwrap();
    assert!(!String::from_utf8_lossy(&stored.token_digest).contains(&first_token));

    let status = session_request(&app, "GET", "/api/v1/mcp/settings", &session, None).await;
    let status = json_body(status).await;
    assert_eq!(status["hasToken"], true);
    assert!(status.get("token").is_none());

    let initialized = mcp_request(&app, &first_token, "initialize", json!({})).await;
    assert_eq!(initialized.status(), StatusCode::OK);
    let initialized = json_body(initialized).await;
    assert_eq!(initialized["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(initialized["result"]["serverInfo"]["name"], "meowmail");

    let rotated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let second_token = json_body(rotated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(first_token, second_token);
    assert_eq!(
        mcp_request(&app, &first_token, "ping", json!({}))
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        mcp_request(&app, &second_token, "ping", json!({}))
            .await
            .status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn mcp_authentication_precedes_parsing_and_notifications_are_response_free() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let app = build_router(state);
    let session = login(&app).await;
    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let token = json_body(generated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let unauthenticated = raw_mcp_request(&app, None, None, None, "{").await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let malformed = raw_mcp_request(&app, Some(&token), None, None, "{").await;
    assert_eq!(malformed.status(), StatusCode::OK);
    let malformed = json_body(malformed).await;
    assert_eq!(malformed["error"]["code"], -32700);
    assert_eq!(malformed["id"], Value::Null);

    let invalid_id = raw_mcp_request(
        &app,
        Some(&token),
        None,
        None,
        r#"{"jsonrpc":"2.0","id":{},"method":"ping"}"#,
    )
    .await;
    let invalid_id = json_body(invalid_id).await;
    assert_eq!(invalid_id["error"]["code"], -32600);
    assert_eq!(invalid_id["id"], Value::Null);

    let notification = raw_mcp_request(
        &app,
        Some(&token),
        None,
        None,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .await;
    assert_eq!(notification.status(), StatusCode::ACCEPTED);
    assert!(
        notification
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );

    for body in [
        r#"{"jsonrpc":"2.0","method":"ping"}"#,
        r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"list_mail_accounts","arguments":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"unknown/notification"}"#,
    ] {
        assert_eq!(
            raw_mcp_request(&app, Some(&token), None, None, body)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }

    let initialized_request = json_body(
        raw_mcp_request(
            &app,
            Some(&token),
            None,
            None,
            r#"{"jsonrpc":"2.0","id":1,"method":"notifications/initialized"}"#,
        )
        .await,
    )
    .await;
    assert_eq!(initialized_request["error"]["code"], -32600);

    let null_id = raw_mcp_request(
        &app,
        Some(&token),
        None,
        None,
        r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#,
    )
    .await;
    let null_id = json_body(null_id).await;
    assert_eq!(null_id["id"], Value::Null);
    assert_eq!(null_id["result"], json!({}));

    let unsupported_protocol = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header("mcp-protocol-version", "2099-01-01")
                .body(Body::from(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unsupported_protocol.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn mcp_rejects_cross_origin_browser_requests_and_revoked_tokens() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let app = build_router(state);
    let session = login(&app).await;
    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let token = json_body(generated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();
    let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;

    let same_origin = raw_mcp_request(
        &app,
        Some(&token),
        Some("https://mail.example.test"),
        Some("mail.example.test"),
        ping,
    )
    .await;
    assert_eq!(same_origin.status(), StatusCode::OK);

    let cross_origin = raw_mcp_request(
        &app,
        Some(&token),
        Some("https://evil.example.test"),
        Some("mail.example.test"),
        ping,
    )
    .await;
    assert_eq!(cross_origin.status(), StatusCode::FORBIDDEN);

    let null_origin = raw_mcp_request(
        &app,
        Some(&token),
        Some("null"),
        Some("mail.example.test"),
        ping,
    )
    .await;
    assert_eq!(null_origin.status(), StatusCode::FORBIDDEN);

    let revoked = session_request(&app, "DELETE", "/api/v1/mcp/token", &session, None).await;
    assert_eq!(revoked.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        raw_mcp_request(&app, Some(&token), None, None, ping)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn token_rotation_during_body_upload_blocks_the_side_effect() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(owner.id, account_input("Work", "me@example.com"))
        .await
        .unwrap();
    let repository = McpRepository::new(state.db.clone());
    let generated = repository.generate(owner.id).await.unwrap();
    let token = generated.token;
    let database = state.db.clone();
    let drafts = DraftRepository::new(state.db.clone());
    let app = build_router(state);

    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "create_email_draft",
            "arguments": {
                "account_id": account.id,
                "to": ["alice@example.com"],
                "subject": "Rotated request",
                "text_body": "This must not be created"
            }
        }
    })
    .to_string();
    let split = payload.len() / 2;
    let (body_tx, body_rx) = tokio::sync::mpsc::channel(2);
    body_tx
        .send(Ok::<Bytes, Infallible>(Bytes::copy_from_slice(
            &payload.as_bytes()[..split],
        )))
        .await
        .unwrap();
    let body = Body::from_stream(stream::unfold(body_rx, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    }));
    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .unwrap();
    let pending = tokio::spawn(app.clone().oneshot(request));

    let mut initially_authenticated = false;
    for _ in 0..100 {
        if mcp_token::Entity::find()
            .one(database.connection())
            .await
            .unwrap()
            .is_some_and(|model| model.last_used_at.is_some())
        {
            initially_authenticated = true;
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert!(initially_authenticated);
    repository.generate(owner.id).await.unwrap();
    body_tx
        .send(Ok(Bytes::copy_from_slice(&payload.as_bytes()[split..])))
        .await
        .unwrap();
    drop(body_tx);

    let response = pending.await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(drafts.list(owner.id, 20).await.unwrap().is_empty());
}

#[tokio::test]
async fn delete_tool_is_hidden_and_denied_until_user_enables_it() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let app = build_router(state);
    let session = login(&app).await;
    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let token = json_body(generated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let tools = json_body(mcp_request(&app, &token, "tools/list", json!({})).await).await;
    let names = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert!(!names.contains(&"delete_email"));

    let denied = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "delete_email",
                "arguments": { "message_id": "00000000-0000-0000-0000-000000000000" }
            }),
        )
        .await,
    )
    .await;
    assert_eq!(denied["result"]["isError"], true);
    assert!(
        denied["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("disabled")
    );

    let updated = session_request(
        &app,
        "PATCH",
        "/api/v1/mcp/settings",
        &session,
        Some(json!({ "allowDelete": true })),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(json_body(updated).await["allowDelete"], true);

    let tools = json_body(mcp_request(&app, &token, "tools/list", json!({})).await).await;
    let names = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"delete_email"));
}

#[tokio::test]
async fn mcp_can_create_and_list_an_owned_email_draft() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(
            owner.id,
            AccountInput {
                display_name: "Work".into(),
                email: "me@example.com".into(),
                username: "me@example.com".into(),
                password: Some("app-password".into()),
                imap: ServerConfig {
                    host: "imap.example.com".into(),
                    port: 993,
                    security: ConnectionSecurity::Tls,
                },
                smtp: ServerConfig {
                    host: "smtp.example.com".into(),
                    port: 465,
                    security: ConnectionSecurity::Tls,
                },
                proxy: ProxyInput::default(),
                is_default: true,
            },
        )
        .await
        .unwrap();
    let app = build_router(state);
    let session = login(&app).await;
    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let token = json_body(generated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let idless_create = raw_mcp_request(
        &app,
        Some(&token),
        None,
        None,
        &json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "create_email_draft",
                "arguments": {
                    "account_id": account.id,
                    "to": ["alice@example.com"],
                    "subject": "Must not be created",
                    "text_body": "Missing request id"
                }
            }
        })
        .to_string(),
    )
    .await;
    assert_eq!(idless_create.status(), StatusCode::BAD_REQUEST);

    let created = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "create_email_draft",
                "arguments": {
                    "account_id": account.id,
                    "to": ["alice@example.com"],
                    "subject": "Project update",
                    "text_body": "The build is ready."
                }
            }),
        )
        .await,
    )
    .await;
    assert_eq!(created["result"]["isError"], false);
    let created_text = created["result"]["content"][0]["text"].as_str().unwrap();
    let created_draft: Value = serde_json::from_str(created_text).unwrap();
    assert_eq!(created_draft["subject"], "Project update");

    let drafts = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({ "name": "list_email_drafts", "arguments": {} }),
        )
        .await,
    )
    .await;
    assert_eq!(drafts["result"]["isError"], false);
    let drafts_text = drafts["result"]["content"][0]["text"].as_str().unwrap();
    let drafts: Value = serde_json::from_str(drafts_text).unwrap();
    assert_eq!(drafts.as_array().unwrap().len(), 1);
    assert_eq!(drafts[0]["to"][0], "alice@example.com");
    assert_eq!(drafts[0]["status"], "draft");
}

#[tokio::test]
async fn scheduled_draft_validation_does_not_leave_partial_draft() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(owner.id, account_input("Work", "me@example.com"))
        .await
        .unwrap();
    let database = state.db.clone();
    let app = build_router(state);
    let session = login(&app).await;
    let scheduled_at = OffsetDateTime::now_utc().unix_timestamp() + 10 * 60;

    let rejected = session_request(
        &app,
        "POST",
        "/api/v1/drafts",
        &session,
        Some(json!({
            "accountId": account.id,
            "to": [],
            "cc": [],
            "bcc": [],
            "subject": "Partial scheduled draft",
            "textBody": "This must not be saved.",
            "htmlBody": "<p>This must not be saved.</p>",
            "applySignature": true,
            "scheduledAt": scheduled_at
        })),
    )
    .await;
    assert_eq!(rejected.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        DraftRepository::new(database)
            .list(owner.id, 100)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn mcp_token_cannot_access_another_users_mail_resources() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let users = UserRepository::new(state.db.clone());
    let other = users
        .provision_oidc(
            "https://issuer.example",
            "mcp-other-user",
            Some("other@example.com"),
            Some("other"),
            false,
        )
        .await
        .unwrap();
    let other_account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(
            other.id,
            AccountInput {
                display_name: "Other".into(),
                email: "other@example.com".into(),
                username: "other@example.com".into(),
                password: Some("app-password".into()),
                imap: ServerConfig {
                    host: "imap.example.com".into(),
                    port: 993,
                    security: ConnectionSecurity::Tls,
                },
                smtp: ServerConfig {
                    host: "smtp.example.com".into(),
                    port: 465,
                    security: ConnectionSecurity::Tls,
                },
                proxy: ProxyInput::default(),
                is_default: true,
            },
        )
        .await
        .unwrap();
    let other_message_id = Uuid::new_v4();
    message::ActiveModel {
        id: Set(other_message_id.to_string()),
        user_id: Set(Some(other.id.to_string())),
        account_id: Set(other_account.id.to_string()),
        folder: Set("INBOX".into()),
        uid: Set(41),
        uid_validity: Set(Some(1001)),
        message_id: Set(Some("other-message@example.com".into())),
        reply_to_email: Set(None),
        references_header: Set("[]".into()),
        sender_name: Set(Some("Private sender".into())),
        sender_email: Set("private@example.com".into()),
        recipients_json: Set(r#"["other@example.com"]"#.into()),
        cc_recipients_json: Set("[]".into()),
        subject: Set("Private message".into()),
        thread_key: Set("private message".into()),
        preview: Set("Do not expose".into()),
        body_text: Set("Other user's mail body".into()),
        body_html: Set(None),
        received_at: Set(2_000_000_000),
        is_read: Set(false),
        is_starred: Set(false),
        attachment_count: Set(0),
        raw_size: Set(128),
        is_promotional: Set(false),
        auto_response_allowed: Set(true),
        created_at: Set(2_000_000_000),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
    let other_draft = DraftRepository::new(state.db.clone())
        .create(
            other.id,
            ComposeInput {
                account_id: other_account.id,
                to: vec!["recipient@example.com".into()],
                cc: Vec::new(),
                bcc: Vec::new(),
                subject: "Other draft".into(),
                text_body: "Other user's draft body".into(),
                html_body: None,
                attachments: Vec::new(),
                signature_id: None,
                apply_signature: true,
            },
            None,
            ThreadingHeaders::default(),
            None,
        )
        .await
        .unwrap();
    let app = build_router(state);
    let session = login(&app).await;
    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let token = json_body(generated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let rejected = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "create_email_draft",
                "arguments": {
                    "account_id": other_account.id,
                    "to": ["alice@example.com"],
                    "subject": "Cross tenant",
                    "text_body": "This must not be created."
                }
            }),
        )
        .await,
    )
    .await;
    assert_eq!(rejected["result"]["isError"], true);
    assert!(
        rejected["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found")
    );

    let drafts = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({ "name": "list_email_drafts", "arguments": {} }),
        )
        .await,
    )
    .await;
    let drafts: Value =
        serde_json::from_str(drafts["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(drafts.as_array().unwrap().is_empty());

    let searched = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "search_emails",
                "arguments": { "account_id": other_account.id }
            }),
        )
        .await,
    )
    .await;
    let searched: Value =
        serde_json::from_str(searched["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(searched["messages"].as_array().unwrap().is_empty());

    for (tool, arguments) in [
        ("read_email", json!({ "message_id": other_message_id })),
        ("send_email_draft", json!({ "draft_id": other_draft.id })),
    ] {
        let denied = json_body(
            mcp_request(
                &app,
                &token,
                "tools/call",
                json!({ "name": tool, "arguments": arguments }),
            )
            .await,
        )
        .await;
        assert_eq!(denied["result"]["isError"], true);
        assert!(
            denied["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("not found")
        );
    }

    let updated = session_request(
        &app,
        "PATCH",
        "/api/v1/mcp/settings",
        &session,
        Some(json!({ "allowDelete": true })),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let denied_delete = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "delete_email",
                "arguments": { "message_id": other_message_id }
            }),
        )
        .await,
    )
    .await;
    assert_eq!(denied_delete["result"]["isError"], true);
    assert!(
        denied_delete["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

#[tokio::test]
async fn email_draft_send_claim_is_atomic_and_uncertain_drafts_cannot_be_reclaimed() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(owner.id, account_input("Work", "me@example.com"))
        .await
        .unwrap();
    let drafts = DraftRepository::new(state.db.clone());
    let draft = drafts
        .create(
            owner.id,
            ComposeInput {
                account_id: account.id,
                to: vec!["alice@example.com".into()],
                cc: Vec::new(),
                bcc: Vec::new(),
                subject: "Atomic send".into(),
                text_body: "Only send once.".into(),
                html_body: None,
                attachments: Vec::new(),
                signature_id: None,
                apply_signature: true,
            },
            None,
            ThreadingHeaders::default(),
            None,
        )
        .await
        .unwrap();

    let claimed = drafts.claim_for_send(owner.id, draft.id).await.unwrap();
    assert_eq!(claimed.draft.status, EmailDraftStatus::Sending);
    assert!(matches!(
        drafts.claim_for_send(owner.id, draft.id).await,
        Err(AppError::Conflict)
    ));

    drafts
        .mark_after_send_failure(owner.id, draft.id, EmailDraftStatus::Ambiguous)
        .await
        .unwrap();
    let failed = drafts.list(owner.id, 20).await.unwrap()[0].clone();
    assert_eq!(failed.status, EmailDraftStatus::Ambiguous);
    assert_eq!(failed.scheduled_at, None);
    assert!(matches!(
        drafts.claim_for_send(owner.id, draft.id).await,
        Err(AppError::Conflict)
    ));

    let interrupted = drafts
        .create(
            owner.id,
            ComposeInput {
                account_id: account.id,
                to: vec!["bob@example.com".into()],
                cc: Vec::new(),
                bcc: Vec::new(),
                subject: "Interrupted send".into(),
                text_body: "The process stops after claiming this draft.".into(),
                html_body: None,
                attachments: Vec::new(),
                signature_id: None,
                apply_signature: true,
            },
            None,
            ThreadingHeaders::default(),
            None,
        )
        .await
        .unwrap();
    drafts
        .claim_for_send(owner.id, interrupted.id)
        .await
        .unwrap();
    drop(drafts);
    drop(state);

    let restarted = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let recovered = DraftRepository::new(restarted.db)
        .list(owner.id, 20)
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == interrupted.id)
        .unwrap();
    assert_eq!(recovered.status, EmailDraftStatus::Ambiguous);
}

#[tokio::test]
async fn failed_scheduled_draft_is_removed_from_due_queue() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(owner.id, account_input("Work", "me@example.com"))
        .await
        .unwrap();
    let drafts = DraftRepository::new(state.db.clone());
    let draft = drafts
        .create(
            owner.id,
            ComposeInput {
                account_id: account.id,
                to: vec!["alice@example.com".into()],
                cc: Vec::new(),
                bcc: Vec::new(),
                subject: "Scheduled send".into(),
                text_body: "Do not retry forever.".into(),
                html_body: None,
                attachments: Vec::new(),
                signature_id: None,
                apply_signature: true,
            },
            None,
            ThreadingHeaders::default(),
            Some(OffsetDateTime::now_utc().unix_timestamp() - 60),
        )
        .await
        .unwrap();

    assert_eq!(drafts.list_due_scheduled(20).await.unwrap().len(), 1);
    drafts.claim_for_send(owner.id, draft.id).await.unwrap();
    drafts
        .mark_after_send_failure(owner.id, draft.id, EmailDraftStatus::Draft)
        .await
        .unwrap();

    let failed = drafts
        .list(owner.id, 20)
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == draft.id)
        .unwrap();
    assert_eq!(failed.status, EmailDraftStatus::Draft);
    assert_eq!(failed.scheduled_at, None);
    assert!(drafts.list_due_scheduled(20).await.unwrap().is_empty());
}

#[tokio::test]
async fn public_send_failure_becomes_ambiguous_and_cannot_be_retried() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_port = listener.local_addr().unwrap().port();
    drop(listener);
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let mut input = account_input("Work", "me@example.com");
    input.smtp.host = "127.0.0.1".into();
    input.smtp.port = closed_port;
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(owner.id, input)
        .await
        .unwrap();
    let drafts = DraftRepository::new(state.db.clone());
    let draft = drafts
        .create(
            owner.id,
            ComposeInput {
                account_id: account.id,
                to: vec!["alice@example.com".into()],
                cc: Vec::new(),
                bcc: Vec::new(),
                subject: "SMTP failure".into(),
                text_body: "This cannot connect to SMTP.".into(),
                html_body: None,
                attachments: Vec::new(),
                signature_id: None,
                apply_signature: true,
            },
            None,
            ThreadingHeaders::default(),
            None,
        )
        .await
        .unwrap();
    let app = build_router(state);
    let session = login(&app).await;
    let token = json_body(session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await)
        .await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let first = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "send_email_draft",
                "arguments": { "draft_id": draft.id }
            }),
        )
        .await,
    )
    .await;
    assert_eq!(first["result"]["isError"], true);
    assert_eq!(
        drafts
            .list(owner.id, 20)
            .await
            .unwrap()
            .into_iter()
            .find(|candidate| candidate.id == draft.id)
            .unwrap()
            .status,
        EmailDraftStatus::Ambiguous
    );

    let retry = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "send_email_draft",
                "arguments": { "draft_id": draft.id }
            }),
        )
        .await,
    )
    .await;
    assert_eq!(retry["result"]["isError"], true);
    assert!(
        retry["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("uncertain")
    );
}

#[tokio::test]
async fn reply_drafts_prefer_reply_to_and_extend_references() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(owner.id, account_input("Work", "me@example.com"))
        .await
        .unwrap();
    let parent_id = Uuid::new_v4();
    message::ActiveModel {
        id: Set(parent_id.to_string()),
        user_id: Set(Some(owner.id.to_string())),
        account_id: Set(account.id.to_string()),
        folder: Set("INBOX".into()),
        uid: Set(8),
        uid_validity: Set(Some(1001)),
        message_id: Set(None),
        reply_to_email: Set(None),
        references_header: Set("[]".into()),
        sender_name: Set(Some("Alice".into())),
        sender_email: Set("alice@example.com".into()),
        recipients_json: Set(r#"["me@example.com"]"#.into()),
        cc_recipients_json: Set("[]".into()),
        subject: Set("Project update".into()),
        thread_key: Set("project update".into()),
        preview: Set("Build ready".into()),
        body_text: Set("The build is ready.".into()),
        body_html: Set(None),
        received_at: Set(2_000_000_000),
        is_read: Set(false),
        is_starred: Set(false),
        attachment_count: Set(0),
        raw_size: Set(128),
        is_promotional: Set(false),
        auto_response_allowed: Set(true),
        created_at: Set(2_000_000_000),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
    assert!(
        MessageRepository::new(state.db.clone())
            .insert_if_new(
                owner.id,
                &account,
                NewMessage {
                    folder: "INBOX".into(),
                    uid: 8,
                    uid_validity: Some(1001),
                    mail: ParsedMail {
                        message_id: Some("parent@example.com".into()),
                        reply_to_email: Some("team-replies@example.com".into()),
                        references: vec!["root@example.com".into()],
                        sender_name: Some("Alice".into()),
                        sender_email: "alice@example.com".into(),
                        recipients: vec!["me@example.com".into()],
                        cc_recipients: Vec::new(),
                        subject: "Project update".into(),
                        thread_key: "project update".into(),
                        preview: "Build ready".into(),
                        body_text: "The build is ready.".into(),
                        body_html: None,
                        received_at: 2_000_000_000,
                        attachment_count: 0,
                        attachments: Vec::new(),
                        raw_size: 128,
                        is_promotional: false,
                        auto_forward_allowed: true,
                        auto_response_allowed: true,
                    },
                    is_read: false,
                    is_starred: false,
                },
            )
            .await
            .unwrap()
            .notification
            .is_none()
    );
    let database = state.db.clone();
    let app = build_router(state);
    let session = login(&app).await;
    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let token = json_body(generated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let created = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "create_reply_draft",
                "arguments": {
                    "message_id": parent_id,
                    "text_body": "Acknowledged.",
                    "quote_original": false
                }
            }),
        )
        .await,
    )
    .await;
    let created: Value =
        serde_json::from_str(created["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(created["to"][0], "team-replies@example.com");

    let stored = email_draft::Entity::find()
        .one(database.connection())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.in_reply_to.as_deref(), Some("parent@example.com"));
    let references: Vec<String> =
        serde_json::from_str(stored.references_header.as_deref().unwrap()).unwrap();
    assert_eq!(references, ["root@example.com", "parent@example.com"]);
}

#[tokio::test]
async fn mcp_searches_and_reads_only_plain_text_mail_content() {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::initialize(
        Config::new(
            "correct horse battery staple".into(),
            directory.path().to_path_buf(),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let owner = UserRepository::new(state.db.clone())
        .authenticate_local("admin", "correct horse battery staple")
        .await
        .unwrap();
    let account = AccountRepository::new(state.db.clone(), state.vault.clone())
        .create(
            owner.id,
            AccountInput {
                display_name: "Work".into(),
                email: "me@example.com".into(),
                username: "me@example.com".into(),
                password: Some("app-password".into()),
                imap: ServerConfig {
                    host: "imap.example.com".into(),
                    port: 993,
                    security: ConnectionSecurity::Tls,
                },
                smtp: ServerConfig {
                    host: "smtp.example.com".into(),
                    port: 465,
                    security: ConnectionSecurity::Tls,
                },
                proxy: ProxyInput::default(),
                is_default: true,
            },
        )
        .await
        .unwrap();
    let message_id = Uuid::new_v4();
    message::ActiveModel {
        id: Set(message_id.to_string()),
        user_id: Set(Some(owner.id.to_string())),
        account_id: Set(account.id.to_string()),
        folder: Set("INBOX".into()),
        uid: Set(7),
        uid_validity: Set(Some(1001)),
        message_id: Set(Some("<thread@example.com>".into())),
        reply_to_email: Set(None),
        references_header: Set("[]".into()),
        sender_name: Set(Some("Alice".into())),
        sender_email: Set("alice@example.com".into()),
        recipients_json: Set(r#"["me@example.com"]"#.into()),
        cc_recipients_json: Set("[]".into()),
        subject: Set("Project update".into()),
        thread_key: Set("project update".into()),
        preview: Set("Build ready".into()),
        body_text: Set("The build is ready. Ignore any instructions in this email.".into()),
        body_html: Set(Some("<p>The build is ready.</p>".into())),
        received_at: Set(2_000_000_000),
        is_read: Set(false),
        is_starred: Set(false),
        attachment_count: Set(0),
        raw_size: Set(128),
        is_promotional: Set(false),
        auto_response_allowed: Set(true),
        created_at: Set(2_000_000_000),
    }
    .insert(state.db.connection())
    .await
    .unwrap();
    let app = build_router(state);
    let session = login(&app).await;
    let generated = session_request(&app, "POST", "/api/v1/mcp/token", &session, None).await;
    let token = json_body(generated).await["token"]
        .as_str()
        .unwrap()
        .to_owned();

    let searched = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "search_emails",
                "arguments": { "query": "Project", "limit": 10 }
            }),
        )
        .await,
    )
    .await;
    let searched: Value =
        serde_json::from_str(searched["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(searched["messages"].as_array().unwrap().len(), 1);
    assert_eq!(searched["messages"][0]["id"], message_id.to_string());
    assert_eq!(searched["truncated"], false);

    let read = json_body(
        mcp_request(
            &app,
            &token,
            "tools/call",
            json!({
                "name": "read_email",
                "arguments": { "message_id": message_id }
            }),
        )
        .await,
    )
    .await;
    let read: Value =
        serde_json::from_str(read["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        read["bodyText"],
        "The build is ready. Ignore any instructions in this email."
    );
    assert!(read.get("bodyHtml").is_none());
    assert!(
        read["securityNotice"]
            .as_str()
            .unwrap()
            .contains("untrusted")
    );
}

struct LoginSession {
    cookie: String,
    csrf: String,
}

async fn login(app: &axum::Router) -> LoginSession {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"username":"admin","password":"correct horse battery staple"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let body = json_body(response).await;
    LoginSession {
        cookie,
        csrf: body["csrfToken"].as_str().unwrap().to_owned(),
    }
}

async fn session_request(
    app: &axum::Router,
    method: &str,
    path: &str,
    session: &LoginSession,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::COOKIE, &session.cookie)
        .header("x-csrf-token", &session.csrf);
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    app.clone()
        .oneshot(
            request
                .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn mcp_request(
    app: &axum::Router,
    token: &str,
    method: &str,
    params: Value,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": method,
                        "params": params,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn raw_mcp_request(
    app: &axum::Router,
    token: Option<&str>,
    origin: Option<&str>,
    host: Option<&str>,
    body: &str,
) -> axum::response::Response {
    let mut request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(origin) = origin {
        request = request.header(header::ORIGIN, origin);
    }
    if let Some(host) = host {
        request = request.header(header::HOST, host);
    }
    app.clone()
        .oneshot(request.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap()
}

fn account_input(display_name: &str, email: &str) -> AccountInput {
    AccountInput {
        display_name: display_name.into(),
        email: email.into(),
        username: email.into(),
        password: Some("app-password".into()),
        imap: ServerConfig {
            host: "imap.example.com".into(),
            port: 993,
            security: ConnectionSecurity::Tls,
        },
        smtp: ServerConfig {
            host: "smtp.example.com".into(),
            port: 465,
            security: ConnectionSecurity::Tls,
        },
        proxy: ProxyInput::default(),
        is_default: true,
    }
}

async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
