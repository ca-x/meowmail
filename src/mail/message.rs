use ammonia::Builder;
use mail_parser::MessageParser;

#[derive(Debug, Clone)]
pub struct ParsedMail {
    pub message_id: Option<String>,
    pub sender_name: Option<String>,
    pub sender_email: String,
    pub recipients: Vec<String>,
    pub subject: String,
    pub preview: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub received_at: i64,
    pub attachment_count: usize,
}

pub fn parse_message(raw: &[u8], fallback_timestamp: i64) -> Option<ParsedMail> {
    let message = MessageParser::default().parse(raw)?;
    let sender = message.from().and_then(|address| address.first());
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
    Some(ParsedMail {
        message_id: message.message_id().map(str::to_owned),
        sender_name: sender.and_then(|address| address.name()).map(str::to_owned),
        sender_email: sender
            .and_then(|address| address.address())
            .unwrap_or("unknown@invalid")
            .to_owned(),
        recipients,
        subject: message.subject().unwrap_or("(No subject)").to_owned(),
        preview,
        body_text,
        body_html,
        received_at: message
            .date()
            .map_or(fallback_timestamp, |date| date.to_timestamp()),
        attachment_count: message.attachment_count(),
    })
}
