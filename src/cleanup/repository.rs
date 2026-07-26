use std::collections::HashSet;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{
        Database,
        entities::{cleanup_rule, mail_account, mail_setting, message},
    },
    error::AppError,
    mail::ParsedMail,
};

use super::{CleanupRule, CleanupRuleInput, MailSettings};

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
        })
    }

    pub async fn update_settings(
        &self,
        user_id: Uuid,
        settings: MailSettings,
    ) -> Result<MailSettings, AppError> {
        let model = mail_setting::Entity::find_by_id(user_id.to_string())
            .one(self.db.connection())
            .await?
            .ok_or_else(|| AppError::internal(anyhow::anyhow!("mail settings are missing")))?;
        let mut active = model.into_active_model();
        active.keep_local_after_server_delete = Set(settings.keep_local_after_server_delete);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active.update(self.db.connection()).await?;
        Ok(settings)
    }

    pub async fn list(&self, user_id: Uuid) -> Result<Vec<CleanupRule>, AppError> {
        cleanup_rule::Entity::find()
            .filter(cleanup_rule::Column::UserId.eq(user_id.to_string()))
            .order_by_desc(cleanup_rule::Column::CreatedAt)
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

    pub fn match_new_mail<'a>(
        rules: &'a [CleanupRule],
        mail: &ParsedMail,
        now: i64,
    ) -> Option<&'a CleanupRule> {
        rules.iter().find(|rule| {
            rule_matches(
                rule,
                mail.sender_name.as_deref(),
                &mail.sender_email,
                &mail.subject,
                &mail.body_text,
                mail.received_at,
                now,
            )
        })
    }

    pub async fn apply_cached_rules(
        &self,
        user_id: Uuid,
        account_id: Uuid,
        rules: &[CleanupRule],
        now: i64,
    ) -> Result<Vec<i64>, AppError> {
        let models = message::Entity::find()
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .filter(message::Column::AccountId.eq(account_id.to_string()))
            .filter(message::Column::Folder.eq("INBOX"))
            .all(self.db.connection())
            .await?;
        let mut ids = Vec::new();
        let mut server_uids = Vec::new();
        for model in models {
            if let Some(rule) = rules.iter().find(|rule| {
                rule_matches(
                    rule,
                    model.sender_name.as_deref(),
                    &model.sender_email,
                    &model.subject,
                    &model.body_text,
                    model.received_at,
                    now,
                )
            }) {
                ids.push(model.id);
                if rule.delete_from_server {
                    server_uids.push(model.uid);
                }
            }
        }
        if !ids.is_empty() {
            message::Entity::delete_many()
                .filter(message::Column::Id.is_in(ids))
                .filter(message::Column::UserId.eq(user_id.to_string()))
                .exec(self.db.connection())
                .await?;
        }
        Ok(server_uids)
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

impl TryFrom<cleanup_rule::Model> for CleanupRule {
    type Error = AppError;

    fn try_from(model: cleanup_rule::Model) -> Result<Self, Self::Error> {
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
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

fn rule_matches(
    rule: &CleanupRule,
    sender_name: Option<&str>,
    sender_email: &str,
    subject: &str,
    body: &str,
    received_at: i64,
    now: i64,
) -> bool {
    let contains =
        |haystack: &str, needle: &str| haystack.to_lowercase().contains(&needle.to_lowercase());
    if let Some(needle) = rule.sender_contains.as_deref()
        && !contains(sender_email, needle)
        && !sender_name.is_some_and(|name| contains(name, needle))
    {
        return false;
    }
    if let Some(needle) = rule.subject_contains.as_deref()
        && !contains(subject, needle)
    {
        return false;
    }
    if let Some(needle) = rule.body_contains.as_deref()
        && !contains(body, needle)
    {
        return false;
    }
    if let Some(days) = rule.older_than_days
        && received_at > now.saturating_sub(i64::from(days) * 86_400)
    {
        return false;
    }
    true
}
