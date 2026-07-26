use std::{fs::OpenOptions, io::Write, path::Path};

use anyhow::{Context, Result, anyhow};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chacha20poly1305::{
    XChaCha20Poly1305, XNonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;
use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;

#[derive(Clone)]
pub struct CredentialVault {
    cipher: XChaCha20Poly1305,
}

impl CredentialVault {
    pub fn load(pin: &SecretString, salt_path: &Path) -> Result<Self> {
        let salt = load_or_create_salt(salt_path)?;
        let mut key = Zeroizing::new([0_u8; 32]);
        let params = Params::new(64 * 1024, 3, 1, Some(32)).context("invalid Argon2 parameters")?;
        Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
            .hash_password_into(pin.expose_secret().as_bytes(), &salt, key.as_mut())
            .map_err(|error| anyhow!("credential key derivation failed: {error}"))?;
        Ok(Self {
            cipher: XChaCha20Poly1305::new_from_slice(key.as_ref())
                .map_err(|_| anyhow!("credential key initialization failed"))?,
        })
    }

    pub fn seal(&self, secret: &str) -> Result<String> {
        let mut nonce = [0_u8; NONCE_LEN];
        rand::rng().fill_bytes(&mut nonce);
        let nonce_value = XNonce::try_from(nonce.as_slice())
            .map_err(|_| anyhow!("credential nonce initialization failed"))?;
        let encrypted = self
            .cipher
            .encrypt(&nonce_value, secret.as_bytes())
            .map_err(|_| anyhow!("credential encryption failed"))?;
        let mut envelope = Vec::with_capacity(NONCE_LEN + encrypted.len());
        envelope.extend_from_slice(&nonce);
        envelope.extend_from_slice(&encrypted);
        Ok(URL_SAFE_NO_PAD.encode(envelope))
    }

    pub fn open(&self, envelope: &str) -> Result<SecretString> {
        let decoded = URL_SAFE_NO_PAD
            .decode(envelope)
            .context("credential envelope is invalid")?;
        let (nonce, encrypted) = decoded
            .split_at_checked(NONCE_LEN)
            .ok_or_else(|| anyhow!("credential envelope is truncated"))?;
        let nonce_value =
            XNonce::try_from(nonce).map_err(|_| anyhow!("credential nonce is invalid"))?;
        let plaintext = self
            .cipher
            .decrypt(&nonce_value, encrypted)
            .map_err(|_| anyhow!("credential decryption failed; MEOWMAIL_PIN may have changed"))?;
        let plaintext = String::from_utf8(plaintext).context("credential plaintext is invalid")?;
        Ok(SecretString::from(plaintext))
    }
}

fn load_or_create_salt(path: &Path) -> Result<[u8; SALT_LEN]> {
    if path.exists() {
        let bytes =
            std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        return bytes
            .try_into()
            .map_err(|_| anyhow!("vault salt has an invalid length"));
    }

    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("vault salt path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut salt = [0_u8; SALT_LEN];
    rand::rng().fill_bytes(&mut salt);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&salt)?;
            file.sync_all()?;
            Ok(salt)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let bytes = std::fs::read(path)?;
            bytes
                .try_into()
                .map_err(|_| anyhow!("vault salt has an invalid length"))
        }
        Err(error) => Err(error.into()),
    }
}
