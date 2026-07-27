use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailSettings {
    pub keep_local_after_server_delete: bool,
    #[serde(default = "default_sync_fetch_limit")]
    pub sync_fetch_limit: Option<u32>,
}

impl MailSettings {
    pub fn validate(&self) -> Result<(), AppError> {
        if self
            .sync_fetch_limit
            .is_some_and(|limit| !(1..=10_000).contains(&limit))
        {
            return Err(AppError::Validation(
                "mail sync fetch limit is invalid".into(),
            ));
        }
        Ok(())
    }
}

fn default_sync_fetch_limit() -> Option<u32> {
    Some(50)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RuleMatchMode {
    #[default]
    All,
    Any,
}

impl RuleMatchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Any => "any",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        match value {
            "all" => Ok(Self::All),
            "any" => Ok(Self::Any),
            _ => Err(AppError::Validation("rule match mode is invalid".into())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuleField {
    Sender,
    SenderDomain,
    Recipient,
    Cc,
    RecipientOrCc,
    Subject,
    Body,
    AttachmentName,
    MessageSize,
    ReceivedAt,
    AgeDays,
    HasAttachment,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuleOperator {
    ContainsAny,
    ContainsAll,
    Equals,
    NotContains,
    GreaterThan,
    LessThan,
    Before,
    After,
    IsTrue,
    IsFalse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuleCondition {
    pub field: RuleField,
    pub operator: RuleOperator,
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuleActionKind {
    DeleteLocal,
    DeleteServer,
    MarkRead,
    MarkUnread,
    Star,
    Unstar,
    Forward,
    AutoReply,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuleAction {
    pub kind: RuleActionKind,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRule {
    pub id: Uuid,
    pub account_id: Option<Uuid>,
    pub name: String,
    pub match_mode: RuleMatchMode,
    pub conditions: Vec<RuleCondition>,
    pub actions: Vec<RuleAction>,
    pub position: i32,
    pub stop_processing: bool,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
    // Legacy fields remain in the response for compatibility with 0.2 clients.
    pub sender_contains: Option<String>,
    pub subject_contains: Option<String>,
    pub body_contains: Option<String>,
    pub older_than_days: Option<u32>,
    pub delete_from_server: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRuleInput {
    #[serde(default)]
    pub account_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub match_mode: RuleMatchMode,
    #[serde(default)]
    pub conditions: Vec<RuleCondition>,
    #[serde(default)]
    pub actions: Vec<RuleAction>,
    #[serde(default)]
    pub position: Option<i32>,
    #[serde(default)]
    pub stop_processing: bool,
    #[serde(default)]
    pub sender_contains: Option<String>,
    #[serde(default)]
    pub subject_contains: Option<String>,
    #[serde(default)]
    pub body_contains: Option<String>,
    #[serde(default)]
    pub older_than_days: Option<u32>,
    #[serde(default)]
    pub delete_from_server: bool,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

fn enabled_by_default() -> bool {
    true
}

impl CleanupRuleInput {
    pub fn normalize(&mut self) -> Result<(), AppError> {
        self.name = clean_required(&self.name, "rule name", 120)?;
        self.sender_contains = clean_optional(self.sender_contains.take(), "sender filter", 320)?;
        self.subject_contains =
            clean_optional(self.subject_contains.take(), "subject filter", 998)?;
        self.body_contains = clean_optional(self.body_contains.take(), "body filter", 2_000)?;
        if self
            .older_than_days
            .is_some_and(|days| days == 0 || days > 36_500)
        {
            return Err(AppError::Validation("mail age is invalid".into()));
        }
        if self.conditions.is_empty() {
            if let Some(value) = self.sender_contains.clone() {
                self.conditions
                    .push(text_condition(RuleField::Sender, value));
            }
            if let Some(value) = self.subject_contains.clone() {
                self.conditions
                    .push(text_condition(RuleField::Subject, value));
            }
            if let Some(value) = self.body_contains.clone() {
                self.conditions.push(text_condition(RuleField::Body, value));
            }
            if let Some(value) = self.older_than_days {
                self.conditions.push(RuleCondition {
                    field: RuleField::AgeDays,
                    operator: RuleOperator::GreaterThan,
                    values: vec![value.to_string()],
                });
            }
        }
        if self.conditions.is_empty() || self.conditions.len() > 20 {
            return Err(AppError::Validation(
                "a cleanup rule needs at least one condition".into(),
            ));
        }
        for condition in &mut self.conditions {
            normalize_condition(condition)?;
        }
        if self.actions.is_empty() {
            self.actions.push(RuleAction {
                kind: if self.delete_from_server {
                    RuleActionKind::DeleteServer
                } else {
                    RuleActionKind::DeleteLocal
                },
                value: None,
            });
        }
        if self.actions.len() > 10 {
            return Err(AppError::Validation("rule has too many actions".into()));
        }
        for action in &mut self.actions {
            normalize_action(action)?;
        }
        let has_delete = self.actions.iter().any(|action| {
            matches!(
                action.kind,
                RuleActionKind::DeleteLocal | RuleActionKind::DeleteServer
            )
        });
        let has_external = self.actions.iter().any(|action| {
            matches!(
                action.kind,
                RuleActionKind::Forward | RuleActionKind::AutoReply
            )
        });
        if has_delete && has_external {
            return Err(AppError::Validation(
                "forward or auto-reply cannot be combined with deletion".into(),
            ));
        }
        self.sender_contains = legacy_text(&self.conditions, RuleField::Sender);
        self.subject_contains = legacy_text(&self.conditions, RuleField::Subject);
        self.body_contains = legacy_text(&self.conditions, RuleField::Body);
        self.older_than_days = self.conditions.iter().find_map(|condition| {
            (condition.field == RuleField::AgeDays)
                .then(|| condition.values.first()?.parse::<u32>().ok())
                .flatten()
        });
        self.delete_from_server = self
            .actions
            .iter()
            .any(|action| action.kind == RuleActionKind::DeleteServer);
        Ok(())
    }
}

fn text_condition(field: RuleField, value: String) -> RuleCondition {
    RuleCondition {
        field,
        operator: RuleOperator::ContainsAny,
        values: vec![value],
    }
}

fn normalize_condition(condition: &mut RuleCondition) -> Result<(), AppError> {
    for value in &mut condition.values {
        *value = value.trim().to_owned();
    }
    condition.values.retain(|value| !value.is_empty());
    if condition.values.len() > 20
        || condition
            .values
            .iter()
            .any(|value| value.chars().count() > 2_000 || value.chars().any(char::is_control))
    {
        return Err(AppError::Validation(
            "rule condition value is invalid".into(),
        ));
    }
    match condition.field {
        RuleField::HasAttachment => {
            if !matches!(
                condition.operator,
                RuleOperator::IsTrue | RuleOperator::IsFalse
            ) {
                return Err(AppError::Validation("rule operator is invalid".into()));
            }
            condition.values.clear();
        }
        RuleField::MessageSize | RuleField::AgeDays | RuleField::ReceivedAt => {
            if condition.values.len() != 1
                || condition.values[0]
                    .parse::<i64>()
                    .ok()
                    .is_none_or(|value| value < 0)
                || !matches!(
                    condition.operator,
                    RuleOperator::GreaterThan
                        | RuleOperator::LessThan
                        | RuleOperator::Before
                        | RuleOperator::After
                        | RuleOperator::Equals
                )
            {
                return Err(AppError::Validation(
                    "numeric rule condition is invalid".into(),
                ));
            }
        }
        _ => {
            if condition.values.is_empty()
                || !matches!(
                    condition.operator,
                    RuleOperator::ContainsAny
                        | RuleOperator::ContainsAll
                        | RuleOperator::Equals
                        | RuleOperator::NotContains
                )
            {
                return Err(AppError::Validation(
                    "text rule condition is invalid".into(),
                ));
            }
        }
    }
    Ok(())
}

fn normalize_action(action: &mut RuleAction) -> Result<(), AppError> {
    action.value = action
        .value
        .take()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    match action.kind {
        RuleActionKind::Forward => {
            let value = action
                .value
                .as_ref()
                .ok_or_else(|| AppError::Validation("forward address is required".into()))?;
            if value.len() > 254 || !value.contains('@') || value.contains(['\r', '\n']) {
                return Err(AppError::Validation("forward address is invalid".into()));
            }
        }
        RuleActionKind::AutoReply => {
            if action
                .value
                .as_ref()
                .is_none_or(|value| value.len() > 32 * 1024 || value.contains('\0'))
            {
                return Err(AppError::Validation("auto-reply content is invalid".into()));
            }
        }
        _ => action.value = None,
    }
    Ok(())
}

fn legacy_text(conditions: &[RuleCondition], field: RuleField) -> Option<String> {
    conditions
        .iter()
        .find(|condition| condition.field == field)
        .and_then(|condition| condition.values.first().cloned())
}

fn clean_required(value: &str, label: &str, max: usize) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max || value.chars().any(char::is_control) {
        return Err(AppError::Validation(format!("{label} is invalid")));
    }
    Ok(value.to_owned())
}

fn clean_optional(
    value: Option<String>,
    label: &str,
    max: usize,
) -> Result<Option<String>, AppError> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .map(|value| clean_required(&value, label, max))
        .transpose()
}
