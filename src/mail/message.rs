use ammonia::Builder;
use mail_parser::{MessageParser, MimeHeaders};
use sha2::{Digest, Sha256};

const MAX_ATTACHMENT_METADATA: usize = 256;
const MAX_ATTACHMENT_BYTES: usize = 32 * 1024 * 1024;
const MAX_ATTACHMENT_TOTAL_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct MailAttachment {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub content: Option<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct ParsedMail {
    pub message_id: Option<String>,
    pub reply_to_email: Option<String>,
    pub references: Vec<String>,
    pub sender_name: Option<String>,
    pub sender_email: String,
    pub recipients: Vec<String>,
    pub cc_recipients: Vec<String>,
    pub subject: String,
    pub thread_key: String,
    pub preview: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub received_at: i64,
    pub attachment_count: usize,
    pub attachments: Vec<MailAttachment>,
    pub raw_size: usize,
    pub is_promotional: bool,
    pub auto_forward_allowed: bool,
    pub auto_response_allowed: bool,
}

pub fn parse_message(raw: &[u8], fallback_timestamp: i64) -> Option<ParsedMail> {
    let message = MessageParser::default().parse(raw)?;
    let sender = message.from().and_then(|address| address.first());
    let reply_to_email = message
        .reply_to()
        .and_then(|addresses| addresses.first())
        .and_then(|address| address.address())
        .filter(|address| address.len() <= 254 && !address.chars().any(char::is_control))
        .map(str::to_owned);
    let references = message
        .references()
        .as_text_list()
        .map(|values| {
            values
                .iter()
                .rev()
                .take(200)
                .rev()
                .filter(|value| value.len() <= 998)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let in_reply_to = message
        .in_reply_to()
        .as_text_list()
        .map(|values| {
            values
                .iter()
                .rev()
                .take(200)
                .rev()
                .filter(|value| value.len() <= 998)
                .map(|value| value.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let body_text = message
        .body_text(0)
        .map_or_else(String::new, |value| value.into_owned());
    let body_html = message.body_html(0).map(|value| {
        Builder::default()
            .url_relative(ammonia::UrlRelative::Deny)
            .clean(&value)
            .to_string()
    });
    let preview = message.body_preview(180).map_or_else(
        || body_text.chars().take(180).collect(),
        |value| value.into_owned(),
    );
    let recipients = message
        .to()
        .map(|addresses| {
            addresses
                .iter()
                .filter_map(|address| address.address().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let cc_recipients = message
        .cc()
        .map(|addresses| {
            addresses
                .iter()
                .filter_map(|address| address.address().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let subject = message.subject().unwrap_or("(No subject)").to_owned();
    let message_id = message.message_id().map(str::to_owned);
    let thread_key = build_thread_key(message_id.as_deref(), &references, &in_reply_to, raw);
    let precedence = message
        .header_raw("Precedence")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let auto_submitted = message
        .header_raw("Auto-Submitted")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let has_list_headers =
        message.header_raw("List-Unsubscribe").is_some() || message.header_raw("List-ID").is_some();
    let has_campaign_header = message.headers_raw().any(|(name, _)| {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "x-campaign" | "x-campaign-id" | "x-mailer-campaign-id" | "x-marketing-id"
        )
    });
    let is_promotional =
        has_list_headers || matches!(precedence.as_str(), "bulk" | "list") || has_campaign_header;
    let sender_email = sender
        .and_then(|address| address.address())
        .unwrap_or("unknown@invalid")
        .to_owned();
    let sender_local = sender_email
        .split('@')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let auto_forward_allowed = auto_submitted.is_empty() || auto_submitted == "no";
    let auto_response_allowed = auto_forward_allowed
        && !is_promotional
        && !matches!(precedence.as_str(), "bulk" | "list" | "junk")
        && !sender_local.contains("no-reply")
        && !sender_local.contains("noreply")
        && !sender_local.contains("mailer-daemon")
        && !sender_local.contains("postmaster");
    let attachment_count = message.attachment_count();
    let mut retained_bytes = 0_usize;
    let attachments = message
        .attachments()
        .take(MAX_ATTACHMENT_METADATA)
        .enumerate()
        .map(|(index, attachment)| {
            let content_type = attachment_content_type(attachment.content_type());
            let filename = attachment_filename(
                attachment.attachment_name(),
                &content_type,
                index.saturating_add(1),
            );
            let contents = attachment.contents();
            let size = contents.len();
            let can_retain = size <= MAX_ATTACHMENT_BYTES
                && retained_bytes.saturating_add(size) <= MAX_ATTACHMENT_TOTAL_BYTES;
            let content = can_retain.then(|| {
                retained_bytes = retained_bytes.saturating_add(size);
                contents.to_vec()
            });
            MailAttachment {
                filename,
                content_type,
                size,
                content,
            }
        })
        .collect();
    Some(ParsedMail {
        message_id,
        reply_to_email,
        references,
        sender_name: sender.and_then(|address| address.name()).map(str::to_owned),
        sender_email,
        recipients,
        cc_recipients,
        subject,
        thread_key,
        preview,
        body_text,
        body_html,
        received_at: message
            .date()
            .map_or(fallback_timestamp, |date| date.to_timestamp()),
        attachment_count,
        attachments,
        raw_size: raw.len(),
        is_promotional,
        auto_forward_allowed,
        auto_response_allowed,
    })
}

fn build_thread_key(
    message_id: Option<&str>,
    references: &[String],
    in_reply_to: &[String],
    raw: &[u8],
) -> String {
    if let Some(root) = references
        .first()
        .map(String::as_str)
        .or_else(|| in_reply_to.first().map(String::as_str))
        .or(message_id)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return format!("id:{}", root.trim_matches(['<', '>']).to_ascii_lowercase());
    }
    format!("raw:{:x}", Sha256::digest(raw))
}

pub fn normalize_thread_subject(subject: &str) -> String {
    let mut value = subject.trim();
    loop {
        let trimmed = value.trim_start();
        let lower = trimmed.to_ascii_lowercase();
        let prefix_len = if lower.starts_with("re:") || lower.starts_with("fw:") {
            Some(3)
        } else if lower.starts_with("fwd:") {
            Some(4)
        } else if trimmed.starts_with("回复：") {
            Some("回复：".len())
        } else if trimmed.starts_with("转发：") {
            Some("转发：".len())
        } else {
            None
        };
        let Some(prefix_len) = prefix_len else { break };
        value = &trimmed[prefix_len..];
    }
    let normalized = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if normalized.is_empty() {
        "(no-subject)".into()
    } else {
        normalized
    }
}

fn attachment_content_type(content_type: Option<&mail_parser::ContentType<'_>>) -> String {
    let Some(content_type) = content_type else {
        return "application/octet-stream".into();
    };
    let value = format!(
        "{}/{}",
        content_type.ctype(),
        content_type.subtype().unwrap_or("octet-stream")
    )
    .to_ascii_lowercase();
    if value.len() <= 127
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'-' | b'.'))
    {
        value
    } else {
        "application/octet-stream".into()
    }
}

fn attachment_filename(name: Option<&str>, content_type: &str, position: usize) -> String {
    let supplied = name
        .and_then(|value| value.rsplit(['/', '\\']).next())
        .map(str::trim)
        .filter(|value| !value.is_empty() && !matches!(*value, "." | ".."))
        .map(|value| {
            value
                .chars()
                .filter(|character| !character.is_control())
                .take(255)
                .collect::<String>()
        })
        .filter(|value| !value.is_empty());
    supplied.unwrap_or_else(|| {
        let extension = mime_guess::get_mime_extensions_str(content_type)
            .and_then(|values| values.first())
            .copied()
            .unwrap_or("bin");
        format!("attachment-{position}.{extension}")
    })
}

#[cfg(test)]
mod tests {
    use super::{attachment_filename, parse_message};

    #[test]
    fn parses_reply_to_and_reference_message_ids() {
        let parsed = parse_message(
            b"From: Alice <alice@example.com>\r\n\
Reply-To: Team <reply@example.com>\r\n\
To: Me <me@example.com>\r\n\
Subject: Re: Project\r\n\
Message-ID: <parent@example.com>\r\n\
References: <root@example.com> <middle@example.com>\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hello\r\n",
            1,
        )
        .unwrap();

        assert_eq!(parsed.message_id.as_deref(), Some("parent@example.com"));
        assert_eq!(parsed.reply_to_email.as_deref(), Some("reply@example.com"));
        assert_eq!(
            parsed.references,
            ["root@example.com", "middle@example.com"]
        );
        assert_eq!(parsed.thread_key, "id:root@example.com");
    }

    #[test]
    fn message_ids_thread_replies_without_merging_unrelated_subjects() {
        let root = parse_message(
            b"From: Alice <alice@example.com>\r\nTo: Me <me@example.com>\r\nSubject: Status\r\nMessage-ID: <root@example.com>\r\n\r\nRoot\r\n",
            1,
        )
        .unwrap();
        let reply = parse_message(
            b"From: Bob <bob@example.com>\r\nTo: Me <me@example.com>\r\nSubject: Re: Status\r\nMessage-ID: <reply@example.com>\r\nReferences: <root@example.com>\r\nIn-Reply-To: <root@example.com>\r\n\r\nReply\r\n",
            2,
        )
        .unwrap();
        let unrelated = parse_message(
            b"From: Carol <carol@example.com>\r\nTo: Me <me@example.com>\r\nSubject: Status\r\nMessage-ID: <other@example.com>\r\n\r\nOther\r\n",
            3,
        )
        .unwrap();

        assert_eq!(root.thread_key, reply.thread_key);
        assert_ne!(root.thread_key, unrelated.thread_key);
    }

    #[test]
    fn bulk_mail_can_be_forwarded_but_auto_submitted_mail_is_suppressed() {
        let bulk = parse_message(
            b"From: News <news@example.com>\r\nTo: Me <me@example.com>\r\nSubject: Weekly\r\nPrecedence: bulk\r\n\r\nNews\r\n",
            1,
        )
        .unwrap();
        assert!(bulk.auto_forward_allowed);
        assert!(!bulk.auto_response_allowed);

        let automatic = parse_message(
            b"From: Bot <bot@example.com>\r\nTo: Me <me@example.com>\r\nSubject: Automatic\r\nAuto-Submitted: auto-replied\r\n\r\nAutomatic\r\n",
            1,
        )
        .unwrap();
        assert!(!automatic.auto_forward_allowed);
        assert!(!automatic.auto_response_allowed);
    }

    #[test]
    fn attachment_filenames_drop_path_and_control_characters() {
        assert_eq!(
            attachment_filename(Some("../folder/secret\nreport.pdf"), "application/pdf", 1),
            "secretreport.pdf"
        );
    }
}
