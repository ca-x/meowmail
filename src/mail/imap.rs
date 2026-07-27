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
