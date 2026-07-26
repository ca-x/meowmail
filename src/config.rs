use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result, bail};
use secrecy::SecretString;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    Local,
    Oidc,
    Hybrid,
}

impl AuthMode {
    pub fn local_enabled(self) -> bool {
        matches!(self, Self::Local | Self::Hybrid)
    }

    pub fn oidc_enabled(self) -> bool {
        matches!(self, Self::Oidc | Self::Hybrid)
    }
}

#[derive(Clone)]
pub struct BootstrapAdmin {
    pub username: String,
    pub password: SecretString,
}

#[derive(Clone)]
pub struct OidcConfig {
    pub issuer: Url,
    pub client_id: String,
    pub client_secret: Option<SecretString>,
    pub redirect_url: Url,
    pub scopes: Vec<String>,
    pub first_user_admin: bool,
}

pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub auth_mode: AuthMode,
    pub bootstrap_admin: Option<BootstrapAdmin>,
    pub oidc: Option<OidcConfig>,
    pub vault_secret: Option<SecretString>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let auth_mode = match env_value("MEOWMAIL_AUTH_MODE")
            .unwrap_or_else(|| "local".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "local" => AuthMode::Local,
            "oidc" => AuthMode::Oidc,
            "hybrid" => AuthMode::Hybrid,
            _ => bail!("MEOWMAIL_AUTH_MODE must be local, oidc, or hybrid"),
        };
        let bind = env_value("MEOWMAIL_BIND")
            .unwrap_or_else(|| "0.0.0.0:8080".into())
            .parse()
            .context("MEOWMAIL_BIND is invalid")?;
        let data_dir =
            PathBuf::from(env_value("MEOWMAIL_DATA_DIR").unwrap_or_else(|| "data".into()));

        let bootstrap_username = env_value("MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME");
        let bootstrap_password = env_value("MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD");
        let bootstrap_admin = match (bootstrap_username, bootstrap_password) {
            (Some(username), Some(password)) => Some(BootstrapAdmin {
                username: validate_username(username)?,
                password: SecretString::from(validate_password(password)?),
            }),
            (None, None) => None,
            _ => bail!(
                "MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME and MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD must be set together"
            ),
        };

        let oidc = if auth_mode.oidc_enabled() {
            Some(OidcConfig::from_env()?)
        } else {
            None
        };
        let vault_secret = env_value("MEOWMAIL_VAULT_KEY")
            .map(validate_vault_secret)
            .transpose()?
            .map(SecretString::from);

        Ok(Self {
            bind,
            data_dir,
            auth_mode,
            bootstrap_admin,
            oidc,
            vault_secret,
        })
    }

    pub fn new(password: String, data_dir: PathBuf) -> Result<Self> {
        let password = validate_password(password)?;
        Ok(Self {
            bind: "0.0.0.0:8080"
                .parse()
                .expect("static bind address is valid"),
            data_dir,
            auth_mode: AuthMode::Local,
            bootstrap_admin: Some(BootstrapAdmin {
                username: "admin".into(),
                password: SecretString::from(password.clone()),
            }),
            oidc: None,
            vault_secret: Some(SecretString::from(password)),
        })
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("meowmail.sqlite3")
    }

    pub fn vault_salt_path(&self) -> PathBuf {
        self.data_dir.join("vault.salt")
    }

    pub fn vault_key_path(&self) -> PathBuf {
        self.data_dir.join("vault.key")
    }
}

impl OidcConfig {
    fn from_env() -> Result<Self> {
        let issuer = required_any(&["MEOWMAIL_OIDC_ISSUER", "LAZYCAT_AUTH_OIDC_ISSUER_URI"])?;
        let client_id = required_any(&["MEOWMAIL_OIDC_CLIENT_ID", "LAZYCAT_AUTH_OIDC_CLIENT_ID"])?;
        let client_secret = env_any(&[
            "MEOWMAIL_OIDC_CLIENT_SECRET",
            "LAZYCAT_AUTH_OIDC_CLIENT_SECRET",
        ])
        .map(SecretString::from);
        let redirect = env_value("MEOWMAIL_OIDC_REDIRECT_URL")
            .or_else(|| {
                env_value("LAZYCAT_PUBLIC_URL")
                    .map(|base| format!("{}/api/v1/auth/oidc/callback", base.trim_end_matches('/')))
            })
            .ok_or_else(|| anyhow::anyhow!("MEOWMAIL_OIDC_REDIRECT_URL is required"))?;
        let issuer = validate_http_url("MEOWMAIL_OIDC_ISSUER", issuer)?;
        let redirect_url = validate_http_url("MEOWMAIL_OIDC_REDIRECT_URL", redirect)?;
        let scopes = env_value("MEOWMAIL_OIDC_SCOPES")
            .unwrap_or_else(|| "openid profile email".into())
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if !scopes.iter().any(|scope| scope == "openid") {
            bail!("MEOWMAIL_OIDC_SCOPES must include openid");
        }
        let first_user_admin = parse_bool("MEOWMAIL_OIDC_FIRST_USER_ADMIN", true)?;
        Ok(Self {
            issuer,
            client_id,
            client_secret,
            redirect_url,
            scopes,
            first_user_admin,
        })
    }
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_any(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| env_value(name))
}

fn required_any(names: &[&str]) -> Result<String> {
    env_any(names).ok_or_else(|| anyhow::anyhow!("{} is required", names[0]))
}

fn parse_bool(name: &str, default: bool) -> Result<bool> {
    match env_value(name).as_deref() {
        None => Ok(default),
        Some("1" | "true" | "yes" | "on") => Ok(true),
        Some("0" | "false" | "no" | "off") => Ok(false),
        Some(_) => bail!("{name} must be true or false"),
    }
}

fn validate_username(value: String) -> Result<String> {
    let value = value.trim();
    if !(1..=128).contains(&value.chars().count())
        || value.chars().any(|character| character.is_control())
    {
        bail!("bootstrap administrator username is invalid");
    }
    Ok(value.to_owned())
}

fn validate_password(value: String) -> Result<String> {
    if !(8..=4096).contains(&value.chars().count()) || value.chars().any(char::is_control) {
        bail!("bootstrap administrator password must contain 8-4096 non-control characters");
    }
    Ok(value)
}

fn validate_vault_secret(value: String) -> Result<String> {
    if !(4..=4096).contains(&value.chars().count()) || value.chars().any(char::is_control) {
        bail!("MEOWMAIL_VAULT_KEY is invalid");
    }
    Ok(value)
}

fn validate_http_url(name: &str, value: String) -> Result<Url> {
    let url = Url::parse(&value).with_context(|| format!("{name} is invalid"))?;
    let http_loopback = url.scheme() == "http"
        && url
            .host_str()
            .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
    if url.scheme() != "https" && !http_loopback {
        bail!("{name} must use HTTPS (HTTP is allowed only for loopback development)");
    }
    if !url.username().is_empty() || url.password().is_some() || url.host_str().is_none() {
        bail!("{name} must be an absolute URL without credentials");
    }
    Ok(url)
}

impl std::fmt::Debug for Config {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Config")
            .field("bind", &self.bind)
            .field("data_dir", &self.data_dir)
            .field("auth_mode", &self.auth_mode)
            .field("bootstrap_admin", &self.bootstrap_admin.is_some())
            .field("oidc", &self.oidc.is_some())
            .field(
                "vault_secret",
                &self.vault_secret.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}
