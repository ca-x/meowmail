use meowmail::mail::parse_message;

#[test]
fn parses_multipart_mail_and_sanitizes_html() {
    let raw = b"From: Alice Example <alice@example.com>\r\nTo: me@example.net\r\nSubject: Hello =?UTF-8?B?5aaZ6YKu?=\r\nMessage-ID: <one@example.com>\r\nDate: Sun, 26 Jul 2026 12:00:00 +0000\r\nMIME-Version: 1.0\r\nContent-Type: multipart/alternative; boundary=meow\r\n\r\n--meow\r\nContent-Type: text/plain; charset=utf-8\r\n\r\nPlain hello\r\n--meow\r\nContent-Type: text/html; charset=utf-8\r\n\r\n<p>Hello <strong>Meowmail</strong></p><script>alert(1)</script>\r\n--meow--\r\n";
    let parsed = parse_message(raw, 0).unwrap();
    assert_eq!(parsed.sender_name.as_deref(), Some("Alice Example"));
    assert_eq!(parsed.sender_email, "alice@example.com");
    assert!(parsed.subject.contains("妙邮"));
    assert!(parsed.body_text.contains("Plain hello"));
    let html = parsed.body_html.unwrap();
    assert!(html.contains("<strong>Meowmail</strong>"));
    assert!(!html.contains("<script"));
}
