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
    pub auto_reply_text: String,
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
            auto_reply_text: String::new(),
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
        self.auto_reply_text = self.auto_reply_text.trim().to_owned();
        if self.auto_reply_text.len() > 32 * 1024
            || self.auto_reply_text.chars().any(|value| value == '\0')
        {
            return Err(AppError::Validation("auto-reply text is invalid".into()));
        }
        if self.auto_reply_enabled && self.auto_reply_text.is_empty() {
            return Err(AppError::Validation("auto-reply text is required".into()));
        }
        Ok(())
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
