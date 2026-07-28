use base64::{Engine as _, engine::general_purpose::STANDARD};
use mail_builder::{MessageBuilder, headers::raw::Raw};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    AppState,
    accounts::AccountRepository,
    error::AppError,
    mail::{MailAttachment, send_smtp},
    preferences::{ComposeFontFamily, PreferencesRepository},
};

use super::repository::{MessageRepository, OutgoingStoredMessage};

const MAX_OUTGOING_ATTACHMENTS: usize = 10;
const MAX_OUTGOING_ATTACHMENT_BYTES: usize = 8 * 1024 * 1024;
const MAX_OUTGOING_ATTACHMENT_TOTAL_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeInput {
    pub account_id: Uuid,
    pub to: Vec<String>,
    #[serde(default)]
    pub cc: Vec<String>,
    #[serde(default)]
    pub bcc: Vec<String>,
    pub subject: String,
    pub text_body: String,
    #[serde(default)]
    pub html_body: Option<String>,
    #[serde(default)]
    pub attachments: Vec<ComposeAttachmentInput>,
    #[serde(default)]
    pub signature_id: Option<Uuid>,
    #[serde(default = "default_apply_signature")]
    pub apply_signature: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposeAttachmentInput {
    pub filename: String,
    pub content_type: String,
    pub content_base64: String,
    pub size: usize,
}

#[derive(Debug, Clone)]
struct DecodedAttachment {
    filename: String,
    content_type: String,
    content: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct ThreadingHeaders {
    pub in_reply_to: Option<String>,
    pub references: Vec<String>,
    pub automatic: Option<AutomaticMessageKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomaticMessageKind {
    Reply,
    Forward,
}

impl AutomaticMessageKind {
    fn header_value(self) -> &'static str {
        match self {
            Self::Reply => "auto-replied",
            Self::Forward => "auto-forwarded",
        }
    }
}

impl ComposeInput {
    pub fn validate(&mut self) -> Result<(), AppError> {
        self.validate_inner(true)
    }

    pub fn validate_draft(&mut self) -> Result<(), AppError> {
        self.validate_inner(false)
    }

    fn validate_inner(&mut self, require_recipient: bool) -> Result<(), AppError> {
        if (require_recipient && self.to.is_empty())
            || self.to.len() + self.cc.len() + self.bcc.len() > 100
        {
            return Err(AppError::Validation("recipient count is invalid".into()));
        }
        for address in self.to.iter_mut().chain(&mut self.cc).chain(&mut self.bcc) {
            *address = address.trim().to_ascii_lowercase();
            if address.len() > 254 || address.contains(['\r', '\n']) || !address.contains('@') {
                return Err(AppError::Validation("recipient address is invalid".into()));
            }
        }
        self.subject = self.subject.trim().to_owned();
        if self.subject.len() > 998 || self.subject.contains(['\r', '\n']) {
            return Err(AppError::Validation("subject is invalid".into()));
        }
        if self.text_body.len() > 2 * 1024 * 1024
            || self
                .html_body
                .as_ref()
                .is_some_and(|value| value.len() > 2 * 1024 * 1024)
        {
            return Err(AppError::Validation("message body is too large".into()));
        }
        validate_attachments(&mut self.attachments)?;
        Ok(())
    }
}

pub async fn send_outgoing(
    state: &AppState,
    user_id: Uuid,
    mut input: ComposeInput,
    threading: ThreadingHeaders,
) -> Result<(), AppError> {
    let preferences = PreferencesRepository::new(state.db.clone())
        .mail(user_id)
        .await?;
    if input.subject.trim().is_empty() && preferences.empty_subject_from_body {
        input.subject = first_subject_line(&input.text_body);
    }
    input.validate()?;
    let accounts = AccountRepository::new(state.db.clone(), state.vault.clone());
    let (account, secrets, proxy) = accounts.get_with_secrets(user_id, input.account_id).await?;
    let decoded_attachments = decode_attachments(&input.attachments)?;
    let sent_recipients = input.to.clone();
    let sent_cc = input.cc.clone();
    let sent_bcc = input.bcc.clone();
    let signature_id = input
        .signature_id
        .or(account.signature_id)
        .filter(|_| input.apply_signature)
        .map(|value| value.to_string());
    let signature = PreferencesRepository::new(state.db.clone())
        .signature_text(user_id, signature_id.as_deref())
        .await?;
    if let Some(signature) = signature.as_deref().filter(|value| !value.is_empty()) {
        input.text_body = append_signature(&input.text_body, signature);
        input.html_body = input
            .html_body
            .take()
            .map(|html| append_html_signature(&sanitize_html(&html), signature));
    }
    let mut builder = MessageBuilder::new()
        .from((account.display_name.clone(), account.email.clone()))
        .to(input.to.clone())
        .subject(input.subject.clone())
        .text_body(input.text_body.clone());
    if !input.cc.is_empty() {
        builder = builder.cc(input.cc.clone());
    }
    let html = input.html_body.clone().unwrap_or_else(|| {
        styled_html_body(
            &input.text_body,
            preferences.compose_font_family,
            preferences.compose_font_size,
            &preferences.compose_font_color,
        )
    });
    builder = builder.html_body(html.clone());
    if let Some(in_reply_to) = threading.in_reply_to {
        builder = builder.in_reply_to(in_reply_to);
    }
    if !threading.references.is_empty() {
        let references = threading.references;
        builder = builder.references(references);
    }
    let is_automatic = threading.automatic.is_some();
    if let Some(automatic) = threading.automatic {
        builder = add_automatic_headers(builder, automatic);
    }
    for attachment in &decoded_attachments {
        builder = builder.attachment(
            attachment.content_type.as_str(),
            attachment.filename.as_str(),
            attachment.content.clone(),
        );
    }
    let message = builder.write_to_vec().map_err(AppError::internal)?;
    let mut recipients = sent_recipients.clone();
    recipients.extend(sent_cc.clone());
    recipients.extend(sent_bcc.clone());
    send_smtp(
        &account,
        &secrets,
        &proxy,
        &account.email,
        &recipients,
        &message,
    )
    .await
    .map_err(|error| AppError::Mail(error.to_string()))?;
    if !is_automatic {
        MessageRepository::new(state.db.clone())
            .insert_sent(
                user_id,
                &account,
                OutgoingStoredMessage {
                    recipients: sent_recipients,
                    cc_recipients: sent_cc,
                    bcc_recipients: sent_bcc,
                    subject: input.subject,
                    body_text: input.text_body,
                    body_html: Some(html),
                    attachments: decoded_attachments
                        .into_iter()
                        .map(|attachment| MailAttachment {
                            filename: attachment.filename,
                            content_type: attachment.content_type,
                            size: attachment.content.len(),
                            content: Some(attachment.content),
                        })
                        .collect(),
                },
            )
            .await?;
    }
    Ok(())
}

fn default_apply_signature() -> bool {
    true
}

fn add_automatic_headers(
    builder: MessageBuilder<'_>,
    automatic: AutomaticMessageKind,
) -> MessageBuilder<'_> {
    builder
        .header("Auto-Submitted", Raw::new(automatic.header_value()))
        .header("X-Auto-Response-Suppress", Raw::new("All"))
        .header("Precedence", Raw::new("bulk"))
}

fn validate_attachments(attachments: &mut Vec<ComposeAttachmentInput>) -> Result<(), AppError> {
    if attachments.len() > MAX_OUTGOING_ATTACHMENTS {
        return Err(AppError::Validation("too many attachments".into()));
    }
    let mut total = 0_usize;
    for attachment in attachments {
        attachment.filename = clean_attachment_filename(&attachment.filename)?;
        attachment.content_type = clean_content_type(&attachment.content_type)?;
        if attachment.size > MAX_OUTGOING_ATTACHMENT_BYTES {
            return Err(AppError::Validation("attachment is too large".into()));
        }
        total = total
            .checked_add(attachment.size)
            .ok_or_else(|| AppError::Validation("attachments are too large".into()))?;
        if total > MAX_OUTGOING_ATTACHMENT_TOTAL_BYTES {
            return Err(AppError::Validation("attachments are too large".into()));
        }
        if attachment.content_base64.len() > MAX_OUTGOING_ATTACHMENT_TOTAL_BYTES * 2 {
            return Err(AppError::Validation(
                "attachment payload is too large".into(),
            ));
        }
    }
    Ok(())
}

fn decode_attachments(
    attachments: &[ComposeAttachmentInput],
) -> Result<Vec<DecodedAttachment>, AppError> {
    attachments
        .iter()
        .map(|attachment| {
            let content = STANDARD
                .decode(&attachment.content_base64)
                .map_err(|_| AppError::Validation("attachment payload is invalid".into()))?;
            if content.len() != attachment.size || content.len() > MAX_OUTGOING_ATTACHMENT_BYTES {
                return Err(AppError::Validation("attachment size is invalid".into()));
            }
            Ok(DecodedAttachment {
                filename: attachment.filename.clone(),
                content_type: attachment.content_type.clone(),
                content,
            })
        })
        .collect()
}

fn clean_attachment_filename(value: &str) -> Result<String, AppError> {
    let filename = value
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(value)
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(255)
        .collect::<String>();
    if filename.is_empty() || matches!(filename.as_str(), "." | "..") {
        Err(AppError::Validation(
            "attachment filename is invalid".into(),
        ))
    } else {
        Ok(filename)
    }
}

fn clean_content_type(value: &str) -> Result<String, AppError> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() <= 127
        && value.contains('/')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'-' | b'.'))
    {
        Ok(value)
    } else {
        Ok("application/octet-stream".into())
    }
}

