use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, ExprTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, Set, Statement, TransactionTrait,
    sea_query::Expr,
};
use secrecy::ExposeSecret;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    config::BootstrapAdmin,
    db::{
        Database,
        entities::{mail_setting, notification_setting, user, user_identity},
    },
    error::AppError,
    security::{hash_secret, verify_secret},
};

use super::{PublicUser, Role};

#[derive(Clone)]
pub struct UserRepository {
    db: Database,
}

impl UserRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn bootstrap(&self, admin: Option<&BootstrapAdmin>) -> anyhow::Result<()> {
        let Some(admin) = admin else { return Ok(()) };
        let username = normalize_username(&admin.username)?;
        if user::Entity::find()
            .one(self.db.connection())
            .await?
            .is_some()
        {
            return Ok(());
        }
        let password_hash = hash_secret(admin.password.expose_secret())?;
        let transaction = self.db.connection().begin().await?;
        if user::Entity::find().one(&transaction).await?.is_some() {
            transaction.rollback().await?;
            return Ok(());
        }
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        user::ActiveModel {
            id: Set(id.to_string()),
            username: Set(username.clone()),
            nickname: Set(admin.username.clone()),
            email: Set(None),
            role: Set(Role::Admin.as_str().into()),
            password_hash: Set(Some(password_hash)),
            pin_hash: Set(None),
            avatar_mime: Set(None),
            avatar_data: Set(None),
            ai_enabled: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            last_login_at: Set(None),
        }
        .insert(&transaction)
        .await?;
        claim_first_admin(&transaction, id).await?;
        ensure_user_defaults(&transaction, id).await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn has_local_user(&self) -> Result<bool, AppError> {
        Ok(user::Entity::find()
            .filter(user::Column::PasswordHash.is_not_null())
            .count(self.db.connection())
            .await?
            > 0)
    }

