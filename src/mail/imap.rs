use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use async_imap::Session;
use futures_util::TryStreamExt;
use secrecy::ExposeSecret;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::timeout,
};

use crate::accounts::{AccountSecrets, ConnectionSecurity, MailAccount, ProxyConfig};

use super::{
    proxy::{BoxStream, connect},
    tls,
};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(25);

pub async fn test(
    account: &MailAccount,
    secrets: &AccountSecrets,
    proxy: &ProxyConfig,
) -> Result<()> {
    timeout(OPERATION_TIMEOUT, async {
        let mut session = connect_session(account, secrets, proxy).await?;
        session.noop().await.context("IMAP NOOP failed")?;
        session.logout().await.context("IMAP logout failed")?;
        Ok(())
    })
    .await
    .map_err(|_| anyhow!("IMAP operation timed out"))?
}

pub async fn connect_session(
    account: &MailAccount,
    secrets: &AccountSecrets,
    proxy: &ProxyConfig,
) -> Result<Session<BoxStream>> {
    let stream = connect(&account.imap.host, account.imap.port, proxy).await?;
    let stream = match account.imap.security {
        ConnectionSecurity::Tls => tls::wrap(stream, &account.imap.host).await?,
        ConnectionSecurity::Starttls => starttls(stream, &account.imap.host).await?,
    };
    let client = async_imap::Client::new(stream);
    client
        .login(&account.username, secrets.password.expose_secret())
        .await
        .map_err(|(error, _)| anyhow!("IMAP authentication failed: {error}"))
}

pub async fn delete_uid_set(session: &mut Session<BoxStream>, uid_set: &str) -> Result<()> {
    let capabilities = session
        .capabilities()
        .await
        .context("IMAP CAPABILITY failed")?;
    if !capabilities.has_str("UIDPLUS") {
        bail!("IMAP server does not support safe UID deletion (UIDPLUS)");
    }
    session
        .uid_store(uid_set, "+FLAGS.SILENT (\\Deleted)")
        .await
        .context("IMAP UID STORE failed")?
        .try_collect::<Vec<_>>()
        .await
        .context("IMAP UID STORE response failed")?;
    session
        .uid_expunge(uid_set)
        .await
        .context("IMAP UID EXPUNGE failed")?
        .try_collect::<Vec<_>>()
        .await
        .context("IMAP UID EXPUNGE response failed")?;
    Ok(())
}

async fn starttls(mut stream: BoxStream, host: &str) -> Result<BoxStream> {
    let greeting = read_line(&mut stream, 16 * 1024).await?;
    if !greeting.starts_with(b"*") {
        bail!("IMAP server returned an invalid greeting");
    }
    stream.write_all(b"a0 STARTTLS\r\n").await?;
    loop {
        let line = read_line(&mut stream, 16 * 1024).await?;
        if line.starts_with(b"a0 ") {
            let text = String::from_utf8_lossy(&line);
            if !text.to_ascii_uppercase().starts_with("A0 OK") {
                bail!("IMAP server refused STARTTLS");
            }
            break;
        }
    }
    tls::wrap(stream, host).await
}

async fn read_line(stream: &mut BoxStream, limit: usize) -> Result<Vec<u8>> {
    let mut line = Vec::with_capacity(128);
    let mut byte = [0_u8; 1];
    while line.len() < limit {
        if stream.read(&mut byte).await? == 0 {
            bail!("IMAP server closed the connection");
        }
        line.push(byte[0]);
        if line.ends_with(b"\r\n") {
            return Ok(line);
        }
    }
    bail!("IMAP response exceeded the size limit")
}

/// Write only the requested flags, preserving unrelated server flags.
/// Missing UIDs are retained local copies and have no flags to update remotely.
pub async fn update_flags(
    session: &mut Session<BoxStream>,
    uid: &str,
    is_read: Option<bool>,
    is_starred: Option<bool>,
) -> Result<()> {
    let exists = session
        .uid_fetch(uid, "UID")
        .await?
        .try_collect::<Vec<_>>()
        .await?
        .iter()
        .any(|fetch| fetch.uid.is_some_and(|value| value.to_string() == uid));
    if !exists {
        return Ok(());
    }
    for (value, flag) in [(is_read, "\\Seen"), (is_starred, "\\Flagged")] {
        if let Some(value) = value {
            let operation = if value { "+" } else { "-" };
            session
                .uid_store(uid, format!("{operation}FLAGS.SILENT ({flag})"))
                .await
                .context("IMAP flag update failed")?
                .try_collect::<Vec<_>>()
                .await
                .context("IMAP flag update response failed")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod flag_tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader};

    #[tokio::test]
    async fn writes_read_and_star_flags_without_replacing_other_flags() {
        let (client, server) = tokio::io::duplex(4096);
        let peer = tokio::spawn(async move {
            let mut server = BufReader::new(server);
            for expected in [
                "LOGIN \"test\" \"password\"",
                "UID FETCH 42 UID",
                "UID STORE 42 +FLAGS.SILENT (\\Seen)",
                "UID STORE 42 -FLAGS.SILENT (\\Flagged)",
                "UID FETCH 42 UID",
                "UID STORE 42 -FLAGS.SILENT (\\Seen)",
                "UID STORE 42 +FLAGS.SILENT (\\Flagged)",
            ] {
                let mut command = String::new();
                server.read_line(&mut command).await.unwrap();
                let (tag, body) = command.trim_end().split_once(' ').unwrap();
                assert_eq!(body, expected);
                if expected.contains("FETCH") {
                    server.write_all(b"* 1 FETCH (UID 42)\r\n").await.unwrap();
                }
                server
                    .write_all(format!("{tag} OK done\r\n").as_bytes())
                    .await
                    .unwrap();
            }
        });
        let mut session = async_imap::Client::new(Box::new(client) as BoxStream)
            .login("test", "password")
            .await
            .unwrap();
        update_flags(&mut session, "42", Some(true), Some(false))
            .await
            .unwrap();
        update_flags(&mut session, "42", Some(false), Some(true))
            .await
            .unwrap();
        peer.await.unwrap();
    }
    #[tokio::test]
    async fn missing_uid_does_not_modify_another_message() {
        let (client, server) = tokio::io::duplex(4096);
        let peer = tokio::spawn(async move {
            let mut server = BufReader::new(server);
            for expected in ["LOGIN \"test\" \"password\"", "UID FETCH 42 UID", "NOOP"] {
                let mut command = String::new();
                server.read_line(&mut command).await.unwrap();
                let (tag, body) = command.trim_end().split_once(' ').unwrap();
                assert_eq!(body, expected);
                server
                    .write_all(format!("{tag} OK done\r\n").as_bytes())
                    .await
                    .unwrap();
            }
        });
        let mut session = async_imap::Client::new(Box::new(client) as BoxStream)
            .login("test", "password")
            .await
            .unwrap();
        update_flags(&mut session, "42", Some(true), None)
            .await
            .unwrap();
        session.noop().await.unwrap();
        peer.await.unwrap();
    }
}
