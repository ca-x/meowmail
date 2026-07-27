use std::collections::{HashMap, HashSet};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{
        Database,
        entities::{cleanup_rule, mail_account, mail_setting, message, message_attachment},
    },
    error::AppError,
    mail::ParsedMail,
};

use super::{
    CleanupRule, CleanupRuleInput, MailSettings, RuleActionKind, RuleCondition, RuleField,
    RuleMatchMode, RuleOperator,
};

#[derive(Debug, Clone, Default)]
pub struct RuleOutcome {
    pub matched: bool,
    pub delete_local: bool,
    pub delete_server: bool,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub forwards: Vec<String>,
    pub auto_replies: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CachedRuleOutcome {
    pub server_uids: Vec<i64>,
    pub server_message_ids: Vec<Uuid>,
}

#[derive(Clone)]
pub struct CleanupRepository {
    db: Database,
}

impl CleanupRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn settings(&self, user_id: Uuid) -> Result<MailSettings, AppError> {
        let model = mail_setting::Entity::find_by_id(user_id.to_string())
            .one(self.db.connection())
            .await?
            .ok_or_else(|| AppError::internal(anyhow::anyhow!("mail settings are missing")))?;
        Ok(MailSettings {
            keep_local_after_server_delete: model.keep_local_after_server_delete,
            sync_fetch_limit: model.sync_fetch_limit.map(|value| value as u32),
        })
    }

    pub async fn update_settings(
        &self,
        user_id: Uuid,
        settings: MailSettings,
    ) -> Result<MailSettings, AppError> {
        settings.validate()?;
        let model = mail_setting::Entity::find_by_id(user_id.to_string())
            .one(self.db.connection())
            .await?
            .ok_or_else(|| AppError::internal(anyhow::anyhow!("mail settings are missing")))?;
        let mut active = model.into_active_model();
        active.keep_local_after_server_delete = Set(settings.keep_local_after_server_delete);
        active.sync_fetch_limit = Set(settings.sync_fetch_limit.map(|value| value as i32));
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active.update(self.db.connection()).await?;
        Ok(settings)
    }

