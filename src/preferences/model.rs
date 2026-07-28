use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ReadingMode {
    List,
    #[default]
    Preview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ListDensity {
    #[default]
    Default,
    Compact,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum AfterAction {
    #[default]
    NextMessage,
    MessageList,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SubjectPrefixLanguage {
    Chinese,
    #[default]
    English,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ComposeFontFamily {
    #[default]
    Default,
    Serif,
    Monospace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct MailPreferences {
    pub reading_mode: ReadingMode,
    pub list_density: ListDensity,
    pub conversation_mode: bool,
    pub aggregate_promotions: bool,
    pub show_summary: bool,
    pub show_message_size: bool,
    pub show_attachment_preview: bool,
    pub after_action: AfterAction,
    pub plain_text_reading: bool,
    pub attach_original_on_reply: bool,
    pub subject_prefix_language: SubjectPrefixLanguage,
    pub empty_subject_from_body: bool,
    pub compose_font_family: ComposeFontFamily,
    pub compose_font_size: u8,
    pub compose_font_color: String,
    pub auto_forward_enabled: bool,
    pub auto_forward_address: Option<String>,
    pub auto_reply_enabled: bool,
    pub auto_reply_subject: String,
    pub auto_reply_text: String,
    pub auto_reply_start_at: Option<i64>,
    pub auto_reply_end_at: Option<i64>,
    pub auto_reply_account_ids: Vec<Uuid>,
    pub auto_reply_contacts_only: bool,
}

impl Default for MailPreferences {
    fn default() -> Self {
        Self {
            reading_mode: ReadingMode::Preview,
            list_density: ListDensity::Default,
            conversation_mode: false,
            aggregate_promotions: true,
            show_summary: true,
            show_message_size: false,
            show_attachment_preview: true,
            after_action: AfterAction::NextMessage,
            plain_text_reading: false,
            attach_original_on_reply: true,
            subject_prefix_language: SubjectPrefixLanguage::English,
            empty_subject_from_body: false,
            compose_font_family: ComposeFontFamily::Default,
            compose_font_size: 14,
            compose_font_color: "#1A1A1A".into(),
            auto_forward_enabled: false,
            auto_forward_address: None,
            auto_reply_enabled: false,
            auto_reply_subject: String::new(),
            auto_reply_text: String::new(),
            auto_reply_start_at: None,
            auto_reply_end_at: None,
            auto_reply_account_ids: Vec::new(),
            auto_reply_contacts_only: false,
        }
    }
}

impl MailPreferences {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        if !(11..=24).contains(&self.compose_font_size) {
            return Err(AppError::Validation("compose font size is invalid".into()));
        }
        self.compose_font_color = self.compose_font_color.trim().to_ascii_uppercase();
        if self.compose_font_color.len() != 7
            || !self.compose_font_color.starts_with('#')
            || !self.compose_font_color[1..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(AppError::Validation("compose font color is invalid".into()));
        }
        self.auto_forward_address = self
            .auto_forward_address
            .take()
            .map(|value| value.trim().to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        if self.auto_forward_address.as_ref().is_some_and(|value| {
            value.len() > 254
                || !value.contains('@')
                || value.contains(['\r', '\n'])
                || value.chars().any(char::is_control)
        }) {
            return Err(AppError::Validation(
                "auto-forward address is invalid".into(),
            ));
        }
        if self.auto_forward_enabled && self.auto_forward_address.is_none() {
            return Err(AppError::Validation(
                "auto-forward address is required".into(),
            ));
        }
        self.auto_reply_subject = self.auto_reply_subject.trim().to_owned();
        if self.auto_reply_subject.len() > 998 || self.auto_reply_subject.contains(['\r', '\n']) {
            return Err(AppError::Validation("auto-reply subject is invalid".into()));
        }
        self.auto_reply_text = self.auto_reply_text.trim().to_owned();
        if self.auto_reply_text.len() > 32 * 1024
            || self.auto_reply_text.chars().any(|value| value == '\0')
        {
            return Err(AppError::Validation("auto-reply text is invalid".into()));
        }
        if self
            .auto_reply_start_at
            .is_some_and(|value| !(0..=4_102_444_800).contains(&value))
            || self
                .auto_reply_end_at
                .is_some_and(|value| !(0..=4_102_444_800).contains(&value))
            || self
                .auto_reply_start_at
                .zip(self.auto_reply_end_at)
                .is_some_and(|(start, end)| end < start)
        {
            return Err(AppError::Validation(
                "auto-reply schedule is invalid".into(),
            ));
        }
        self.auto_reply_account_ids.sort_unstable();
        self.auto_reply_account_ids.dedup();
        if self.auto_reply_enabled && self.auto_reply_text.is_empty() {
            return Err(AppError::Validation("auto-reply text is required".into()));
        }
        Ok(())
    }

    pub fn auto_reply_subject_for(&self, subject: &str) -> String {
        if self.auto_reply_subject.is_empty() {
            return prefixed_subject(self.reply_prefix(), subject);
        }
        self.auto_reply_subject.clone()
    }

    pub fn is_auto_reply_active_at(&self, timestamp: i64) -> bool {
        self.auto_reply_enabled
            && self
                .auto_reply_start_at
                .is_none_or(|start| timestamp >= start)
            && self.auto_reply_end_at.is_none_or(|end| timestamp <= end)
    }

    pub fn applies_to_auto_reply_account(&self, account_id: Uuid) -> bool {
        self.auto_reply_account_ids.is_empty() || self.auto_reply_account_ids.contains(&account_id)
    }

    pub fn reply_prefix(&self) -> &'static str {
        match self.subject_prefix_language {
            SubjectPrefixLanguage::Chinese => "回复：",
            SubjectPrefixLanguage::English => "Re:",
        }
    }

    pub fn forward_prefix(&self) -> &'static str {
        match self.subject_prefix_language {
            SubjectPrefixLanguage::Chinese => "转发：",
            SubjectPrefixLanguage::English => "Fwd:",
        }
    }
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    pub id: Uuid,
    pub name: String,
    pub body_text: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureInput {
    pub name: String,
    pub body_text: String,
}

impl SignatureInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.name = self.name.trim().to_owned();
        self.body_text = self.body_text.trim().to_owned();
        if self.name.is_empty()
            || self.name.chars().count() > 120
            || self.name.chars().any(char::is_control)
        {
            return Err(AppError::Validation("signature name is invalid".into()));
        }
        if self.body_text.len() > 64 * 1024 || self.body_text.chars().any(|value| value == '\0') {
            return Err(AppError::Validation("signature body is invalid".into()));
        }
        Ok(())
    }
}
