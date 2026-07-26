use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailSettings {
    pub keep_local_after_server_delete: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRule {
    pub id: Uuid,
    pub account_id: Option<Uuid>,
    pub name: String,
    pub sender_contains: Option<String>,
    pub subject_contains: Option<String>,
    pub body_contains: Option<String>,
    pub older_than_days: Option<u32>,
    pub delete_from_server: bool,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRuleInput {
    pub account_id: Option<Uuid>,
    pub name: String,
    pub sender_contains: Option<String>,
    pub subject_contains: Option<String>,
    pub body_contains: Option<String>,
    pub older_than_days: Option<u32>,
    #[serde(default)]
    pub delete_from_server: bool,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

impl CleanupRuleInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "rule name", 120)?;
        self.sender_contains = clean_optional(self.sender_contains.take(), "sender filter", 320)?;
        self.subject_contains =
            clean_optional(self.subject_contains.take(), "subject filter", 998)?;
        self.body_contains = clean_optional(self.body_contains.take(), "body filter", 2_000)?;
        if self
            .older_than_days
            .is_some_and(|days| days == 0 || days > 36_500)
        {
            return Err(AppError::Validation("mail age is invalid".into()));
        }
        if self.sender_contains.is_none()
            && self.subject_contains.is_none()
            && self.body_contains.is_none()
            && self.older_than_days.is_none()
        {
            return Err(AppError::Validation(
                "a cleanup rule needs at least one condition".into(),
            ));
        }
        Ok(())
    }
}

fn clean_required(value: &str, label: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!("{label} is invalid")));
    }
    Ok(value.to_owned())
}

fn clean_optional(
    value: Option<String>,
    label: &str,
    max: usize,
) -> Result<Option<String>, AppError> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .map(|value| clean_required(&value, label, max))
        .transpose()
}
