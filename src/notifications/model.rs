use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    pub enabled: bool,
    pub message_template: String,
    pub command_template: Option<String>,
    pub http_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEvent {
    pub account: String,
    pub email: String,
    pub sender: String,
    pub sender_email: String,
    pub subject: String,
    pub preview: String,
}
