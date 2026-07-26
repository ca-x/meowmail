use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Result, bail};
use secrecy::{ExposeSecret, SecretString};

pub struct Config {
    pub bind: SocketAddr,
    pub data_dir: PathBuf,
    pub pin: SecretString,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let pin = std::env::var("MEOWMAIL_PIN")
            .map_err(|_| anyhow::anyhow!("MEOWMAIL_PIN is required"))?;
        Self::new(pin, PathBuf::from("data"))
    }

    pub fn new(pin: String, data_dir: PathBuf) -> Result<Self> {
        let visible_len = pin.chars().count();
        if !(4..=128).contains(&visible_len) {
            bail!("MEOWMAIL_PIN must contain between 4 and 128 characters");
        }
        if pin.chars().any(char::is_control) {
            bail!("MEOWMAIL_PIN must not contain control characters");
        }
        Ok(Self {
            bind: "0.0.0.0:8080"
                .parse()
                .expect("static bind address is valid"),
            data_dir,
            pin: SecretString::from(pin),
        })
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("meowmail.sqlite3")
    }

    pub fn vault_salt_path(&self) -> PathBuf {
        self.data_dir.join("vault.salt")
    }

    pub fn pin_bytes(&self) -> &[u8] {
        self.pin.expose_secret().as_bytes()
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Config")
            .field("bind", &self.bind)
            .field("data_dir", &self.data_dir)
            .field("pin", &"[REDACTED]")
            .finish()
    }
}
