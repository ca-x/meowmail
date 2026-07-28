use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    accounts::{ProxyInput, ProxyKind, PublicProxyConfig},
    error::AppError,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiProviderKind {
    OpenAi,
    Claude,
    Gemini,
}

impl AiProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "openai" => Ok(Self::OpenAi),
            "claude" => Ok(Self::Claude),
            "gemini" => Ok(Self::Gemini),
            _ => Err(AppError::Validation("AI provider type is invalid".into())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AiApiType {
    Chat,
    Responses,
    Messages,
    GenerateContent,
}

impl AiApiType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Responses => "responses",
            Self::Messages => "messages",
            Self::GenerateContent => "generateContent",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "chat" => Ok(Self::Chat),
            "responses" => Ok(Self::Responses),
            "messages" => Ok(Self::Messages),
            "generateContent" => Ok(Self::GenerateContent),
            _ => Err(AppError::Validation("AI API type is invalid".into())),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProvider {
    pub id: Uuid,
    pub name: String,
    pub provider_kind: AiProviderKind,
    pub api_type: AiApiType,
    pub model: String,
    pub base_url: Option<String>,
    pub proxy: PublicProxyConfig,
    pub is_default: bool,
    pub enabled: bool,
    pub has_api_key: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderInput {
    pub name: String,
    pub provider_kind: AiProviderKind,
    pub api_type: AiApiType,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub proxy: ProxyInput,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

fn enabled_default() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTextResponse {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTextRequest {
    #[serde(default)]
    pub provider_id: Option<Uuid>,
    pub text: String,
    #[serde(default)]
    pub target_language: Option<String>,
    #[serde(default)]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub is_auto: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LabelInput {
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub is_auto: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelRule {
    pub id: Uuid,
    pub account_id: Option<Uuid>,
    pub provider_id: Option<Uuid>,
    pub name: String,
    pub label_ids: Vec<Uuid>,
    pub instructions: String,
    pub enabled: bool,
    pub apply_automatically: bool,
    pub source_subscription_id: Option<Uuid>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelRuleInput {
    #[serde(default)]
    pub account_id: Option<Uuid>,
    #[serde(default)]
    pub provider_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub label_ids: Vec<Uuid>,
    #[serde(default)]
    pub instructions: String,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    #[serde(default)]
    pub apply_automatically: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelSubscription {
    pub id: Uuid,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub last_synced_at: Option<i64>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelSubscriptionInput {
    pub name: String,
    pub url: String,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelSubscriptionSyncResult {
    pub subscription: AutoLabelSubscription,
    pub labels_imported: u32,
    pub rules_imported: u32,
    pub rules_skipped: u32,
}

pub const AUTO_LABEL_FEED_FORMAT: &str = "meowmail-auto-label-rules";
pub const AUTO_LABEL_FEED_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelRuleFeed {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub labels: Vec<AutoLabelFeedLabel>,
    #[serde(default)]
    pub rules: Vec<AutoLabelFeedRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelFeedLabel {
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub is_auto: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelFeedRule {
    #[serde(default)]
    pub account_email: Option<String>,
    #[serde(default)]
    pub provider_name: Option<String>,
    pub name: String,
    #[serde(default)]
    pub label_names: Vec<String>,
    #[serde(default)]
    pub instructions: String,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    #[serde(default)]
    pub apply_automatically: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoLabelResult {
    pub message_id: Uuid,
    pub labels: Vec<Label>,
}

impl AiProviderInput {
    pub fn normalize(&mut self, require_key: bool) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "provider name", 120)?;
        self.model = clean_required(&self.model, "AI model", 160)?;
        validate_api_pair(self.provider_kind, self.api_type)?;
        self.base_url = self
            .base_url
            .take()
            .map(|value| clean_url(&value))
            .transpose()?;
        self.api_key = self
            .api_key
            .take()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        if self.api_key.as_ref().is_some_and(|value| {
            value.len() > 4096 || value.chars().any(|character| character == '\0')
        }) {
            return Err(AppError::Validation("AI API key is invalid".into()));
        }
        if require_key && self.api_key.is_none() {
            return Err(AppError::Validation("AI API key is required".into()));
        }
        validate_proxy(&mut self.proxy)?;
        Ok(())
    }
}

impl LabelInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "label name", 80)?;
        self.color = self.color.trim().to_owned();
        if self.color.is_empty()
            || self.color.len() > 40
            || self.color.chars().any(|character| character.is_control())
        {
            return Err(AppError::Validation("label color is invalid".into()));
        }
        Ok(())
    }
}

impl AutoLabelRuleInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "auto-label rule name", 120)?;
        self.label_ids.sort_unstable();
        self.label_ids.dedup();
        if self.label_ids.is_empty() || self.label_ids.len() > 12 {
            return Err(AppError::Validation("auto-label labels are invalid".into()));
        }
        self.instructions = self.instructions.trim().to_owned();
        if self.instructions.len() > 4096 || self.instructions.chars().any(|value| value == '\0') {
            return Err(AppError::Validation(
                "auto-label instructions are invalid".into(),
            ));
        }
        Ok(())
    }
}

impl AutoLabelSubscriptionInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "subscription name", 120)?;
        if self.url.len() > 2048 || self.url.chars().any(char::is_control) {
            return Err(AppError::Validation("subscription URL is invalid".into()));
        }
        let parsed = Url::parse(self.url.trim())
            .map_err(|_| AppError::Validation("subscription URL is invalid".into()))?;
        if parsed.scheme() != "https"
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || parsed.fragment().is_some()
        {
            return Err(AppError::Validation(
                "subscription URL must be a public HTTPS URL".into(),
            ));
        }
        self.url = parsed.to_string();
        Ok(())
    }
}

impl AutoLabelRuleFeed {
    pub fn normalize(&mut self, subscription_name: Option<&str>) -> Result<(), AppError> {
        if self.format != AUTO_LABEL_FEED_FORMAT || self.version != AUTO_LABEL_FEED_VERSION {
            return Err(AppError::Validation(
                "auto-label subscription format is unsupported".into(),
            ));
        }
        if self.labels.len() > 500 || self.rules.len() > 2_000 {
            return Err(AppError::Validation(
                "auto-label subscription contains too many entries".into(),
            ));
        }

        let mut label_names = std::collections::HashSet::new();
        for label in &mut self.labels {
            let mut input = LabelInput {
                name: label.name.clone(),
                color: label.color.clone(),
                is_auto: label.is_auto,
            };
            input.normalize()?;
            label.name = input.name;
            label.color = input.color;
            if !label_names.insert(label.name.to_ascii_lowercase()) {
                return Err(AppError::Validation(
                    "auto-label subscription contains duplicate labels".into(),
                ));
            }
        }

        let mut rule_names = std::collections::HashSet::new();
        for rule in &mut self.rules {
            rule.name = clean_required(&rule.name, "auto-label rule name", 120)?;
            if let Some(source) = subscription_name
                && format!("{source}: {}", rule.name).chars().count() > 120
            {
                return Err(AppError::Validation(
                    "subscribed auto-label rule name is too long".into(),
                ));
            }
            if !rule_names.insert(rule.name.to_ascii_lowercase()) {
                return Err(AppError::Validation(
                    "auto-label subscription contains duplicate rules".into(),
                ));
            }
            rule.instructions = rule.instructions.trim().to_owned();
            if rule.instructions.len() > 4096
                || rule.instructions.chars().any(|value| value == '\0')
            {
                return Err(AppError::Validation(
                    "auto-label subscription instructions are invalid".into(),
                ));
            }
            if rule.label_names.is_empty() || rule.label_names.len() > 12 {
                return Err(AppError::Validation(
                    "auto-label subscription labels are invalid".into(),
                ));
            }
            let mut references = std::collections::HashSet::new();
            for name in &mut rule.label_names {
                *name = clean_required(name, "auto-label label name", 80)?;
                let key = name.to_ascii_lowercase();
                if !label_names.contains(&key) || !references.insert(key) {
                    return Err(AppError::Validation(
                        "auto-label subscription references invalid labels".into(),
                    ));
                }
            }
            rule.account_email = rule
                .account_email
                .take()
                .map(|value| validate_feed_email(&value))
                .transpose()?;
            rule.provider_name = rule
                .provider_name
                .take()
                .map(|value| clean_required(&value, "AI provider name", 120))
                .transpose()?;
        }
        Ok(())
    }
}

pub fn validate_ai_text(text: &str) -> Result<String, AppError> {
    let value = text.trim();
    if value.is_empty() || value.len() > 64 * 1024 || value.chars().any(|value| value == '\0') {
        return Err(AppError::Validation("AI text input is invalid".into()));
    }
    Ok(value.to_owned())
}

pub fn clean_optional_phrase(
    value: Option<&str>,
    field: &str,
    max: usize,
) -> Result<Option<String>, AppError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| clean_required(value, field, max))
        .transpose()
}