fn first_subject_line(body: &str) -> String {
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .chars()
        .take(120)
        .collect()
}

fn append_signature(body: &str, signature: &str) -> String {
    if body.trim().is_empty() {
        signature.to_owned()
    } else {
        format!("{}\n\n-- \n{}", body.trim_end(), signature.trim())
    }
}

fn append_html_signature(body: &str, signature: &str) -> String {
    let signature = signature.trim();
    if signature.is_empty() {
        return body.to_owned();
    }
    let escaped = escape_html(signature).replace('\n', "<br>");
    format!(
        "{body}<div style=\"margin-top:24px;color:inherit\"><div style=\"font-family:ui-monospace,SFMono-Regular,Menlo,monospace;color:inherit\">-- </div><div>{escaped}</div></div>"
    )
}

fn sanitize_html(value: &str) -> String {
    ammonia::Builder::default()
        .add_generic_attributes(["align", "class", "height", "style", "width"])
        .clean(value)
        .to_string()
}

fn styled_html_body(body: &str, family: ComposeFontFamily, size: u8, color: &str) -> String {
    let family = match family {
        ComposeFontFamily::Default => "-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif",
        ComposeFontFamily::Serif => "Georgia,'Times New Roman',serif",
        ComposeFontFamily::Monospace => "Menlo,Monaco,'Courier New',monospace",
    };
    let escaped = escape_html(body).replace('\n', "<br>");
    format!(
        "<div style=\"font-family:{family};font-size:{size}px;line-height:1.6;color:{color};white-space:normal\">{escaped}</div>"
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use mail_builder::MessageBuilder;

    use super::{AutomaticMessageKind, add_automatic_headers};

    #[test]
    fn automatic_messages_emit_loop_suppression_headers() {
        for (kind, expected) in [
            (AutomaticMessageKind::Reply, "auto-replied"),
            (AutomaticMessageKind::Forward, "auto-forwarded"),
        ] {
            let message = add_automatic_headers(
                MessageBuilder::new()
                    .from("me@example.com")
                    .to("you@example.com")
                    .subject("Automatic message")
                    .text_body("Hello"),
                kind,
            )
            .write_to_vec()
            .unwrap();
            let message = String::from_utf8(message).unwrap();
            assert!(message.contains(&format!("Auto-Submitted: {expected}\r\n")));
            assert!(message.contains("X-Auto-Response-Suppress: All\r\n"));
            assert!(message.contains("Precedence: bulk\r\n"));
        }
    }
}