    pub async fn authenticate_local(
        &self,
        username: &str,
        password: &str,
    ) -> Result<PublicUser, AppError> {
        let username = normalize_username(username).map_err(|_| AppError::Unauthorized)?;
        let model = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::Unauthorized)?;
        let hash = model
            .password_hash
            .as_deref()
            .ok_or(AppError::Unauthorized)?;
        if !verify_secret(hash, password) {
            return Err(AppError::Unauthorized);
        }
        self.record_login(model).await
    }

    pub async fn provision_oidc(
        &self,
        issuer: &str,
        subject: &str,
        email: Option<&str>,
        preferred_username: Option<&str>,
        first_user_admin: bool,
    ) -> Result<PublicUser, AppError> {
        if issuer.len() > 2048 || subject.is_empty() || subject.len() > 1024 {
            return Err(AppError::Unauthorized);
        }
        if let Some(identity) = user_identity::Entity::find()
            .filter(user_identity::Column::Issuer.eq(issuer))
            .filter(user_identity::Column::Subject.eq(subject))
            .one(self.db.connection())
            .await?
        {
            let model = user::Entity::find_by_id(identity.user_id)
                .one(self.db.connection())
                .await?
                .ok_or_else(|| AppError::internal(anyhow::anyhow!("OIDC user is missing")))?;
            return self.record_login(model).await;
        }

        let transaction = self.db.connection().begin().await?;
        if let Some(identity) = user_identity::Entity::find()
            .filter(user_identity::Column::Issuer.eq(issuer))
            .filter(user_identity::Column::Subject.eq(subject))
            .one(&transaction)
            .await?
        {
            let model = user::Entity::find_by_id(identity.user_id)
                .one(&transaction)
                .await?
                .ok_or_else(|| AppError::internal(anyhow::anyhow!("OIDC user is missing")))?;
            transaction.commit().await?;
            return self.record_login(model).await;
        }

        let id = Uuid::new_v4();
        let role = if first_user_admin && try_claim_first_admin(&transaction, id).await? {
            Role::Admin
        } else {
            Role::User
        };
        let username = unique_username(
            &transaction,
            preferred_username.or(email).unwrap_or("oidc-user"),
            id,
        )
        .await?;
        let nickname = clean_nickname(
            preferred_username
                .or(email.and_then(|value| value.split('@').next()))
                .unwrap_or(&username),
        )?;
        let email = email
            .map(str::trim)
            .filter(|value| !value.is_empty() && value.len() <= 254)
            .map(|value| value.to_ascii_lowercase());
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let model = user::ActiveModel {
            id: Set(id.to_string()),
            username: Set(username),
            nickname: Set(nickname),
            email: Set(email),
            role: Set(role.as_str().into()),
            password_hash: Set(None),
            pin_hash: Set(None),
            avatar_mime: Set(None),
            avatar_data: Set(None),
            ai_enabled: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            last_login_at: Set(Some(now)),
        }
        .insert(&transaction)
        .await?;
        user_identity::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            user_id: Set(id.to_string()),
            issuer: Set(issuer.to_owned()),
            subject: Set(subject.to_owned()),
            created_at: Set(now),
            last_login_at: Set(now),
        }
        .insert(&transaction)
        .await?;
        ensure_user_defaults(&transaction, id).await?;
        transaction.commit().await?;
        PublicUser::try_from(model)
    }

    pub async fn get(&self, id: Uuid) -> Result<PublicUser, AppError> {
        let model = self.get_model(id).await?;
        PublicUser::try_from(model)
    }

    pub async fn get_model(&self, id: Uuid) -> Result<user::Model, AppError> {
        user::Entity::find_by_id(id.to_string())
            .one(self.db.connection())
            .await?
            .ok_or(AppError::Unauthorized)
    }

    pub async fn update_profile(
        &self,
        id: Uuid,
        username: Option<&str>,
        nickname: &str,
    ) -> Result<PublicUser, AppError> {
        let nickname = clean_nickname(nickname)?;
        let transaction = self.db.connection().begin().await?;
        let model = user::Entity::find_by_id(id.to_string())
            .one(&transaction)
            .await?
            .ok_or(AppError::Unauthorized)?;
        let username = match username {
            Some(username) if username.trim().eq_ignore_ascii_case(&model.username) => {
                model.username.clone()
            }
            Some(username) => normalize_editable_username(username)?,
            None => model.username.clone(),
        };
        let conflict = user::Entity::find()
            .filter(user::Column::Username.eq(&username))
            .filter(user::Column::Id.ne(id.to_string()))
            .one(&transaction)
            .await?
            .is_some();
        if conflict {
            transaction.rollback().await?;
            return Err(AppError::Conflict);
        }
        let mut active = model.into_active_model();
        active.username = Set(username);
        active.nickname = Set(nickname);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        let updated = active.update(&transaction).await?;
        transaction.commit().await?;
        PublicUser::try_from(updated)
    }

    pub async fn update_password(
        &self,
        id: Uuid,
        current_password: Option<&str>,
        new_password: &str,
    ) -> Result<PublicUser, AppError> {
        validate_login_password(new_password)?;
        let model = self.get_model(id).await?;
        let previous_hash = model.password_hash.clone();
        if let Some(hash) = previous_hash.as_deref() {
            let supplied = current_password.ok_or(AppError::Unauthorized)?;
            let hash = hash.to_owned();
            let supplied = supplied.to_owned();
            let verified = tokio::task::spawn_blocking(move || verify_secret(&hash, &supplied))
                .await
                .map_err(AppError::internal)?;
            if !verified {
                return Err(AppError::Unauthorized);
            }
        }
        let new_password = new_password.to_owned();
        let new_hash = tokio::task::spawn_blocking(move || hash_secret(&new_password))
            .await
            .map_err(AppError::internal)?
            .map_err(AppError::internal)?;
        let mut update = user::Entity::update_many()
            .col_expr(user::Column::PasswordHash, Expr::value(Some(new_hash)))
            .col_expr(
                user::Column::UpdatedAt,
                Expr::value(OffsetDateTime::now_utc().unix_timestamp()),
            )
            .filter(user::Column::Id.eq(id.to_string()));
        update = match previous_hash {
            Some(hash) => update.filter(user::Column::PasswordHash.eq(hash)),
            None => update.filter(Expr::col(user::Column::PasswordHash).is_null()),
        };
        if update.exec(self.db.connection()).await?.rows_affected != 1 {
            return Err(AppError::Conflict);
        }
        self.get(id).await
    }

    pub async fn set_avatar(
        &self,
        id: Uuid,
        mime: Option<String>,
        data: Option<Vec<u8>>,
    ) -> Result<PublicUser, AppError> {
        let mut active = self.get_model(id).await?.into_active_model();
        active.avatar_mime = Set(mime);
        active.avatar_data = Set(data);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        PublicUser::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn set_pin(&self, id: Uuid, pin: Option<&str>) -> Result<PublicUser, AppError> {
        let pin_hash = pin
            .map(hash_secret)
            .transpose()
            .map_err(AppError::internal)?;
        let mut active = self.get_model(id).await?.into_active_model();
        active.pin_hash = Set(pin_hash);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        PublicUser::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn set_ai_enabled(&self, id: Uuid, enabled: bool) -> Result<PublicUser, AppError> {
        let mut active = self.get_model(id).await?.into_active_model();
        active.ai_enabled = Set(enabled);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        PublicUser::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn verify_pin(&self, id: Uuid, supplied: &str) -> Result<bool, AppError> {
        Ok(self
            .get_model(id)
            .await?
            .pin_hash
            .as_deref()
            .is_some_and(|hash| verify_secret(hash, supplied)))
    }

    async fn record_login(&self, model: user::Model) -> Result<PublicUser, AppError> {
        let mut active = model.into_active_model();
        active.last_login_at = Set(Some(OffsetDateTime::now_utc().unix_timestamp()));
        PublicUser::try_from(active.update(self.db.connection()).await?)
    }
}

impl TryFrom<user::Model> for PublicUser {
    type Error = AppError;

    fn try_from(model: user::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            username: model.username,
            nickname: model.nickname,
            email: model.email,
            role: Role::parse(&model.role)?,
            has_password: model.password_hash.is_some(),
            has_pin: model.pin_hash.is_some(),
            has_avatar: model.avatar_data.is_some(),
            ai_enabled: model.ai_enabled,
            updated_at: model.updated_at,
        })
    }
}