fn validate_api_pair(kind: AiProviderKind, api_type: AiApiType) -> Result<(), AppError> {
    let ok = matches!(
        (kind, api_type),
        (AiProviderKind::OpenAi, AiApiType::Chat)
            | (AiProviderKind::OpenAi, AiApiType::Responses)
            | (AiProviderKind::Claude, AiApiType::Messages)
            | (AiProviderKind::Gemini, AiApiType::GenerateContent)
    );
    if ok {
        Ok(())
    } else {
        Err(AppError::Validation(
            "AI API type does not match provider".into(),
        ))
    }
}

fn validate_proxy(proxy: &mut ProxyInput) -> Result<(), AppError> {
    match proxy.kind {
        ProxyKind::Direct => {
            proxy.host = None;
            proxy.port = None;
            proxy.username = None;
            proxy.password = None;
        }
        ProxyKind::Http | ProxyKind::Socks5 => {
            let host = proxy
                .host
                .as_deref()
                .ok_or_else(|| AppError::Validation("proxy host is required".into()))?;
            proxy.host = Some(clean_host(host, "proxy")?);
            if proxy.port.unwrap_or_default() == 0 {
                return Err(AppError::Validation("proxy port is invalid".into()));
            }
            proxy.username = proxy
                .username
                .take()
                .map(|value| clean_optional(&value, "proxy username", 255))
                .transpose()?
                .flatten();
            if let Some(password) = &proxy.password
                && (password.len() > 255 || password.chars().any(char::is_control))
            {
                return Err(AppError::Validation("proxy password is invalid".into()));
            }
        }
    }
    Ok(())
}

fn clean_url(raw: &str) -> Result<String, AppError> {
    let value = raw.trim().trim_end_matches('/');
    let url =
        url::Url::parse(value).map_err(|_| AppError::Validation("base URL is invalid".into()))?;
    if !matches!(url.scheme(), "https" | "http") || url.username() != "" || url.password().is_some()
    {
        return Err(AppError::Validation("base URL is invalid".into()));
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

fn clean_optional(value: &str, field: &str, max: usize) -> Result<Option<String>, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        clean_required(value, field, max).map(Some)
    }
}

fn clean_host(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim().trim_end_matches('.');
    if value.is_empty()
        || value.len() > 253
        || value.contains("://")
        || value
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
    {
        return Err(AppError::Validation(format!("{field} host is invalid")));
    }
    Ok(value.to_owned())
}

fn validate_feed_email(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 254
        || !value.contains('@')
        || value.chars().any(char::is_control)
    {
        return Err(AppError::Validation(
            "auto-label account reference is invalid".into(),
        ));
    }
    Ok(value.to_ascii_lowercase())
}
