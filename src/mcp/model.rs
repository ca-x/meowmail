use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct McpAccess {
    pub user_id: Uuid,
    pub token_id: Uuid,
    pub allow_delete: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    pub has_token: bool,
    pub allow_delete: bool,
    pub created_at: Option<i64>,
    pub last_used_at: Option<i64>,
    pub endpoint: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedMcpToken {
    #[serde(flatten)]
    pub settings: McpSettings,
    pub token: String,
}
