use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
    TransactionTrait, sea_query::OnConflict,
};
use secrecy::SecretString;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    accounts::{ProxyConfig, PublicProxyConfig},
    db::{
        Database,
        entities::{
            ai_provider, auto_label_rule, auto_label_subscription, label, mail_account, message,
            message_label,
        },
    },
    error::AppError,
    security::CredentialVault,
};

use super::model::{
    AUTO_LABEL_FEED_FORMAT, AUTO_LABEL_FEED_VERSION, AiApiType, AiProvider, AiProviderInput,
    AiProviderKind, AutoLabelFeedLabel, AutoLabelFeedRule, AutoLabelResult, AutoLabelRule,
    AutoLabelRuleFeed, AutoLabelRuleInput, AutoLabelSubscription, AutoLabelSubscriptionInput,
    Label, LabelInput,
};

#[derive(Clone)]
pub struct AiRepository {
    db: Database,
    vault: CredentialVault,
}

#[derive(Clone)]
pub struct AiProviderSecrets {
    pub api_key: Option<SecretString>,
    pub proxy_password: Option<SecretString>,
}

impl AiRepository {
    pub fn new(db: Database, vault: CredentialVault) -> Self {
        Self { db, vault }
    }

    pub async fn list_providers(&self, user_id: Uuid) -> Result<Vec<AiProvider>, AppError> {
        ai_provider::Entity::find()
            .filter(ai_provider::Column::UserId.eq(user_id.to_string()))
            .order_by_desc(ai_provider::Column::IsDefault)
            .order_by_asc(ai_provider::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(AiProvider::try_from)
            .collect()
    }

    pub async fn get_provider(&self, user_id: Uuid, id: Uuid) -> Result<AiProvider, AppError> {
        AiProvider::try_from(self.provider_model(user_id, id).await?)
    }

    pub async fn default_provider(
        &self,
        user_id: Uuid,
    ) -> Result<(ai_provider::Model, AiProviderSecrets, ProxyConfig), AppError> {
        let model = ai_provider::Entity::find()
            .filter(ai_provider::Column::UserId.eq(user_id.to_string()))
            .filter(ai_provider::Column::Enabled.eq(true))
            .order_by_desc(ai_provider::Column::IsDefault)
            .order_by_asc(ai_provider::Column::CreatedAt)
            .one(self.db.connection())
            .await?
            .ok_or_else(|| AppError::Validation("AI provider is not configured".into()))?;
        let id = Uuid::parse_str(&model.id).map_err(AppError::internal)?;
        self.get_provider_with_secrets(user_id, id).await
    }

    pub async fn get_provider_with_secrets(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<(ai_provider::Model, AiProviderSecrets, ProxyConfig), AppError> {
        let model = self.provider_model(user_id, id).await?;
        let secrets = AiProviderSecrets {
            api_key: model
                .api_key_cipher
                .as_deref()
                .map(|value| self.vault.open(value).map_err(AppError::internal))
                .transpose()?,
            proxy_password: model
                .proxy_password_cipher
                .as_deref()
                .map(|value| self.vault.open(value).map_err(AppError::internal))
                .transpose()?,
        };
        let proxy = ProxyConfig {
            kind: crate::accounts::ProxyKind::parse(&model.proxy_kind)?,
            host: model.proxy_host.clone(),
            port: model.proxy_port.map(|value| value as u16),
            username: model.proxy_username.clone(),
            password: secrets.proxy_password.clone(),
        };
        Ok((model, secrets, proxy))
    }

    pub async fn create_provider(
        &self,
        user_id: Uuid,
        mut input: AiProviderInput,
    ) -> Result<AiProvider, AppError> {
        input.normalize(true)?;
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let api_key_cipher = input
            .api_key
            .as_deref()
            .map(|value| self.vault.seal(value).map_err(AppError::internal))
            .transpose()?;
        let proxy_password_cipher = input
            .proxy
            .password
            .as_deref()
            .map(|value| self.vault.seal(value).map_err(AppError::internal))
            .transpose()?;
        let transaction = self.db.connection().begin().await?;
        if input.is_default {
            clear_default_provider(&transaction, user_id).await?;
        }
        ai_provider::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(user_id.to_string()),
            name: Set(input.name),
            provider_kind: Set(input.provider_kind.as_str().into()),
            api_type: Set(input.api_type.as_str().into()),
            model: Set(input.model),
            base_url: Set(input.base_url),
            api_key_cipher: Set(api_key_cipher),
            proxy_kind: Set(input.proxy.kind.as_str().into()),
            proxy_host: Set(input.proxy.host),
            proxy_port: Set(input.proxy.port.map(i32::from)),
            proxy_username: Set(input.proxy.username),
            proxy_password_cipher: Set(proxy_password_cipher),
            is_default: Set(input.is_default),
            enabled: Set(input.enabled),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&transaction)
        .await?;
        transaction.commit().await?;
        self.get_provider(user_id, id).await
    }

    pub async fn update_provider(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: AiProviderInput,
    ) -> Result<AiProvider, AppError> {
        input.normalize(false)?;
        let existing = self.provider_model(user_id, id).await?;
        let api_key_cipher = match input.api_key.as_deref() {
            Some(value) => Some(self.vault.seal(value).map_err(AppError::internal)?),
            None => existing.api_key_cipher.clone(),
        };
        let proxy_password_cipher = if input.proxy.kind == crate::accounts::ProxyKind::Direct {
            None
        } else {
            match input.proxy.password.as_deref() {
                Some(value) => Some(self.vault.seal(value).map_err(AppError::internal)?),
                None => existing.proxy_password_cipher.clone(),
            }
        };
        let transaction = self.db.connection().begin().await?;
        if input.is_default {
            clear_default_provider(&transaction, user_id).await?;
        }
        let mut active = existing.into_active_model();
        active.name = Set(input.name);
        active.provider_kind = Set(input.provider_kind.as_str().into());
        active.api_type = Set(input.api_type.as_str().into());
        active.model = Set(input.model);
        active.base_url = Set(input.base_url);
        active.api_key_cipher = Set(api_key_cipher);
        active.proxy_kind = Set(input.proxy.kind.as_str().into());
        active.proxy_host = Set(input.proxy.host);
        active.proxy_port = Set(input.proxy.port.map(i32::from));
        active.proxy_username = Set(input.proxy.username);
        active.proxy_password_cipher = Set(proxy_password_cipher);
        active.is_default = Set(input.is_default);
        active.enabled = Set(input.enabled);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active.update(&transaction).await?;
        transaction.commit().await?;
        self.get_provider(user_id, id).await
    }

    pub async fn delete_provider(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = ai_provider::Entity::delete_many()
            .filter(ai_provider::Column::Id.eq(id.to_string()))
            .filter(ai_provider::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn list_labels(&self, user_id: Uuid) -> Result<Vec<Label>, AppError> {
        label::Entity::find()
            .filter(label::Column::UserId.eq(user_id.to_string()))
            .order_by_desc(label::Column::IsAuto)
            .order_by_asc(label::Column::Name)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Label::try_from)
            .collect()
    }

    pub async fn create_label(
        &self,
        user_id: Uuid,
        mut input: LabelInput,
    ) -> Result<Label, AppError> {
        input.normalize()?;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        Label::try_from(
            label::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                name: Set(input.name),
                color: Set(input.color),
                is_auto: Set(input.is_auto),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(self.db.connection())
            .await?,
        )
    }

    pub async fn update_label(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: LabelInput,
    ) -> Result<Label, AppError> {
        input.normalize()?;
        let model = label::Entity::find()
            .filter(label::Column::Id.eq(id.to_string()))
            .filter(label::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let mut active = model.into_active_model();
        active.name = Set(input.name);
        active.color = Set(input.color);
        active.is_auto = Set(input.is_auto);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        Label::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn delete_label(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = label::Entity::delete_many()
            .filter(label::Column::Id.eq(id.to_string()))
            .filter(label::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn list_auto_label_rules(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<AutoLabelRule>, AppError> {
        auto_label_rule::Entity::find()
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(auto_label_rule::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(AutoLabelRule::try_from)
            .collect()
    }

    pub async fn create_auto_label_rule(
        &self,
        user_id: Uuid,
        mut input: AutoLabelRuleInput,
    ) -> Result<AutoLabelRule, AppError> {
        input.normalize()?;
        self.validate_auto_label_refs(user_id, &input).await?;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        AutoLabelRule::try_from(
            auto_label_rule::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                account_id: Set(input.account_id.map(|value| value.to_string())),
                provider_id: Set(input.provider_id.map(|value| value.to_string())),
                name: Set(input.name),
                label_ids_json: Set(
                    serde_json::to_string(&input.label_ids).map_err(AppError::internal)?
                ),
                instructions: Set(input.instructions),
                enabled: Set(input.enabled),
                apply_automatically: Set(input.apply_automatically),
                source_subscription_id: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(self.db.connection())
            .await?,
        )
    }

    pub async fn update_auto_label_rule(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: AutoLabelRuleInput,
    ) -> Result<AutoLabelRule, AppError> {
        input.normalize()?;
        self.validate_auto_label_refs(user_id, &input).await?;
        let model = auto_label_rule::Entity::find()
            .filter(auto_label_rule::Column::Id.eq(id.to_string()))
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        if model.source_subscription_id.is_some() {
            return Err(AppError::Validation(
                "subscribed auto-label rules are read-only".into(),
            ));
        }
        let mut active = model.into_active_model();
        active.account_id = Set(input.account_id.map(|value| value.to_string()));
        active.provider_id = Set(input.provider_id.map(|value| value.to_string()));
        active.name = Set(input.name);
        active.label_ids_json =
            Set(serde_json::to_string(&input.label_ids).map_err(AppError::internal)?);
        active.instructions = Set(input.instructions);
        active.enabled = Set(input.enabled);
        active.apply_automatically = Set(input.apply_automatically);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        AutoLabelRule::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn delete_auto_label_rule(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let model = auto_label_rule::Entity::find()
            .filter(auto_label_rule::Column::Id.eq(id.to_string()))
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        if model.source_subscription_id.is_some() {
            return Err(AppError::Validation(
                "subscribed auto-label rules are read-only".into(),
            ));
        }
        let result = auto_label_rule::Entity::delete_many()
            .filter(auto_label_rule::Column::Id.eq(id.to_string()))
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn export_auto_label_feed(
        &self,
        user_id: Uuid,
    ) -> Result<AutoLabelRuleFeed, AppError> {
        let providers = ai_provider::Entity::find()
            .filter(ai_provider::Column::UserId.eq(user_id.to_string()))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(|provider| (provider.id, provider.name))
            .collect::<HashMap<_, _>>();
        let accounts = mail_account::Entity::find()
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(|account| (account.id, account.email))
            .collect::<HashMap<_, _>>();
        let labels = label::Entity::find()
            .filter(label::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(label::Column::CreatedAt)
            .all(self.db.connection())
            .await?;
        let label_names = labels
            .iter()
            .map(|label| (label.id.clone(), label.name.clone()))
            .collect::<HashMap<_, _>>();
        let rules = auto_label_rule::Entity::find()
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .filter(auto_label_rule::Column::SourceSubscriptionId.is_null())
            .order_by_asc(auto_label_rule::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(|rule| {
                let label_ids = serde_json::from_str::<Vec<Uuid>>(&rule.label_ids_json)
                    .map_err(AppError::internal)?;
                Ok::<AutoLabelFeedRule, AppError>(AutoLabelFeedRule {
                    account_email: rule
                        .account_id
                        .as_ref()
                        .and_then(|id| accounts.get(id).cloned()),
                    provider_name: rule
                        .provider_id
                        .as_ref()
                        .and_then(|id| providers.get(id).cloned()),
                    name: rule.name,
                    label_names: label_ids
                        .iter()
                        .filter_map(|id| label_names.get(&id.to_string()).cloned())
                        .collect(),
                    instructions: rule.instructions,
                    enabled: rule.enabled,
                    apply_automatically: rule.apply_automatically,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(AutoLabelRuleFeed {
            format: AUTO_LABEL_FEED_FORMAT.into(),
            version: AUTO_LABEL_FEED_VERSION,
            labels: labels
                .into_iter()
                .map(|label| AutoLabelFeedLabel {
                    name: label.name,
                    color: label.color,
                    is_auto: label.is_auto,
                })
                .collect(),
            rules,
        })
    }

    pub async fn list_auto_label_subscriptions(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<AutoLabelSubscription>, AppError> {
        auto_label_subscription::Entity::find()
            .filter(auto_label_subscription::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(auto_label_subscription::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(AutoLabelSubscription::try_from)
            .collect()
    }

    pub async fn get_auto_label_subscription(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<AutoLabelSubscription, AppError> {
        AutoLabelSubscription::try_from(self.subscription_model(user_id, id).await?)
    }

    pub async fn create_auto_label_subscription(
        &self,
        user_id: Uuid,
        mut input: AutoLabelSubscriptionInput,
    ) -> Result<AutoLabelSubscription, AppError> {
        input.normalize()?;
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        auto_label_subscription::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(user_id.to_string()),
            name: Set(input.name),
            url: Set(input.url),
            enabled: Set(input.enabled),
            last_synced_at: Set(None),
            last_error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(self.db.connection())
        .await?;
        self.get_auto_label_subscription(user_id, id).await
    }

    pub async fn update_auto_label_subscription(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: AutoLabelSubscriptionInput,
    ) -> Result<AutoLabelSubscription, AppError> {
        input.normalize()?;
        let model = self.subscription_model(user_id, id).await?;
        let mut active = model.into_active_model();
        active.name = Set(input.name);
        active.url = Set(input.url);
        active.enabled = Set(input.enabled);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active.update(self.db.connection()).await?;
        self.get_auto_label_subscription(user_id, id).await
    }

    pub async fn delete_auto_label_subscription(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<(), AppError> {
        let _ = self.subscription_model(user_id, id).await?;
        let transaction = self.db.connection().begin().await?;
        auto_label_rule::Entity::delete_many()
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .filter(auto_label_rule::Column::SourceSubscriptionId.eq(id.to_string()))
            .exec(&transaction)
            .await?;
        auto_label_subscription::Entity::delete_many()
            .filter(auto_label_subscription::Column::Id.eq(id.to_string()))
            .filter(auto_label_subscription::Column::UserId.eq(user_id.to_string()))
            .exec(&transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn record_subscription_sync(
        &self,
        user_id: Uuid,
        id: Uuid,
        error: Option<String>,
    ) -> Result<AutoLabelSubscription, AppError> {
        let model = self.subscription_model(user_id, id).await?;
        let mut active = model.into_active_model();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        if error.is_none() {
            active.last_synced_at = Set(Some(now));
        }
        active.last_error = Set(error.map(|value| value.chars().take(500).collect()));
        active.updated_at = Set(now);
        active.update(self.db.connection()).await?;
        self.get_auto_label_subscription(user_id, id).await
    }

    pub async fn replace_subscription_rules(
        &self,
        user_id: Uuid,
        subscription: &AutoLabelSubscription,
        feed: AutoLabelRuleFeed,
    ) -> Result<(u32, u32, u32), AppError> {
        let transaction = self.db.connection().begin().await?;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let mut labels = label::Entity::find()
            .filter(label::Column::UserId.eq(user_id.to_string()))
            .all(&transaction)
            .await?
            .into_iter()
            .map(|label| (label.name.to_ascii_lowercase(), label))
            .collect::<HashMap<_, _>>();
        let mut labels_imported = 0u32;
        for incoming in feed.labels {
            let key = incoming.name.to_ascii_lowercase();
            if labels.contains_key(&key) {
                continue;
            }
            let model = label::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                name: Set(incoming.name),
                color: Set(incoming.color),
                is_auto: Set(incoming.is_auto),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&transaction)
            .await?;
            labels.insert(key, model);
            labels_imported += 1;
        }

        let accounts = mail_account::Entity::find()
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .all(&transaction)
            .await?
            .into_iter()
            .map(|account| (account.email.to_ascii_lowercase(), account.id))
            .collect::<HashMap<_, _>>();
        let providers = ai_provider::Entity::find()
            .filter(ai_provider::Column::UserId.eq(user_id.to_string()))
            .all(&transaction)
            .await?
            .into_iter()
            .map(|provider| (provider.name.to_ascii_lowercase(), provider.id))
            .collect::<HashMap<_, _>>();

        let mut prepared = Vec::new();
        let mut skipped = 0u32;
        for rule in feed.rules {
            let account_id = rule
                .account_email
                .as_ref()
                .and_then(|email| accounts.get(&email.to_ascii_lowercase()).cloned());
            let provider_id = rule
                .provider_name
                .as_ref()
                .and_then(|name| providers.get(&name.to_ascii_lowercase()).cloned());
            if (rule.account_email.is_some() && account_id.is_none())
                || (rule.provider_name.is_some() && provider_id.is_none())
            {
                skipped += 1;
                continue;
            }
            let label_ids = rule
                .label_names
                .iter()
                .filter_map(|name| labels.get(&name.to_ascii_lowercase()))
                .map(|label| Uuid::parse_str(&label.id).map_err(AppError::internal))
                .collect::<Result<Vec<_>, _>>()?;
            prepared.push((rule, account_id, provider_id, label_ids));
        }

        auto_label_rule::Entity::delete_many()
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .filter(auto_label_rule::Column::SourceSubscriptionId.eq(subscription.id.to_string()))
            .exec(&transaction)
            .await?;
        let rules_imported = prepared.len() as u32;
        for (rule, account_id, provider_id, label_ids) in prepared {
            auto_label_rule::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                account_id: Set(account_id),
                provider_id: Set(provider_id),
                name: Set(format!("{}: {}", subscription.name, rule.name)),
                label_ids_json: Set(serde_json::to_string(&label_ids).map_err(AppError::internal)?),
                instructions: Set(rule.instructions),
                enabled: Set(rule.enabled),
                apply_automatically: Set(rule.apply_automatically),
                source_subscription_id: Set(Some(subscription.id.to_string())),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok((labels_imported, rules_imported, skipped))
    }

    pub async fn apply_labels(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        label_ids: &[Uuid],
    ) -> Result<AutoLabelResult, AppError> {
        let message_exists = message::Entity::find()
            .filter(message::Column::Id.eq(message_id.to_string()))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .is_some();
        if !message_exists {
            return Err(AppError::NotFound);
        }
        let labels = self.labels_by_ids(user_id, label_ids).await?;
        let transaction = self.db.connection().begin().await?;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        for label in &labels {
            message_label::Entity::insert(message_label::ActiveModel {
                message_id: Set(message_id.to_string()),
                label_id: Set(label.id.to_string()),
                user_id: Set(user_id.to_string()),
                created_at: Set(now),
            })
            .on_conflict(
                OnConflict::columns([
                    message_label::Column::MessageId,
                    message_label::Column::LabelId,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(&transaction)
            .await?;
        }
        transaction.commit().await?;
        Ok(AutoLabelResult { message_id, labels })
    }

    pub async fn labels_by_ids(&self, user_id: Uuid, ids: &[Uuid]) -> Result<Vec<Label>, AppError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let labels = label::Entity::find()
            .filter(label::Column::UserId.eq(user_id.to_string()))
            .filter(label::Column::Id.is_in(ids.iter().map(ToString::to_string)))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Label::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if labels.len() != ids.len() {
            return Err(AppError::Validation("label scope is invalid".into()));
        }
        Ok(labels)
    }

    pub async fn enabled_auto_label_rules_for_account(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<Vec<AutoLabelRule>, AppError> {
        let enabled_subscriptions = auto_label_subscription::Entity::find()
            .filter(auto_label_subscription::Column::UserId.eq(user_id.to_string()))
            .filter(auto_label_subscription::Column::Enabled.eq(true))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(|subscription| subscription.id)
            .collect::<Vec<_>>();
        let source_scope = if enabled_subscriptions.is_empty() {
            sea_orm::Condition::all().add(auto_label_rule::Column::SourceSubscriptionId.is_null())
        } else {
            sea_orm::Condition::any()
                .add(auto_label_rule::Column::SourceSubscriptionId.is_null())
                .add(auto_label_rule::Column::SourceSubscriptionId.is_in(enabled_subscriptions))
        };
        auto_label_rule::Entity::find()
            .filter(auto_label_rule::Column::UserId.eq(user_id.to_string()))
            .filter(auto_label_rule::Column::Enabled.eq(true))
            .filter(auto_label_rule::Column::ApplyAutomatically.eq(true))
            .filter(source_scope)
            .filter(
                sea_orm::Condition::any()
                    .add(auto_label_rule::Column::AccountId.is_null())
                    .add(auto_label_rule::Column::AccountId.eq(account_id.to_string())),
            )
            .order_by_asc(auto_label_rule::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(AutoLabelRule::try_from)
            .collect()
    }

    async fn subscription_model(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<auto_label_subscription::Model, AppError> {
        auto_label_subscription::Entity::find()
            .filter(auto_label_subscription::Column::Id.eq(id.to_string()))
            .filter(auto_label_subscription::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn provider_model(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<ai_provider::Model, AppError> {
        ai_provider::Entity::find()
            .filter(ai_provider::Column::Id.eq(id.to_string()))
            .filter(ai_provider::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn validate_auto_label_refs(
        &self,
        user_id: Uuid,
        input: &AutoLabelRuleInput,
    ) -> Result<(), AppError> {
        let _ = self.labels_by_ids(user_id, &input.label_ids).await?;
        if let Some(provider_id) = input.provider_id {
            let _ = self.provider_model(user_id, provider_id).await?;
        }
        if let Some(account_id) = input.account_id {
            let exists = mail_account::Entity::find()
                .filter(mail_account::Column::Id.eq(account_id.to_string()))
                .filter(mail_account::Column::UserId.eq(user_id.to_string()))
                .one(self.db.connection())
                .await?
                .is_some();
            if !exists {
                return Err(AppError::Validation("mail account scope is invalid".into()));
            }
        }
        Ok(())
    }
}

async fn clear_default_provider(
    connection: &impl sea_orm::ConnectionTrait,
    user_id: Uuid,
) -> Result<(), AppError> {
    ai_provider::Entity::update_many()
        .filter(ai_provider::Column::UserId.eq(user_id.to_string()))
        .col_expr(
            ai_provider::Column::IsDefault,
            sea_orm::sea_query::Expr::value(false),
        )
        .exec(connection)
        .await?;
    Ok(())
}

impl TryFrom<ai_provider::Model> for AiProvider {
    type Error = AppError;

    fn try_from(model: ai_provider::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            name: model.name,
            provider_kind: AiProviderKind::parse(&model.provider_kind)?,
            api_type: AiApiType::parse(&model.api_type)?,
            model: model.model,
            base_url: model.base_url,
            proxy: PublicProxyConfig {
                kind: crate::accounts::ProxyKind::parse(&model.proxy_kind)?,
                host: model.proxy_host,
                port: model.proxy_port.map(|value| value as u16),
                username: model.proxy_username,
                has_password: model.proxy_password_cipher.is_some(),
            },
            is_default: model.is_default,
            enabled: model.enabled,
            has_api_key: model.api_key_cipher.is_some(),
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

impl TryFrom<label::Model> for Label {
    type Error = AppError;

    fn try_from(model: label::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            name: model.name,
            color: model.color,
            is_auto: model.is_auto,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

impl TryFrom<auto_label_rule::Model> for AutoLabelRule {
    type Error = AppError;

    fn try_from(model: auto_label_rule::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            account_id: model
                .account_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(AppError::internal)?,
            provider_id: model
                .provider_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(AppError::internal)?,
            name: model.name,
            label_ids: serde_json::from_str::<Vec<Uuid>>(&model.label_ids_json)
                .map_err(AppError::internal)?,
            instructions: model.instructions,
            enabled: model.enabled,
            apply_automatically: model.apply_automatically,
            source_subscription_id: model
                .source_subscription_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(AppError::internal)?,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

impl TryFrom<auto_label_subscription::Model> for AutoLabelSubscription {
    type Error = AppError;

    fn try_from(model: auto_label_subscription::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            name: model.name,
            url: model.url,
            enabled: model.enabled,
            last_synced_at: model.last_synced_at,
            last_error: model.last_error,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}