pub fn clean_nickname(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 80 || value.chars().any(char::is_control) {
        return Err(AppError::Validation("nickname is invalid".into()));
    }
    Ok(value.to_owned())
}

fn normalize_username(value: &str) -> anyhow::Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 128 || value.chars().any(char::is_control) {
        anyhow::bail!("username is invalid");
    }
    Ok(value.to_ascii_lowercase())
}

fn normalize_editable_username(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    let valid = (2..=64).contains(&value.chars().count())
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-' | '@')
        })
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && value
            .chars()
            .last()
            .is_some_and(|character| character.is_ascii_alphanumeric());
    if !valid {
        return Err(AppError::Validation("username is invalid".into()));
    }
    Ok(value.to_ascii_lowercase())
}

fn validate_login_password(password: &str) -> Result<(), AppError> {
    if password.chars().count() < 8
        || password.len() > 4096
        || password.chars().any(char::is_control)
    {
        return Err(AppError::Validation(
            "password must contain at least 8 non-control characters and no more than 4096 UTF-8 bytes"
                .into(),
        ));
    }
    Ok(())
}

async fn unique_username(
    connection: &impl ConnectionTrait,
    candidate: &str,
    id: Uuid,
) -> Result<String, AppError> {
    let base = candidate
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
        })
        .take(96)
        .collect::<String>();
    let base = if base.is_empty() { "oidc-user" } else { &base };
    for suffix in 0..100_u8 {
        let value = if suffix == 0 {
            base.to_owned()
        } else {
            format!("{base}-{}", &id.simple().to_string()[..8])
        };
        if user::Entity::find()
            .filter(user::Column::Username.eq(&value))
            .one(connection)
            .await?
            .is_none()
        {
            return Ok(value);
        }
    }
    Err(AppError::Conflict)
}

async fn try_claim_first_admin(
    transaction: &DatabaseTransaction,
    user_id: Uuid,
) -> Result<bool, AppError> {
    if user::Entity::find()
        .filter(user::Column::Role.eq(Role::Admin.as_str()))
        .count(transaction)
        .await?
        > 0
    {
        return Ok(false);
    }
    let result = transaction
        .execute_raw(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "INSERT OR IGNORE INTO system_state(key, value) VALUES('first_admin_user_id', ?)",
            [user_id.to_string().into()],
        ))
        .await?;
    Ok(result.rows_affected() == 1)
}

async fn claim_first_admin(
    transaction: &DatabaseTransaction,
    user_id: Uuid,
) -> Result<(), AppError> {
    transaction
        .execute_raw(Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "INSERT OR IGNORE INTO system_state(key, value) VALUES('first_admin_user_id', ?)",
            [user_id.to_string().into()],
        ))
        .await?;
    Ok(())
}

async fn ensure_user_defaults(
    transaction: &DatabaseTransaction,
    user_id: Uuid,
) -> Result<(), AppError> {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if notification_setting::Entity::find_by_id(user_id.to_string())
        .one(transaction)
        .await?
        .is_none()
    {
        notification_setting::ActiveModel {
            user_id: Set(user_id.to_string()),
            enabled: Set(false),
            message_template: Set("[{account}] {sender}: {subject}".into()),
            command_template: Set(None),
            http_url: Set(None),
            updated_at: Set(now),
        }
        .insert(transaction)
        .await?;
    }
    if mail_setting::Entity::find_by_id(user_id.to_string())
        .one(transaction)
        .await?
        .is_none()
    {
        mail_setting::ActiveModel {
            user_id: Set(user_id.to_string()),
            keep_local_after_server_delete: Set(true),
            sync_fetch_limit: Set(Some(50)),
            updated_at: Set(now),
        }
        .insert(transaction)
        .await?;
    }
    Ok(())
}

pub fn validate_pin(pin: &str) -> Result<(), AppError> {
    if !(4..=128).contains(&pin.chars().count()) || pin.chars().any(char::is_control) {
        return Err(AppError::Validation(
            "PIN must contain 4-128 non-control characters".into(),
        ));
    }
    Ok(())
}