    pub async fn list(&self, user_id: Uuid) -> Result<Vec<CleanupRule>, AppError> {
        cleanup_rule::Entity::find()
            .filter(cleanup_rule::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(cleanup_rule::Column::Position)
            .order_by_asc(cleanup_rule::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(CleanupRule::try_from)
            .collect()
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        mut input: CleanupRuleInput,
    ) -> Result<CleanupRule, AppError> {
        input.normalize()?;
        self.validate_account(user_id, input.account_id).await?;
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let position = match input.position {
            Some(position) => position,
            None => self
                .list(user_id)
                .await?
                .last()
                .map_or(0, |rule| rule.position.saturating_add(1)),
        };
        let model = cleanup_rule::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(user_id.to_string()),
            account_id: Set(input.account_id.map(|value| value.to_string())),
            name: Set(input.name),
            sender_contains: Set(input.sender_contains),
            subject_contains: Set(input.subject_contains),
            body_contains: Set(input.body_contains),
            older_than_days: Set(input.older_than_days.map(|value| value as i32)),
            delete_from_server: Set(input.delete_from_server),
            enabled: Set(input.enabled),
            position: Set(position),
            match_mode: Set(input.match_mode.as_str().into()),
            conditions_json: Set(
                serde_json::to_string(&input.conditions).map_err(AppError::internal)?
            ),
            actions_json: Set(serde_json::to_string(&input.actions).map_err(AppError::internal)?),
            stop_processing: Set(input.stop_processing),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(self.db.connection())
        .await?;
        CleanupRule::try_from(model)
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: CleanupRuleInput,
    ) -> Result<CleanupRule, AppError> {
        input.normalize()?;
        self.validate_account(user_id, input.account_id).await?;
        let model = self.get_model(user_id, id).await?;
        let mut active = model.into_active_model();
        active.account_id = Set(input.account_id.map(|value| value.to_string()));
        active.name = Set(input.name);
        active.sender_contains = Set(input.sender_contains);
        active.subject_contains = Set(input.subject_contains);
        active.body_contains = Set(input.body_contains);
        active.older_than_days = Set(input.older_than_days.map(|value| value as i32));
        active.delete_from_server = Set(input.delete_from_server);
        active.enabled = Set(input.enabled);
        if let Some(position) = input.position {
            active.position = Set(position);
        }
        active.match_mode = Set(input.match_mode.as_str().into());
        active.conditions_json =
            Set(serde_json::to_string(&input.conditions).map_err(AppError::internal)?);
        active.actions_json =
            Set(serde_json::to_string(&input.actions).map_err(AppError::internal)?);
        active.stop_processing = Set(input.stop_processing);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        CleanupRule::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = cleanup_rule::Entity::delete_many()
            .filter(cleanup_rule::Column::Id.eq(id.to_string()))
            .filter(cleanup_rule::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn reorder(&self, user_id: Uuid, ids: &[Uuid]) -> Result<Vec<CleanupRule>, AppError> {
        let current = self.list(user_id).await?;
        let current_ids = current.iter().map(|rule| rule.id).collect::<HashSet<_>>();
        let supplied = ids.iter().copied().collect::<HashSet<_>>();
        if ids.len() != current.len() || supplied.len() != ids.len() || supplied != current_ids {
            return Err(AppError::Validation("rule order is invalid".into()));
        }
        let transaction = self.db.connection().begin().await?;
        for (position, id) in ids.iter().enumerate() {
            let model = cleanup_rule::Entity::find()
                .filter(cleanup_rule::Column::Id.eq(id.to_string()))
                .filter(cleanup_rule::Column::UserId.eq(user_id.to_string()))
                .one(&transaction)
                .await?
                .ok_or(AppError::NotFound)?;
            let mut active = model.into_active_model();
            active.position = Set(i32::try_from(position).map_err(AppError::internal)?);
            active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
            active.update(&transaction).await?;
        }
        transaction.commit().await?;
        self.list(user_id).await
    }

    pub async fn enabled_for_account(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<Vec<CleanupRule>, AppError> {
        Ok(self
            .list(user_id)
            .await?
            .into_iter()
            .filter(|rule| rule.enabled && rule.account_id.is_none_or(|value| value == account_id))
            .collect())
    }

    pub fn match_new_mail(rules: &[CleanupRule], mail: &ParsedMail, now: i64) -> RuleOutcome {
        evaluate_rules(
            rules,
            &RuleContext {
                sender_name: mail.sender_name.as_deref(),
                sender_email: &mail.sender_email,
                recipients: &mail.recipients,
                cc_recipients: &mail.cc_recipients,
                subject: &mail.subject,
                body: &mail.body_text,
                attachment_names: mail
                    .attachments
                    .iter()
                    .map(|attachment| attachment.filename.as_str())
                    .collect(),
                attachment_count: mail.attachment_count,
                raw_size: i64::try_from(mail.raw_size).unwrap_or(i64::MAX),
                received_at: mail.received_at,
                auto_response_allowed: mail.auto_response_allowed,
                now,
            },
        )
    }

    pub async fn apply_cached_rules(
        &self,
        user_id: Uuid,
        account_id: Uuid,
        selected_uid_validity: Option<i64>,
        rules: &[CleanupRule],
        now: i64,
    ) -> Result<CachedRuleOutcome, AppError> {
        let models = message::Entity::find()
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .filter(message::Column::AccountId.eq(account_id.to_string()))
            .filter(message::Column::Folder.eq("INBOX"))
            .all(self.db.connection())
            .await?;
        let message_ids = models
            .iter()
            .map(|model| model.id.clone())
            .collect::<Vec<_>>();
        let mut attachment_names: HashMap<String, Vec<String>> = HashMap::new();
        if !message_ids.is_empty() {
            let attachments = message_attachment::Entity::find()
                .select_only()
                .columns([
                    message_attachment::Column::MessageId,
                    message_attachment::Column::Filename,
                ])
                .filter(message_attachment::Column::MessageId.is_in(message_ids))
                .into_tuple::<(String, String)>()
                .all(self.db.connection())
                .await?;
            for (message_id, filename) in attachments {
                attachment_names
                    .entry(message_id)
                    .or_default()
                    .push(filename);
            }
        }
        let mut local_delete_ids = Vec::new();
        let mut outcome_for_server = CachedRuleOutcome::default();
        for model in models {
            let recipients = serde_json::from_str::<Vec<String>>(&model.recipients_json)
                .map_err(AppError::internal)?;
            let cc_recipients = serde_json::from_str::<Vec<String>>(&model.cc_recipients_json)
                .map_err(AppError::internal)?;
            let names = attachment_names.get(&model.id).cloned().unwrap_or_default();
            let name_refs = names.iter().map(String::as_str).collect::<Vec<_>>();
            let outcome = evaluate_rules(
                rules,
                &RuleContext {
                    sender_name: model.sender_name.as_deref(),
                    sender_email: &model.sender_email,
                    recipients: &recipients,
                    cc_recipients: &cc_recipients,
                    subject: &model.subject,
                    body: &model.body_text,
                    attachment_names: name_refs,
                    attachment_count: model.attachment_count.max(0) as usize,
                    raw_size: model.raw_size,
                    received_at: model.received_at,
                    auto_response_allowed: model.auto_response_allowed,
                    now,
                },
            );
            if outcome.delete_server {
                if uid_validity_matches(model.uid_validity, selected_uid_validity) {
                    outcome_for_server.server_uids.push(model.uid);
                    outcome_for_server
                        .server_message_ids
                        .push(Uuid::parse_str(&model.id).map_err(AppError::internal)?);
                }
            } else if outcome.delete_local {
                local_delete_ids.push(model.id.clone());
            } else if outcome.is_read.is_some() || outcome.is_starred.is_some() {
                let mut active = model.into_active_model();
                if let Some(value) = outcome.is_read {
                    active.is_read = Set(value);
                }
                if let Some(value) = outcome.is_starred {
                    active.is_starred = Set(value);
                }
                active.update(self.db.connection()).await?;
            }
        }
        if !local_delete_ids.is_empty() {
            message::Entity::delete_many()
                .filter(message::Column::Id.is_in(local_delete_ids))
                .filter(message::Column::UserId.eq(user_id.to_string()))
                .exec(self.db.connection())
                .await?;
        }
        Ok(outcome_for_server)
    }

    pub async fn delete_cached_after_server_success(
        &self,
        user_id: Uuid,
        ids: &[Uuid],
    ) -> Result<u64, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        Ok(message::Entity::delete_many()
            .filter(message::Column::Id.is_in(ids.iter().map(Uuid::to_string)))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?
            .rows_affected)
    }

    pub async fn reconcile_server_uids(
        &self,
        user_id: Uuid,
        account_id: Uuid,
        server_uids: &HashSet<u32>,
    ) -> Result<u64, AppError> {
        let local = message::Entity::find()
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .filter(message::Column::AccountId.eq(account_id.to_string()))
            .filter(message::Column::Folder.eq("INBOX"))
            .all(self.db.connection())
            .await?;
        let ids = local
            .into_iter()
            .filter(|model| !server_uids.contains(&(model.uid as u32)))
            .map(|model| model.id)
            .collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(0);
        }
        Ok(message::Entity::delete_many()
            .filter(message::Column::Id.is_in(ids))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?
            .rows_affected)
    }

    async fn get_model(&self, user_id: Uuid, id: Uuid) -> Result<cleanup_rule::Model, AppError> {
        cleanup_rule::Entity::find()
            .filter(cleanup_rule::Column::Id.eq(id.to_string()))
            .filter(cleanup_rule::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn validate_account(
        &self,
        user_id: Uuid,
        account_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        let Some(account_id) = account_id else {
            return Ok(());
        };
        if mail_account::Entity::find()
            .filter(mail_account::Column::Id.eq(account_id.to_string()))
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .is_none()
        {
            return Err(AppError::Validation("cleanup account is invalid".into()));
        }
        Ok(())
    }
}

fn uid_validity_matches(cached: Option<i64>, selected: Option<i64>) -> bool {
    cached.is_some() && cached == selected
}

impl TryFrom<cleanup_rule::Model> for CleanupRule {
    type Error = AppError;

    fn try_from(model: cleanup_rule::Model) -> Result<Self, Self::Error> {
        let match_mode = RuleMatchMode::parse(&model.match_mode)?;
        let conditions = parse_conditions(&model)?;
        let actions = parse_actions(&model)?;
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            account_id: model
                .account_id
                .map(|value| Uuid::parse_str(&value).map_err(AppError::internal))
                .transpose()?,
            name: model.name,
            sender_contains: model.sender_contains,
            subject_contains: model.subject_contains,
            body_contains: model.body_contains,
            older_than_days: model.older_than_days.map(|value| value as u32),
            delete_from_server: model.delete_from_server,
            enabled: model.enabled,
            position: model.position,
            match_mode,
            conditions,
            actions,
            stop_processing: model.stop_processing,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

fn parse_conditions(model: &cleanup_rule::Model) -> Result<Vec<RuleCondition>, AppError> {
    let parsed = serde_json::from_str::<Vec<RuleCondition>>(&model.conditions_json)
        .map_err(AppError::internal)?;
    if !parsed.is_empty() {
        return Ok(parsed);
    }
    let mut legacy = CleanupRuleInput {
        account_id: None,
        name: model.name.clone(),
        match_mode: RuleMatchMode::All,
        conditions: Vec::new(),
        actions: Vec::new(),
        position: Some(model.position),
        stop_processing: model.stop_processing,
        sender_contains: model.sender_contains.clone(),
        subject_contains: model.subject_contains.clone(),
        body_contains: model.body_contains.clone(),
        older_than_days: model.older_than_days.map(|value| value as u32),
        delete_from_server: model.delete_from_server,
        enabled: model.enabled,
    };
    legacy.normalize()?;
    Ok(legacy.conditions)
}

fn parse_actions(model: &cleanup_rule::Model) -> Result<Vec<super::RuleAction>, AppError> {
    let parsed = serde_json::from_str::<Vec<super::RuleAction>>(&model.actions_json)
        .map_err(AppError::internal)?;
    if !parsed.is_empty() {
        return Ok(parsed);
    }
    Ok(vec![super::RuleAction {
        kind: if model.delete_from_server {
            RuleActionKind::DeleteServer
        } else {
            RuleActionKind::DeleteLocal
        },
        value: None,
    }])
}

struct RuleContext<'a> {
    sender_name: Option<&'a str>,
    sender_email: &'a str,
    recipients: &'a [String],
    cc_recipients: &'a [String],
    subject: &'a str,
    body: &'a str,
    attachment_names: Vec<&'a str>,
    attachment_count: usize,
    raw_size: i64,
    received_at: i64,
    auto_response_allowed: bool,
    now: i64,
}

fn evaluate_rules(rules: &[CleanupRule], context: &RuleContext<'_>) -> RuleOutcome {
    let mut outcome = RuleOutcome::default();
    for rule in rules.iter().filter(|rule| rule.enabled) {
        let matches = match rule.match_mode {
            RuleMatchMode::All => rule
                .conditions
                .iter()
                .all(|condition| condition_matches(condition, context)),
            RuleMatchMode::Any => rule
                .conditions
                .iter()
                .any(|condition| condition_matches(condition, context)),
        };
        if !matches {
            continue;
        }
        outcome.matched = true;
        for action in &rule.actions {
            match action.kind {
                RuleActionKind::DeleteLocal => outcome.delete_local = true,
                RuleActionKind::DeleteServer => {
                    outcome.delete_server = true;
                    outcome.delete_local = true;
                }
                RuleActionKind::MarkRead => outcome.is_read = Some(true),
                RuleActionKind::MarkUnread => outcome.is_read = Some(false),
                RuleActionKind::Star => outcome.is_starred = Some(true),
                RuleActionKind::Unstar => outcome.is_starred = Some(false),
                RuleActionKind::Forward => {
                    if let Some(value) = action.value.clone() {
                        outcome.forwards.push(value);
                    }
                }
                RuleActionKind::AutoReply if context.auto_response_allowed => {
                    if let Some(value) = action.value.clone() {
                        outcome.auto_replies.push(value);
                    }
                }
                RuleActionKind::AutoReply => {}
            }
        }
        if rule.stop_processing {
            break;
        }
    }
    outcome
}

fn condition_matches(condition: &RuleCondition, context: &RuleContext<'_>) -> bool {
    match condition.field {
        RuleField::HasAttachment => match condition.operator {
            RuleOperator::IsTrue => context.attachment_count > 0,
            RuleOperator::IsFalse => context.attachment_count == 0,
            _ => false,
        },
        RuleField::MessageSize => numeric_matches(context.raw_size, condition),
        RuleField::ReceivedAt => numeric_matches(context.received_at, condition),
        RuleField::AgeDays => numeric_matches(
            context.now.saturating_sub(context.received_at) / 86_400,
            condition,
        ),
        RuleField::Sender => text_matches(
            &format!(
                "{} <{}>",
                context.sender_name.unwrap_or_default(),
                context.sender_email
            ),
            condition,
        ),
        RuleField::SenderDomain => text_matches(
            context.sender_email.split('@').nth(1).unwrap_or_default(),
            condition,
        ),
        RuleField::Recipient => text_matches(&context.recipients.join(", "), condition),
        RuleField::Cc => text_matches(&context.cc_recipients.join(", "), condition),
        RuleField::RecipientOrCc => text_matches(
            &context
                .recipients
                .iter()
                .chain(context.cc_recipients)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
            condition,
        ),
        RuleField::Subject => text_matches(context.subject, condition),
        RuleField::Body => text_matches(context.body, condition),
        RuleField::AttachmentName => text_matches(&context.attachment_names.join(", "), condition),
    }
}

fn text_matches(haystack: &str, condition: &RuleCondition) -> bool {
    let haystack = haystack.to_lowercase();
    let values = condition
        .values
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>();
    match condition.operator {
        RuleOperator::ContainsAny => values.iter().any(|value| haystack.contains(value)),
        RuleOperator::ContainsAll => values.iter().all(|value| haystack.contains(value)),
        RuleOperator::Equals => values.iter().any(|value| haystack.trim() == value.trim()),
        RuleOperator::NotContains => values.iter().all(|value| !haystack.contains(value)),
        _ => false,
    }
}

fn numeric_matches(actual: i64, condition: &RuleCondition) -> bool {
    let Some(expected) = condition
        .values
        .first()
        .and_then(|value| value.parse::<i64>().ok())
    else {
        return false;
    };
    match condition.operator {
        RuleOperator::GreaterThan | RuleOperator::After => actual > expected,
        RuleOperator::LessThan | RuleOperator::Before => actual < expected,
        RuleOperator::Equals => actual == expected,
        _ => false,
    }
}
