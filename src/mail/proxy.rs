use std::{fmt::Debug, io, net::IpAddr, time::Duration};

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use secrecy::ExposeSecret;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};

use crate::accounts::{ProxyConfig, ProxyKind};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
const HANDSHAKE_LIMIT: usize = 16 * 1024;

pub trait NetworkStream: AsyncRead + AsyncWrite + Unpin + Send + Debug {}
impl<T> NetworkStream for T where T: AsyncRead + AsyncWrite + Unpin + Send + Debug {}
pub type BoxStream = Box<dyn NetworkStream>;

pub async fn connect(
    target_host: &str,
    target_port: u16,
    proxy: &ProxyConfig,
) -> Result<BoxStream> {
    let future = async {
        match proxy.kind {
            ProxyKind::Direct => {
                let stream = TcpStream::connect((target_host, target_port))
                    .await
                    .with_context(|| format!("failed to connect to {target_host}:{target_port}"))?;
                configure(&stream)?;
                Ok::<BoxStream, anyhow::Error>(Box::new(stream))
            }
            ProxyKind::Http => http_connect(target_host, target_port, proxy).await,
            ProxyKind::Socks5 => socks5_connect(target_host, target_port, proxy).await,
        }
    };
    timeout(CONNECT_TIMEOUT, future)
        .await
        .map_err(|_| anyhow!("mail connection timed out"))?
}

