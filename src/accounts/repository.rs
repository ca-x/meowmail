use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use secrecy::SecretString;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{Database, entities::mail_account},
    error::AppError,
    security::CredentialVault,
};

use super::{
    AccountInput, AccountSecrets, ConnectionSecurity, MailAccount, ProxyConfig, ProxyKind,
    PublicProxyConfig, ServerConfig,
};

#[derive(Clone)]
pub struct AccountRepository {
    db: Database,
    vault: CredentialVault,
}

impl AccountRepository {
    pub fn new(db: Database, vault: CredentialVault) -> Self {
        Self { db, vault }
    }

    pub async fn list(&self, user_id: Uuid) -> Result<Vec<MailAccount>, AppError> {
        mail_account::Entity::find()
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .order_by_desc(mail_account::Column::IsDefault)
            .order_by_asc(mail_account::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(MailAccount::try_from)
            .collect()
    }

    pub async fn get(&self, user_id: Uuid, id: Uuid) -> Result<MailAccount, AppError> {
        MailAccount::try_from(self.get_model(user_id, id).await?)
    }

    pub async fn get_with_secrets(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<(MailAccount, AccountSecrets, ProxyConfig), AppError> {
        let model = self.get_model(user_id, id).await?;
        let secrets = AccountSecrets {
            password: self
                .vault
                .open(&model.password_cipher)
                .map_err(AppError::internal)?,
            proxy_password: model
                .proxy_password_cipher
                .as_deref()
                .map(|value| self.vault.open(value).map_err(AppError::internal))
                .transpose()?,
        };
        let proxy = proxy_config(&model, secrets.proxy_password.clone())?;
        Ok((MailAccount::try_from(model)?, secrets, proxy))
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        mut input: AccountInput,
    ) -> Result<MailAccount, AppError> {
        input.validate(true)?;
        self.ensure_email_available(user_id, &input.email, None)
            .await?;
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let password_cipher = self
            .vault
            .seal(
                input
                    .password
                    .as_deref()
                    .expect("validated password exists"),
            )
            .map_err(AppError::internal)?;
        let proxy_password_cipher = seal_optional(&self.vault, input.proxy.password.as_deref())?;
        let transaction = self.db.connection().begin().await?;
        let count = mail_account::Entity::find()
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .count(&transaction)
            .await?;
        let is_default = input.is_default || count == 0;
        if is_default {
            clear_default(&transaction, user_id).await?;
        }
        mail_account::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(Some(user_id.to_string())),
            display_name: Set(input.display_name),
            email: Set(input.email),
            username: Set(input.username),
            password_cipher: Set(password_cipher),
            imap_host: Set(input.imap.host),
            imap_port: Set(i32::from(input.imap.port)),
            imap_security: Set(input.imap.security.as_str().to_owned()),
            smtp_host: Set(input.smtp.host),
            smtp_port: Set(i32::from(input.smtp.port)),
            smtp_security: Set(input.smtp.security.as_str().to_owned()),
            proxy_kind: Set(input.proxy.kind.as_str().to_owned()),
            proxy_host: Set(input.proxy.host),
            proxy_port: Set(input.proxy.port.map(i32::from)),
            proxy_username: Set(input.proxy.username),
            proxy_password_cipher: Set(proxy_password_cipher),
            is_default: Set(is_default),
            last_synced_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&transaction)
        .await?;
        transaction.commit().await?;
        self.get(user_id, id).await
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: AccountInput,
    ) -> Result<MailAccount, AppError> {
        input.validate(false)?;
        self.ensure_email_available(user_id, &input.email, Some(id))
            .await?;
        let existing = self.get_model(user_id, id).await?;
        let password_cipher = match input.password.as_deref() {
            Some(password) => self.vault.seal(password).map_err(AppError::internal)?,
            None => existing.password_cipher.clone(),
        };
        let proxy_password_cipher = if input.proxy.kind == ProxyKind::Direct {
            None
        } else {
            match input.proxy.password.as_deref() {
                Some(password) => Some(self.vault.seal(password).map_err(AppError::internal)?),
                None => existing.proxy_password_cipher.clone(),
            }
        };
        let transaction = self.db.connection().begin().await?;
        if input.is_default {
            clear_default(&transaction, user_id).await?;
        }
        let mut active = existing.clone().into_active_model();
        active.display_name = Set(input.display_name);
        active.email = Set(input.email);
        active.username = Set(input.username);
        active.password_cipher = Set(password_cipher);
        active.imap_host = Set(input.imap.host);
        active.imap_port = Set(i32::from(input.imap.port));
        active.imap_security = Set(input.imap.security.as_str().to_owned());
        active.smtp_host = Set(input.smtp.host);
        active.smtp_port = Set(i32::from(input.smtp.port));
        active.smtp_security = Set(input.smtp.security.as_str().to_owned());
        active.proxy_kind = Set(input.proxy.kind.as_str().to_owned());
        active.proxy_host = Set(input.proxy.host);
        active.proxy_port = Set(input.proxy.port.map(i32::from));
        active.proxy_username = Set(input.proxy.username);
        active.proxy_password_cipher = Set(proxy_password_cipher);
        active.is_default = Set(input.is_default);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active.update(&transaction).await?;
        if !input.is_default && existing.is_default {
            ensure_default(&transaction, user_id).await?;
        }
        transaction.commit().await?;
        self.get(user_id, id).await
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let transaction = self.db.connection().begin().await?;
        let result = mail_account::Entity::delete_many()
            .filter(mail_account::Column::Id.eq(id.to_string()))
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .exec(&transaction)
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        ensure_default(&transaction, user_id).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn mark_synced(
        &self,
        user_id: Uuid,
        id: Uuid,
        timestamp: i64,
    ) -> Result<(), AppError> {
        let model = self.get_model(user_id, id).await?;
        let mut active = model.into_active_model();
        active.last_synced_at = Set(Some(timestamp));
        active.updated_at = Set(timestamp);
        active.update(self.db.connection()).await?;
        Ok(())
    }

    async fn get_model(&self, user_id: Uuid, id: Uuid) -> Result<mail_account::Model, AppError> {
        mail_account::Entity::find()
            .filter(mail_account::Column::Id.eq(id.to_string()))
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn ensure_email_available(
        &self,
        user_id: Uuid,
        email: &str,
        except: Option<Uuid>,
    ) -> Result<(), AppError> {
        let existing = mail_account::Entity::find()
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .filter(mail_account::Column::Email.eq(email))
            .one(self.db.connection())
            .await?;
        if existing.is_some_and(|model| except.is_none_or(|except| model.id != except.to_string()))
        {
            return Err(AppError::Conflict);
        }
        Ok(())
    }
}

impl TryFrom<mail_account::Model> for MailAccount {
    type Error = AppError;

    fn try_from(model: mail_account::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            display_name: model.display_name,
            email: model.email,
            username: model.username,
            imap: ServerConfig {
                host: model.imap_host,
                port: model.imap_port as u16,
                security: ConnectionSecurity::parse(&model.imap_security)?,
            },
            smtp: ServerConfig {
                host: model.smtp_host,
                port: model.smtp_port as u16,
                security: ConnectionSecurity::parse(&model.smtp_security)?,
            },
            proxy: PublicProxyConfig {
                kind: ProxyKind::parse(&model.proxy_kind)?,
                host: model.proxy_host,
                port: model.proxy_port.map(|value| value as u16),
                username: model.proxy_username,
                has_password: model.proxy_password_cipher.is_some(),
            },
            is_default: model.is_default,
            last_synced_at: model.last_synced_at,
            created_at: model.created_at,
            updated_at: model.updated_at,
            has_password: true,
        })
    }
}

fn proxy_config(
    model: &mail_account::Model,
    password: Option<SecretString>,
) -> Result<ProxyConfig, AppError> {
    Ok(ProxyConfig {
        kind: ProxyKind::parse(&model.proxy_kind)?,
        host: model.proxy_host.clone(),
        port: model.proxy_port.map(|value| value as u16),
        username: model.proxy_username.clone(),
        password,
    })
}

fn seal_optional(vault: &CredentialVault, value: Option<&str>) -> Result<Option<String>, AppError> {
    value
        .filter(|value| !value.is_empty())
        .map(|value| vault.seal(value).map_err(AppError::internal))
        .transpose()
}

async fn clear_default(
    connection: &impl sea_orm::ConnectionTrait,
    user_id: Uuid,
) -> Result<(), AppError> {
    mail_account::Entity::update_many()
        .filter(mail_account::Column::UserId.eq(user_id.to_string()))
        .col_expr(
            mail_account::Column::IsDefault,
            sea_orm::sea_query::Expr::value(false),
        )
        .exec(connection)
        .await?;
    Ok(())
}

async fn ensure_default(
    connection: &impl sea_orm::ConnectionTrait,
    user_id: Uuid,
) -> Result<(), AppError> {
    if mail_account::Entity::find()
        .filter(mail_account::Column::UserId.eq(user_id.to_string()))
        .filter(mail_account::Column::IsDefault.eq(true))
        .count(connection)
        .await?
        == 0
        && let Some(model) = mail_account::Entity::find()
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(mail_account::Column::CreatedAt)
            .one(connection)
            .await?
    {
        let mut active = model.into_active_model();
        active.is_default = Set(true);
        active.update(connection).await?;
    }
    Ok(())
}
