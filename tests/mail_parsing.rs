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

#[test]
fn parses_attachment_metadata_and_decoded_content() {
    let raw = b"From: Alice <alice@example.com>\r\n\
To: me@example.net\r\n\
Subject: Report\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=meow\r\n\
\r\n\
--meow\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
See attachment.\r\n\
--meow\r\n\
Content-Type: application/pdf; name=\"handbook.pdf\"\r\n\
Content-Disposition: attachment; filename=\"handbook.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
JVBERi0xLjQK\r\n\
--meow--\r\n";

    let parsed = parse_message(raw, 0).unwrap();

    assert_eq!(parsed.attachment_count, 1);
    assert_eq!(parsed.attachments.len(), 1);
    assert_eq!(parsed.attachments[0].filename, "handbook.pdf");
    assert_eq!(parsed.attachments[0].content_type, "application/pdf");
    assert_eq!(parsed.attachments[0].size, 9);
    assert_eq!(
        parsed.attachments[0].content.as_deref(),
        Some(b"%PDF-1.4\n".as_slice())
    );
}

#[test]
fn decodes_gb2312_body_used_by_chinese_mail_providers() {
    let mut raw = b"From: sender@163.com\r\n\
To: me@example.net\r\n\
Subject: Chinese body\r\n\
MIME-Version: 1.0\r\n\
Content-Type: text/plain; charset=gb2312\r\n\
Content-Transfer-Encoding: 8bit\r\n\
\r\n"
        .to_vec();
    raw.extend_from_slice(&[0xD6, 0xD0, 0xCE, 0xC4, 0xB2, 0xE2, 0xCA, 0xD4]);
    raw.extend_from_slice(b"\r\n");

    let parsed = parse_message(&raw, 0).unwrap();

    assert_eq!(parsed.body_text.trim(), "中文测试");
}