async fn http_connect(
    target_host: &str,
    target_port: u16,
    proxy: &ProxyConfig,
) -> Result<BoxStream> {
    let (proxy_host, proxy_port) = proxy_endpoint(proxy)?;
    let mut stream = TcpStream::connect((proxy_host, proxy_port))
        .await
        .context("failed to connect to HTTP proxy")?;
    configure(&stream)?;
    let authority = format!("{target_host}:{target_port}");
    let mut request = format!(
        "CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\nProxy-Connection: keep-alive\r\n"
    );
    if let Some(username) = proxy.username.as_deref() {
        reject_header_value(username)?;
        let password = proxy
            .password
            .as_ref()
            .map_or("", |value| value.expose_secret());
        reject_header_value(password)?;
        let credentials = STANDARD.encode(format!("{username}:{password}"));
        request.push_str(&format!("Proxy-Authorization: Basic {credentials}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).await?;
    let response = read_until(&mut stream, b"\r\n\r\n", HANDSHAKE_LIMIT).await?;
    let status_line = std::str::from_utf8(&response)
        .ok()
        .and_then(|value| value.lines().next())
        .ok_or_else(|| anyhow!("HTTP proxy returned an invalid response"))?;
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("HTTP proxy returned an invalid status"))?;
    if status != 200 {
        bail!("HTTP proxy refused the tunnel with status {status}");
    }
    Ok(Box::new(stream))
}

async fn socks5_connect(
    target_host: &str,
    target_port: u16,
    proxy: &ProxyConfig,
) -> Result<BoxStream> {
    let (proxy_host, proxy_port) = proxy_endpoint(proxy)?;
    let mut stream = TcpStream::connect((proxy_host, proxy_port))
        .await
        .context("failed to connect to SOCKS5 proxy")?;
    configure(&stream)?;
    let with_auth = proxy.username.is_some();
    let methods: &[u8] = if with_auth {
        &[0x05, 0x02, 0x00, 0x02]
    } else {
        &[0x05, 0x01, 0x00]
    };
    stream.write_all(methods).await?;
    let mut greeting = [0_u8; 2];
    stream.read_exact(&mut greeting).await?;
    if greeting[0] != 0x05 || greeting[1] == 0xff {
        bail!("SOCKS5 proxy did not accept an authentication method");
    }
    match greeting[1] {
        0x00 => {}
        0x02 => authenticate_socks5(&mut stream, proxy).await?,
        method => bail!("SOCKS5 proxy selected unsupported authentication method {method}"),
    }

    let mut request = vec![0x05, 0x01, 0x00];
    encode_socks_address(target_host, &mut request)?;
    request.extend_from_slice(&target_port.to_be_bytes());
    stream.write_all(&request).await?;
    let mut reply = [0_u8; 4];
    stream.read_exact(&mut reply).await?;
    if reply[0] != 0x05 || reply[1] != 0x00 {
        bail!("SOCKS5 proxy refused the tunnel with code {}", reply[1]);
    }
    consume_socks_address(&mut stream, reply[3]).await?;
    let mut bound_port = [0_u8; 2];
    stream.read_exact(&mut bound_port).await?;
    Ok(Box::new(stream))
}

async fn authenticate_socks5(stream: &mut TcpStream, proxy: &ProxyConfig) -> Result<()> {
    let username = proxy
        .username
        .as_deref()
        .ok_or_else(|| anyhow!("SOCKS5 username is missing"))?;
    let password = proxy
        .password
        .as_ref()
        .map_or("", |value| value.expose_secret());
    if username.len() > 255 || password.len() > 255 {
        bail!("SOCKS5 credentials are too long");
    }
    let mut request = Vec::with_capacity(3 + username.len() + password.len());
    request.extend_from_slice(&[0x01, username.len() as u8]);
    request.extend_from_slice(username.as_bytes());
    request.push(password.len() as u8);
    request.extend_from_slice(password.as_bytes());
    stream.write_all(&request).await?;
    let mut response = [0_u8; 2];
    stream.read_exact(&mut response).await?;
    if response != [0x01, 0x00] {
        bail!("SOCKS5 authentication failed");
    }
    Ok(())
}

fn encode_socks_address(host: &str, output: &mut Vec<u8>) -> Result<()> {
    if let Ok(address) = host.parse::<IpAddr>() {
        match address {
            IpAddr::V4(address) => {
                output.push(0x01);
                output.extend_from_slice(&address.octets());
            }
            IpAddr::V6(address) => {
                output.push(0x04);
                output.extend_from_slice(&address.octets());
            }
        }
    } else {
        if host.len() > 255 {
            bail!("target host is too long for SOCKS5");
        }
        output.extend_from_slice(&[0x03, host.len() as u8]);
        output.extend_from_slice(host.as_bytes());
    }
    Ok(())
}

async fn consume_socks_address(stream: &mut TcpStream, kind: u8) -> Result<()> {
    let length = match kind {
        0x01 => 4,
        0x04 => 16,
        0x03 => {
            let mut length = [0_u8; 1];
            stream.read_exact(&mut length).await?;
            usize::from(length[0])
        }
        _ => bail!("SOCKS5 proxy returned an invalid address type"),
    };
    let mut address = vec![0_u8; length];
    stream.read_exact(&mut address).await?;
    Ok(())
}

fn proxy_endpoint(proxy: &ProxyConfig) -> Result<(&str, u16)> {
    Ok((
        proxy
            .host
            .as_deref()
            .ok_or_else(|| anyhow!("proxy host is missing"))?,
        proxy.port.ok_or_else(|| anyhow!("proxy port is missing"))?,
    ))
}

fn configure(stream: &TcpStream) -> Result<()> {
    stream
        .set_nodelay(true)
        .context("failed to configure mail connection")
}

fn reject_header_value(value: &str) -> Result<()> {
    if value.contains(['\r', '\n']) {
        bail!("proxy credentials contain invalid characters");
    }
    Ok(())
}

async fn read_until(stream: &mut TcpStream, delimiter: &[u8], limit: usize) -> Result<Vec<u8>> {
    let mut response = Vec::with_capacity(512);
    let mut byte = [0_u8; 1];
    while response.len() < limit {
        let read = stream.read(&mut byte).await?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "proxy closed the connection",
            )
            .into());
        }
        response.push(byte[0]);
        if response.ends_with(delimiter) {
            return Ok(response);
        }
    }
    bail!("proxy response exceeded the size limit")
}
