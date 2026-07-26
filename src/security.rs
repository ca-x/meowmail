use std::{fs::OpenOptions, io::Write, path::Path};

use anyhow::{Context, Result, anyhow};
use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
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
const KEY_LEN: usize = 32;

#[derive(Clone)]
pub struct CredentialVault {
    cipher: XChaCha20Poly1305,
}

impl CredentialVault {
    pub fn load(secret: Option<&SecretString>, salt_path: &Path, key_path: &Path) -> Result<Self> {
        let mut key = Zeroizing::new([0_u8; KEY_LEN]);
        if let Some(secret) = secret {
            let salt = load_or_create_bytes::<SALT_LEN>(salt_path)?;
            derive_key(secret.expose_secret().as_bytes(), &salt, key.as_mut())?;
        } else {
            key.copy_from_slice(&load_or_create_bytes::<KEY_LEN>(key_path)?);
        }
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
            .map_err(|_| anyhow!("credential decryption failed; the vault key may have changed"))?;
        let plaintext = String::from_utf8(plaintext).context("credential plaintext is invalid")?;
        Ok(SecretString::from(plaintext))
    }
}

pub fn hash_secret(secret: &str) -> Result<String> {
    let mut salt_bytes = [0_u8; SALT_LEN];
    rand::rng().fill_bytes(&mut salt_bytes);
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|error| anyhow!(error.to_string()))?;
    argon2()
        .hash_password(secret.as_bytes(), &salt)
        .map(|value| value.to_string())
        .map_err(|error| anyhow!("secret hashing failed: {error}"))
}

pub fn verify_secret(encoded: &str, supplied: &str) -> bool {
    let Ok(hash) = PasswordHash::new(encoded) else {
        return false;
    };
    argon2().verify_password(supplied.as_bytes(), &hash).is_ok()
}

pub fn encrypt_archive(passphrase: &str, plaintext: &[u8]) -> Result<String> {
    let mut salt = [0_u8; SALT_LEN];
    let mut nonce = [0_u8; NONCE_LEN];
    rand::rng().fill_bytes(&mut salt);
    rand::rng().fill_bytes(&mut nonce);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    derive_key(passphrase.as_bytes(), &salt, key.as_mut())?;
    let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
        .map_err(|_| anyhow!("archive key initialization failed"))?;
    let nonce_value = XNonce::try_from(nonce.as_slice())
        .map_err(|_| anyhow!("archive nonce initialization failed"))?;
    let encrypted = cipher
        .encrypt(&nonce_value, plaintext)
        .map_err(|_| anyhow!("archive encryption failed"))?;
    let mut envelope = Vec::with_capacity(SALT_LEN + NONCE_LEN + encrypted.len());
    envelope.extend_from_slice(&salt);
    envelope.extend_from_slice(&nonce);
    envelope.extend_from_slice(&encrypted);
    Ok(URL_SAFE_NO_PAD.encode(envelope))
}

pub fn decrypt_archive(passphrase: &str, envelope: &str) -> Result<Vec<u8>> {
    let decoded = URL_SAFE_NO_PAD
        .decode(envelope)
        .context("migration archive encoding is invalid")?;
    if decoded.len() <= SALT_LEN + NONCE_LEN {
        return Err(anyhow!("migration archive is truncated"));
    }
    let (salt, rest) = decoded.split_at(SALT_LEN);
    let (nonce, encrypted) = rest.split_at(NONCE_LEN);
    let mut key = Zeroizing::new([0_u8; KEY_LEN]);
    derive_key(passphrase.as_bytes(), salt, key.as_mut())?;
    let nonce_value = XNonce::try_from(nonce).map_err(|_| anyhow!("archive nonce is invalid"))?;
    XChaCha20Poly1305::new_from_slice(key.as_ref())
        .map_err(|_| anyhow!("archive key initialization failed"))?
        .decrypt(&nonce_value, encrypted)
        .map_err(|_| anyhow!("migration archive passphrase or contents are invalid"))
}

fn argon2() -> Argon2<'static> {
    let params =
        Params::new(64 * 1024, 3, 1, Some(KEY_LEN)).expect("static Argon2 parameters are valid");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

fn derive_key(secret: &[u8], salt: &[u8], output: &mut [u8]) -> Result<()> {
    argon2()
        .hash_password_into(secret, salt, output)
        .map_err(|error| anyhow!("key derivation failed: {error}"))
}

fn load_or_create_bytes<const N: usize>(path: &Path) -> Result<[u8; N]> {
    if path.exists() {
        let bytes =
            std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
        return bytes
            .try_into()
            .map_err(|_| anyhow!("{} has an invalid length", path.display()));
    }

    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("secret path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let mut bytes = [0_u8; N];
    rand::rng().fill_bytes(&mut bytes);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&bytes)?;
            file.sync_all()?;
            Ok(bytes)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let bytes = std::fs::read(path)?;
            bytes
                .try_into()
                .map_err(|_| anyhow!("{} has an invalid length", path.display()))
        }
        Err(error) => Err(error.into()),
    }
}
