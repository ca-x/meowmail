use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub notes: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactInput {
    pub display_name: String,
    pub email: String,
    #[serde(default)]
    pub notes: String,
}

impl ContactInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.display_name = clean_optional(&self.display_name, "display name", 160)?;
        self.email = clean_required(&self.email, "email", 254)?.to_ascii_lowercase();
        if !looks_like_email(&self.email) {
            return Err(AppError::Validation("contact email is invalid".into()));
        }
        self.notes = clean_optional(&self.notes, "notes", 4096)?;
        if self.display_name.is_empty() {
            self.display_name = self.email.clone();
        }
        Ok(())
    }
}

fn clean_required(value: &str, field: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!("{field} is invalid")));
    }
    Ok(value.to_owned())
}

fn clean_optional(value: &str, field: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.chars().count() > max || value.chars().any(|ch| ch == '\0') {
        return Err(AppError::Validation(format!("{field} is invalid")));
    }
    Ok(value.to_owned())
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !value.contains(['\r', '\n', ' ', '<', '>'])
}
