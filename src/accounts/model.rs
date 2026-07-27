use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionSecurity {
    Tls,
    Starttls,
}

impl ConnectionSecurity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tls => "tls",
            Self::Starttls => "starttls",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "tls" => Ok(Self::Tls),
            "starttls" => Ok(Self::Starttls),
            _ => Err(AppError::Validation(
                "unsupported connection security".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyKind {
    Direct,
    Http,
    Socks5,
}

impl ProxyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Http => "http",
            Self::Socks5 => "socks5",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "direct" => Ok(Self::Direct),
            "http" => Ok(Self::Http),
            "socks5" => Ok(Self::Socks5),
            _ => Err(AppError::Validation("unsupported proxy kind".into())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub security: ConnectionSecurity,
}

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub kind: ProxyKind,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub password: Option<SecretString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyInput {
    pub kind: ProxyKind,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

impl Default for ProxyInput {
    fn default() -> Self {
        Self {
            kind: ProxyKind::Direct,
            host: None,
            port: None,
            username: None,
            password: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInput {
    pub display_name: String,
    pub email: String,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    pub imap: ServerConfig,
    pub smtp: ServerConfig,
    #[serde(default)]
    pub proxy: ProxyInput,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccount {
    pub id: Uuid,
    pub display_name: String,
    pub email: String,
    pub username: String,
    pub imap: ServerConfig,
    pub smtp: ServerConfig,
    pub proxy: PublicProxyConfig,
    pub signature_id: Option<Uuid>,
    pub is_default: bool,
    pub last_synced_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub has_password: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountIdentityInput {
    pub display_name: String,
    pub signature_id: Option<Uuid>,
    pub is_default: bool,
}

impl AccountIdentityInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.display_name = clean_required(&self.display_name, "display name", 80)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicProxyConfig {
    pub kind: ProxyKind,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub username: Option<String>,
    pub has_password: bool,
}

#[derive(Clone)]
pub struct AccountSecrets {
    pub password: SecretString,
    pub proxy_password: Option<SecretString>,
}

impl AccountInput {
    pub fn validate(&mut self, require_password: bool) -> Result<(), AppError> {
        self.display_name = clean_required(&self.display_name, "display name", 80)?;
        self.email = clean_required(&self.email, "email", 254)?.to_ascii_lowercase();
        if !looks_like_email(&self.email) {
            return Err(AppError::Validation("email address is invalid".into()));
        }
        self.username = clean_required(&self.username, "username", 320)?;
        validate_server(&mut self.imap, "IMAP")?;
        validate_server(&mut self.smtp, "SMTP")?;
        if let Some(password) = &self.password {
            if password.is_empty()
                || password.len() > 4096
                || password.chars().any(char::is_control)
            {
                return Err(AppError::Validation("mail password is invalid".into()));
            }
        } else if require_password {
            return Err(AppError::Validation("mail password is required".into()));
        }
        validate_proxy(&mut self.proxy)?;
        Ok(())
    }
}

fn validate_server(server: &mut ServerConfig, label: &str) -> Result<(), AppError> {
    server.host = clean_host(&server.host, label)?;
    if server.port == 0 {
        return Err(AppError::Validation(format!("{label} port is invalid")));
    }
    Ok(())
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
                .as_deref()
                .map(|value| clean_optional(value, "proxy username", 255))
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
        return Ok(None);
    }
    clean_required(value, field, max).map(Some)
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
    Ok(value.to_ascii_lowercase())
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !value.chars().any(char::is_whitespace)
}
