use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set,
    sea_query::OnConflict,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{
        Database,
        entities::{mail_account, preference, signature},
    },
    error::AppError,
};

use super::{MailPreferences, Signature, SignatureInput};

const MAIL_PREFERENCES_KEY: &str = "mail.preferences.v1";

#[derive(Clone)]
pub struct PreferencesRepository {
    db: Database,
}

impl PreferencesRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn mail(&self, user_id: Uuid) -> Result<MailPreferences, AppError> {
        let value = preference::Entity::find()
            .filter(preference::Column::UserId.eq(user_id.to_string()))
            .filter(preference::Column::Key.eq(MAIL_PREFERENCES_KEY))
            .one(self.db.connection())
            .await?
            .map(|model| model.value);
        let mut preferences: MailPreferences = value
            .map(|value| serde_json::from_str(&value).map_err(AppError::internal))
            .transpose()?
            .unwrap_or_default();
        preferences.normalize()?;
        Ok(preferences)
    }

    pub async fn update_mail(
        &self,
        user_id: Uuid,
        mut preferences: MailPreferences,
    ) -> Result<MailPreferences, AppError> {
        preferences.normalize()?;
        let value = serde_json::to_string(&preferences).map_err(AppError::internal)?;
        preference::Entity::insert(preference::ActiveModel {
            user_id: Set(user_id.to_string()),
            key: Set(MAIL_PREFERENCES_KEY.into()),
            value: Set(value),
        })
        .on_conflict(
            OnConflict::columns([preference::Column::UserId, preference::Column::Key])
                .update_column(preference::Column::Value)
                .to_owned(),
        )
        .exec(self.db.connection())
        .await?;
        Ok(preferences)
    }

    pub async fn list_signatures(&self, user_id: Uuid) -> Result<Vec<Signature>, AppError> {
        signature::Entity::find()
            .filter(signature::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(signature::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Signature::try_from)
            .collect()
    }

    pub async fn create_signature(
        &self,
        user_id: Uuid,
        mut input: SignatureInput,
    ) -> Result<Signature, AppError> {
        input.normalize()?;
        self.ensure_signature_name(user_id, &input.name, None)
            .await?;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        Signature::try_from(
            signature::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                name: Set(input.name),
                body_text: Set(input.body_text),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(self.db.connection())
            .await?,
        )
    }

    pub async fn update_signature(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: SignatureInput,
    ) -> Result<Signature, AppError> {
        input.normalize()?;
        self.ensure_signature_name(user_id, &input.name, Some(id))
            .await?;
        let model = self.signature_model(user_id, id).await?;
        let mut active = model.into_active_model();
        active.name = Set(input.name);
        active.body_text = Set(input.body_text);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        Signature::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn delete_signature(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = signature::Entity::delete_many()
            .filter(signature::Column::Id.eq(id.to_string()))
            .filter(signature::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        mail_account::Entity::update_many()
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .filter(mail_account::Column::SignatureId.eq(id.to_string()))
            .col_expr(
                mail_account::Column::SignatureId,
                sea_orm::sea_query::Expr::value(Option::<String>::None),
            )
            .exec(self.db.connection())
            .await?;
        Ok(())
    }

    pub async fn signature_text(
        &self,
        user_id: Uuid,
        signature_id: Option<&str>,
    ) -> Result<Option<String>, AppError> {
        let Some(signature_id) = signature_id else {
            return Ok(None);
        };
        Ok(signature::Entity::find()
            .filter(signature::Column::Id.eq(signature_id))
            .filter(signature::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .map(|model| model.body_text))
    }

    async fn signature_model(&self, user_id: Uuid, id: Uuid) -> Result<signature::Model, AppError> {
        signature::Entity::find()
            .filter(signature::Column::Id.eq(id.to_string()))
            .filter(signature::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn ensure_signature_name(
        &self,
        user_id: Uuid,
        name: &str,
        except: Option<Uuid>,
    ) -> Result<(), AppError> {
        if self
            .list_signatures(user_id)
            .await?
            .into_iter()
            .any(|signature| {
                signature.name.eq_ignore_ascii_case(name) && except != Some(signature.id)
            })
        {
            return Err(AppError::Conflict);
        }
        Ok(())
    }
}

impl TryFrom<signature::Model> for Signature {
    type Error = AppError;

    fn try_from(model: signature::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            name: model.name,
            body_text: model.body_text,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}
