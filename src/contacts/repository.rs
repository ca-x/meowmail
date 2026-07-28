use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{Database, entities::contact},
    error::AppError,
};

use super::{Contact, ContactInput};

#[derive(Clone)]
pub struct ContactRepository {
    db: Database,
}

impl ContactRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        query: Option<String>,
        limit: u64,
    ) -> Result<Vec<Contact>, AppError> {
        let mut select = contact::Entity::find()
            .filter(contact::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(contact::Column::DisplayName)
            .order_by_asc(contact::Column::Email)
            .limit(limit.clamp(1, 100));
        if let Some(query) = query
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
        {
            let pattern = format!("%{}%", query.to_ascii_lowercase());
            select = select.filter(sea_orm::sea_query::Expr::cust_with_values(
                "(LOWER(display_name) LIKE ? OR LOWER(email) LIKE ?)",
                [pattern.clone(), pattern],
            ));
        }
        select
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Contact::try_from)
            .collect()
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        mut input: ContactInput,
    ) -> Result<Contact, AppError> {
        input.normalize()?;
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        contact::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(user_id.to_string()),
            display_name: Set(input.display_name),
            email: Set(input.email),
            notes: Set(input.notes),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(self.db.connection())
        .await
        .map_err(map_contact_error)
        .and_then(Contact::try_from)
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: ContactInput,
    ) -> Result<Contact, AppError> {
        input.normalize()?;
        let model = contact::Entity::find()
            .filter(contact::Column::Id.eq(id.to_string()))
            .filter(contact::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let mut active = model.into_active_model();
        active.display_name = Set(input.display_name);
        active.email = Set(input.email);
        active.notes = Set(input.notes);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active
            .update(self.db.connection())
            .await
            .map_err(map_contact_error)
            .and_then(Contact::try_from)
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = contact::Entity::delete_many()
            .filter(contact::Column::Id.eq(id.to_string()))
            .filter(contact::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn contains_email(&self, user_id: Uuid, email: &str) -> Result<bool, AppError> {
        let email = email.trim().to_ascii_lowercase();
        if email.is_empty() {
            return Ok(false);
        }
        Ok(contact::Entity::find()
            .filter(contact::Column::UserId.eq(user_id.to_string()))
            .filter(contact::Column::Email.eq(email))
            .one(self.db.connection())
            .await?
            .is_some())
    }
}

impl TryFrom<contact::Model> for Contact {
    type Error = AppError;

    fn try_from(model: contact::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            display_name: model.display_name,
            email: model.email,
            notes: model.notes,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

fn map_contact_error(error: sea_orm::DbErr) -> AppError {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("unique") || message.contains("constraint") {
        AppError::Conflict
    } else {
        AppError::from(error)
    }
}
