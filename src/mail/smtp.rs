use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
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
const RESPONSE_LIMIT: usize = 64 * 1024;

pub async fn test(
    account: &MailAccount,
    secrets: &AccountSecrets,
    proxy: &ProxyConfig,
) -> Result<()> {
    timeout(OPERATION_TIMEOUT, async {
        let mut stream = authenticated_stream(account, secrets, proxy).await?;
        let _ = command(&mut stream, "QUIT", &[221]).await;
        Ok(())
    })
    .await
    .map_err(|_| anyhow!("SMTP operation timed out"))?
}

pub async fn send(
    account: &MailAccount,
    secrets: &AccountSecrets,
    proxy: &ProxyConfig,
    envelope_from: &str,
    recipients: &[String],
    message: &[u8],
) -> Result<()> {
    timeout(OPERATION_TIMEOUT, async {
        let mut stream = authenticated_stream(account, secrets, proxy).await?;
        command(&mut stream, &format!("MAIL FROM:<{envelope_from}>"), &[250]).await?;
        for recipient in recipients {
            command(&mut stream, &format!("RCPT TO:<{recipient}>"), &[250, 251]).await?;
        }
        command(&mut stream, "DATA", &[354]).await?;
        let stuffed = dot_stuff(message);
        stream.write_all(&stuffed).await?;
        if !stuffed.ends_with(b"\r\n") {
            stream.write_all(b"\r\n").await?;
        }
        stream.write_all(b".\r\n").await?;
        let (code, response) = read_response(&mut stream).await?;
        if code != 250 {
            bail!("SMTP DATA failed with code {code}: {}", response.trim());
        }
        let _ = command(&mut stream, "QUIT", &[221]).await;
        Ok(())
    })
    .await
    .map_err(|_| anyhow!("SMTP send timed out"))?
}

async fn authenticated_stream(
    account: &MailAccount,
    secrets: &AccountSecrets,
    proxy: &ProxyConfig,
) -> Result<BoxStream> {
    let stream = connect(&account.smtp.host, account.smtp.port, proxy).await?;
    let mut stream = match account.smtp.security {
        ConnectionSecurity::Tls => {
            let mut stream = tls::wrap(stream, &account.smtp.host).await?;
            expect_greeting(&mut stream).await?;
            stream
        }
        ConnectionSecurity::Starttls => {
            let mut stream = stream;
            expect_greeting(&mut stream).await?;
            command(&mut stream, "EHLO meowmail.local", &[250]).await?;
            command(&mut stream, "STARTTLS", &[220]).await?;
            tls::wrap(stream, &account.smtp.host).await?
        }
    };
    command(&mut stream, "EHLO meowmail.local", &[250]).await?;
    authenticate(
        &mut stream,
        &account.username,
        secrets.password.expose_secret(),
    )
    .await?;
    Ok(stream)
}

async fn authenticate(stream: &mut BoxStream, username: &str, password: &str) -> Result<()> {
    let payload = STANDARD.encode(format!("\0{username}\0{password}"));
    let (code, response) = raw_command(stream, &format!("AUTH PLAIN {payload}")).await?;
    if code == 235 {
        return Ok(());
    }
    if !matches!(code, 500 | 501 | 502 | 504) {
        bail!(
            "SMTP authentication failed with code {code}: {}",
            response.trim()
        );
    }
    command(stream, "AUTH LOGIN", &[334]).await?;
    command(stream, &STANDARD.encode(username), &[334]).await?;
    command(stream, &STANDARD.encode(password), &[235]).await?;
    Ok(())
}

async fn expect_greeting(stream: &mut BoxStream) -> Result<()> {
    let (code, response) = read_response(stream).await?;
    if code != 220 {
        bail!("SMTP server returned code {code}: {}", response.trim());
    }
    Ok(())
}

async fn command(stream: &mut BoxStream, value: &str, expected: &[u16]) -> Result<String> {
    let (code, response) = raw_command(stream, value).await?;
    if !expected.contains(&code) {
        bail!("SMTP command failed with code {code}: {}", response.trim());
    }
    Ok(response)
}

async fn raw_command(stream: &mut BoxStream, value: &str) -> Result<(u16, String)> {
    if value.contains(['\r', '\n']) {
        bail!("SMTP command contains invalid characters");
    }
    stream.write_all(value.as_bytes()).await?;
    stream.write_all(b"\r\n").await?;
    read_response(stream).await
}

async fn read_response(stream: &mut BoxStream) -> Result<(u16, String)> {
    let mut response = Vec::with_capacity(256);
    let mut expected_code = None;
    loop {
        let line_start = response.len();
        read_line_into(stream, &mut response).await?;
        let line = &response[line_start..];
        if line.len() < 4 || !line[..3].iter().all(u8::is_ascii_digit) {
            bail!("SMTP server returned an invalid response");
        }
        let code = std::str::from_utf8(&line[..3])?.parse::<u16>()?;
        if expected_code
            .replace(code)
            .is_some_and(|expected| expected != code)
        {
            bail!("SMTP server returned inconsistent response codes");
        }
        if line[3] == b' ' {
            return Ok((code, String::from_utf8_lossy(&response).into_owned()));
        }
        if line[3] != b'-' {
            bail!("SMTP server returned an invalid multiline response");
        }
    }
}

async fn read_line_into(stream: &mut BoxStream, output: &mut Vec<u8>) -> Result<()> {
    let mut byte = [0_u8; 1];
    while output.len() < RESPONSE_LIMIT {
        if stream.read(&mut byte).await? == 0 {
            bail!("SMTP server closed the connection");
        }
        output.push(byte[0]);
        if output.ends_with(b"\r\n") {
            return Ok(());
        }
    }
    bail!("SMTP response exceeded the size limit")
}

fn dot_stuff(message: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(message.len() + 32);
    let mut line_start = true;
    for &byte in message {
        if line_start && byte == b'.' {
            output.push(b'.');
        }
        output.push(byte);
        line_start = byte == b'\n';
    }
    output
}
