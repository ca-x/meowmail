use std::collections::HashMap;

use crate::error::AppError;

use super::NotificationEvent;

const ALLOWED: &[&str] = &[
    "account",
    "email",
    "sender",
    "sender_email",
    "subject",
    "preview",
    "message",
];

pub fn render_template(
    template: &str,
    event: &NotificationEvent,
    message: Option<&str>,
) -> Result<String, AppError> {
    let values = HashMap::from([
        ("account", event.account.as_str()),
        ("email", event.email.as_str()),
        ("sender", event.sender.as_str()),
        ("sender_email", event.sender_email.as_str()),
        ("subject", event.subject.as_str()),
        ("preview", event.preview.as_str()),
        ("message", message.unwrap_or("")),
    ]);
    let mut output = String::with_capacity(template.len() + 64);
    let mut remaining = template;
    while let Some(open) = remaining.find('{') {
        output.push_str(&remaining[..open]);
        let after_open = &remaining[open + 1..];
        let close = after_open.find('}').ok_or_else(|| {
            AppError::Validation("notification template has an unmatched `{`".into())
        })?;
        let name = &after_open[..close];
        if !ALLOWED.contains(&name) {
            return Err(AppError::Validation(format!(
                "notification template contains unknown placeholder {{{name}}}"
            )));
        }
        output.push_str(values.get(name).copied().unwrap_or_default());
        remaining = &after_open[close + 1..];
    }
    if remaining.contains('}') {
        return Err(AppError::Validation(
            "notification template has an unmatched `}`".into(),
        ));
    }
    output.push_str(remaining);
    Ok(output)
}
