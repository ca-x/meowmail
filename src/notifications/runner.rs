use std::{path::Path, process::Stdio, time::Duration};

use reqwest::redirect::Policy;
use sea_orm::EntityTrait;
use serde_json::json;
use tokio::{process::Command, time::timeout};
use uuid::Uuid;

use crate::{
    db::{Database, entities::notification_setting},
    error::AppError,
};

use super::{NotificationEvent, NotificationSettings, render_template};

const HOOK_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct NotificationRunner {
    db: Database,
    client: reqwest::Client,
}

impl NotificationRunner {
    pub fn new(db: Database) -> Self {
        let client = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(HOOK_TIMEOUT)
            .user_agent(concat!("meowmail/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("notification HTTP client configuration is valid");
        Self { db, client }
    }

    pub fn dispatch(&self, event: NotificationEvent) {
        let runner = self.clone();
        tokio::spawn(async move {
            if let Err(error) = runner.dispatch_inner(&event).await {
                tracing::warn!(error = %error, "new-mail notification hook failed");
            }
        });
    }

    pub async fn test(
        &self,
        user_id: Uuid,
        settings: &NotificationSettings,
    ) -> Result<(), AppError> {
        validate_settings(settings)?;
        self.run(
            settings,
            &NotificationEvent {
                user_id,
                account: "Work".into(),
                email: "me@example.com".into(),
                sender: "Meowmail".into(),
                sender_email: "hello@meowmail.local".into(),
                subject: "Notification test".into(),
                preview: "Your Meowmail notification settings are working.".into(),
            },
        )
        .await
    }

    async fn dispatch_inner(&self, event: &NotificationEvent) -> Result<(), AppError> {
        let model = notification_setting::Entity::find_by_id(event.user_id.to_string())
            .one(self.db.connection())
            .await?
            .ok_or_else(|| {
                AppError::internal(anyhow::anyhow!("notification settings are missing"))
            })?;
        let settings = NotificationSettings {
            enabled: model.enabled,
            message_template: model.message_template,
            command_template: model.command_template,
            http_url: model.http_url,
        };
        if settings.enabled {
            self.run(&settings, event).await?;
        }
        Ok(())
    }

    async fn run(
        &self,
        settings: &NotificationSettings,
        event: &NotificationEvent,
    ) -> Result<(), AppError> {
        validate_settings(settings)?;
        let message = render_template(&settings.message_template, event, None)?;
        if let Some(command_template) = settings.command_template.as_deref() {
            self.run_command(command_template, event, &message).await?;
        }
        if let Some(http_url) = settings.http_url.as_deref() {
            self.run_http(http_url, event, &message).await?;
        }
        Ok(())
    }

    async fn run_command(
        &self,
        template: &str,
        event: &NotificationEvent,
        message: &str,
    ) -> Result<(), AppError> {
        let tokens = shell_words::split(template)
            .map_err(|_| AppError::Validation("notification command has invalid quoting".into()))?;
        let (program, arguments) = tokens
            .split_first()
            .ok_or_else(|| AppError::Validation("notification command is empty".into()))?;
        if program.contains('{') || !Path::new(program).is_absolute() {
            return Err(AppError::Validation(
                "notification command executable must be an absolute path without placeholders"
                    .into(),
            ));
        }
        let rendered = arguments
            .iter()
            .map(|argument| render_template(argument, event, Some(message)))
            .collect::<Result<Vec<_>, _>>()?;
        let status = timeout(
            HOOK_TIMEOUT,
            Command::new(program)
                .args(rendered)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .status(),
        )
        .await
        .map_err(|_| AppError::Mail("notification command timed out".into()))?
        .map_err(AppError::internal)?;
        if !status.success() {
            return Err(AppError::Mail("notification command failed".into()));
        }
        Ok(())
    }

    async fn run_http(
        &self,
        url: &str,
        event: &NotificationEvent,
        message: &str,
    ) -> Result<(), AppError> {
        let response = self
            .client
            .post(url)
            .json(&json!({
                "message": message,
                "account": event.account,
                "email": event.email,
                "sender": event.sender,
                "senderEmail": event.sender_email,
                "subject": event.subject,
                "preview": event.preview,
            }))
            .send()
            .await
            .map_err(|_| AppError::Mail("notification HTTP request failed".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Mail(format!(
                "notification HTTP endpoint returned {}",
                response.status().as_u16()
            )));
        }
        Ok(())
    }
}

pub fn validate_settings(settings: &NotificationSettings) -> Result<(), AppError> {
    if settings.message_template.is_empty() || settings.message_template.len() > 2_000 {
        return Err(AppError::Validation(
            "notification message template is invalid".into(),
        ));
    }
    if settings.message_template.contains("{message}") {
        return Err(AppError::Validation(
            "{message} cannot reference itself in the message template".into(),
        ));
    }
    let sample = NotificationEvent {
        user_id: Uuid::nil(),
        account: "account".into(),
        email: "mail@example.com".into(),
        sender: "sender".into(),
        sender_email: "sender@example.com".into(),
        subject: "subject".into(),
        preview: "preview".into(),
    };
    let message = render_template(&settings.message_template, &sample, None)?;
    if let Some(command) = settings.command_template.as_deref() {
        if command.len() > 4_096 {
            return Err(AppError::Validation(
                "notification command is too long".into(),
            ));
        }
        let tokens = shell_words::split(command)
            .map_err(|_| AppError::Validation("notification command has invalid quoting".into()))?;
        let (program, args) = tokens
            .split_first()
            .ok_or_else(|| AppError::Validation("notification command is empty".into()))?;
        if program.contains('{') || !Path::new(program).is_absolute() {
            return Err(AppError::Validation(
                "notification command executable must be an absolute path without placeholders"
                    .into(),
            ));
        }
        for argument in args {
            render_template(argument, &sample, Some(&message))?;
        }
    }
    if let Some(http_url) = settings.http_url.as_deref() {
        if http_url.len() > 2_048 || http_url.contains('{') {
            return Err(AppError::Validation(
                "notification HTTP URL is invalid".into(),
            ));
        }
        let url = url::Url::parse(http_url)
            .map_err(|_| AppError::Validation("notification HTTP URL is invalid".into()))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(AppError::Validation(
                "notification HTTP URL must be a fixed HTTP/HTTPS address without credentials"
                    .into(),
            ));
        }
    }
    if settings.enabled && settings.command_template.is_none() && settings.http_url.is_none() {
        return Err(AppError::Validation(
            "enable at least one notification target".into(),
        ));
    }
    Ok(())
}
