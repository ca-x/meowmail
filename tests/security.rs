use std::path::PathBuf;

use meowmail::{config::Config, security::CredentialVault};
use secrecy::SecretString;

#[test]
fn pin_configuration_rejects_unsafe_values() {
    assert!(Config::new("123".into(), PathBuf::from("data")).is_err());
    assert!(Config::new("12\n34".into(), PathBuf::from("data")).is_err());
    assert!(Config::new("correct horse battery staple".into(), PathBuf::from("data")).is_ok());
}

#[test]
fn credential_vault_round_trips_and_rejects_a_different_pin() {
    let directory = tempfile::tempdir().unwrap();
    let salt = directory.path().join("vault.salt");
    let first = CredentialVault::load(&SecretString::from("first secure pin"), &salt).unwrap();
    let second = CredentialVault::load(&SecretString::from("different secure pin"), &salt).unwrap();
    let encrypted = first.seal("mail-password-123").unwrap();

    assert!(!encrypted.contains("mail-password-123"));
    assert_eq!(
        secrecy::ExposeSecret::expose_secret(&first.open(&encrypted).unwrap()),
        "mail-password-123"
    );
    assert!(second.open(&encrypted).is_err());
}
