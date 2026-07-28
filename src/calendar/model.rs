use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarAccount {
    pub id: Uuid,
    pub name: String,
    pub base_url: String,
    pub username: String,
    pub enabled: bool,
    pub has_password: bool,
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarAccountInput {
    pub name: String,
    pub base_url: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: Uuid,
    pub account_id: Uuid,
    pub display_name: String,
    pub color: String,
    pub remote_href: String,
    pub sync_token: Option<String>,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarUpdate {
    pub display_name: String,
    pub color: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarEvent {
    pub id: Uuid,
    pub calendar_id: Uuid,
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub location: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub all_day: bool,
    pub timezone: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct ParsedEvent {
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub location: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub all_day: bool,
    pub timezone: Option<String>,
}

fn enabled_default() -> bool {
    true
}

impl CalendarAccountInput {
    pub fn normalize(&mut self, require_password: bool) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "calendar account name", 120)?;
        self.base_url = clean_url(&self.base_url)?;
        self.username = clean_required(&self.username, "calendar username", 320)?;
        self.password = self
            .password
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if self.password.as_ref().is_some_and(|value| {
            value.len() > 4096 || value.chars().any(|character| character == '\0')
        }) {
            return Err(AppError::Validation("calendar password is invalid".into()));
        }
        if require_password && self.password.is_none() {
            return Err(AppError::Validation("calendar password is required".into()));
        }
        Ok(())
    }
}

impl CalendarUpdate {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.display_name = clean_required(&self.display_name, "calendar name", 160)?;
        self.color = self.color.trim().to_owned();
        if self.color.is_empty()
            || self.color.len() > 40
            || self.color.chars().any(|character| character.is_control())
        {
            return Err(AppError::Validation("calendar color is invalid".into()));
        }
        Ok(())
    }
}

fn clean_url(raw: &str) -> Result<String, AppError> {
    let value = raw.trim().trim_end_matches('/');
    let url = url::Url::parse(value)
        .map_err(|_| AppError::Validation("calendar URL is invalid".into()))?;
    if !matches!(url.scheme(), "https" | "http") || url.username() != "" || url.password().is_some()
    {
        return Err(AppError::Validation("calendar URL is invalid".into()));
    }
    Ok(value.to_owned())
}

fn clean_required(value: &str, field: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!("{field} is invalid")));
    }
    Ok(value.to_owned())
}
