use meowmail::{
    accounts::{ProxyConfig, ProxyKind},
    mail::connect_via_proxy,
};
use secrecy::SecretString;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn read_headers(stream: &mut (impl AsyncRead + Unpin)) -> Vec<u8> {
    let mut headers = Vec::new();
    let mut byte = [0_u8; 1];
    while !headers.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).await.unwrap();
        headers.push(byte[0]);
        assert!(headers.len() < 16 * 1024);
    }
    headers
}

#[tokio::test]
async fn http_connect_uses_basic_auth_and_returns_the_tunnel() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = String::from_utf8(read_headers(&mut socket).await).unwrap();
        assert!(request.starts_with("CONNECT imap.example.test:993 HTTP/1.1\r\n"));
        assert!(request.contains("Host: imap.example.test:993\r\n"));
        assert!(request.contains("Proxy-Authorization: Basic YWxpY2U6c2VjcmV0\r\n"));
        socket
            .write_all(b"HTTP/1.1 200 Connection Established\r\nProxy-Agent: mock\r\n\r\n")
            .await
            .unwrap();
        let mut ping = [0_u8; 4];
        socket.read_exact(&mut ping).await.unwrap();
        assert_eq!(&ping, b"ping");
        socket.write_all(b"pong").await.unwrap();
    });

    let proxy = ProxyConfig {
        kind: ProxyKind::Http,
        host: Some(address.ip().to_string()),
        port: Some(address.port()),
        username: Some("alice".into()),
        password: Some(SecretString::from("secret")),
    };
    let mut tunnel = connect_via_proxy("imap.example.test", 993, &proxy)
        .await
        .unwrap();
    tunnel.write_all(b"ping").await.unwrap();
    let mut pong = [0_u8; 4];
    tunnel.read_exact(&mut pong).await.unwrap();
    assert_eq!(&pong, b"pong");
    server.await.unwrap();
}

#[tokio::test]
async fn socks5_authenticates_and_encodes_a_domain_target() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut greeting = [0_u8; 4];
        socket.read_exact(&mut greeting).await.unwrap();
        assert_eq!(greeting, [0x05, 0x02, 0x00, 0x02]);
        socket.write_all(&[0x05, 0x02]).await.unwrap();

        let mut auth_header = [0_u8; 2];
        socket.read_exact(&mut auth_header).await.unwrap();
        assert_eq!(auth_header, [0x01, 0x05]);
        let mut username = [0_u8; 5];
        socket.read_exact(&mut username).await.unwrap();
        assert_eq!(&username, b"alice");
        let password_len = socket.read_u8().await.unwrap();
        assert_eq!(password_len, 6);
        let mut password = [0_u8; 6];
        socket.read_exact(&mut password).await.unwrap();
        assert_eq!(&password, b"secret");
        socket.write_all(&[0x01, 0x00]).await.unwrap();

        let mut connect_header = [0_u8; 5];
        socket.read_exact(&mut connect_header).await.unwrap();
        assert_eq!(&connect_header[..4], &[0x05, 0x01, 0x00, 0x03]);
        let host_len = usize::from(connect_header[4]);
        let mut host = vec![0_u8; host_len];
        socket.read_exact(&mut host).await.unwrap();
        assert_eq!(&host, b"smtp.example.test");
        let port = socket.read_u16().await.unwrap();
        assert_eq!(port, 587);
        socket
            .write_all(&[0x05, 0x00, 0x00, 0x01, 127, 0, 0, 1, 0, 0])
            .await
            .unwrap();
        let mut ping = [0_u8; 4];
        socket.read_exact(&mut ping).await.unwrap();
        assert_eq!(&ping, b"ping");
        socket.write_all(b"pong").await.unwrap();
    });

    let proxy = ProxyConfig {
        kind: ProxyKind::Socks5,
        host: Some(address.ip().to_string()),
        port: Some(address.port()),
        username: Some("alice".into()),
        password: Some(SecretString::from("secret")),
    };
    let mut tunnel = connect_via_proxy("smtp.example.test", 587, &proxy)
        .await
        .unwrap();
    tunnel.write_all(b"ping").await.unwrap();
    let mut pong = [0_u8; 4];
    tunnel.read_exact(&mut pong).await.unwrap();
    assert_eq!(&pong, b"pong");
    server.await.unwrap();
}
