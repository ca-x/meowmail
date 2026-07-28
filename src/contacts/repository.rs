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

use super::{Contact, ContactInput, model::contact_search_aliases};

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
        let limit = limit.clamp(1, 100) as usize;
        let query = query
            .map(|value| value.trim().to_lowercase())
            .filter(|value| !value.is_empty());
        let mut select = contact::Entity::find()
            .filter(contact::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(contact::Column::DisplayName)
            .order_by_asc(contact::Column::Email);
        if query.is_none() {
            select = select.limit(limit as u64);
        }
        let mut contacts = select
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Contact::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(query) = query {
            contacts.retain(|contact| contact_matches_query(contact, &query));
            contacts.truncate(limit);
        }
        Ok(contacts)
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
            search_aliases: contact_search_aliases(&model.display_name),
            display_name: model.display_name,
            email: model.email,
            notes: model.notes,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

fn contact_matches_query(contact: &Contact, query: &str) -> bool {
    contact.display_name.to_lowercase().contains(query)
        || contact.email.to_lowercase().contains(query)
        || contact.notes.to_lowercase().contains(query)
        || contact
            .search_aliases
            .iter()
            .any(|alias| alias.contains(query))
}

fn map_contact_error(error: sea_orm::DbErr) -> AppError {
    let message = error.to_string().to_ascii_lowercase();
    if message.contains("unique") || message.contains("constraint") {
        AppError::Conflict
    } else {
        AppError::from(error)
    }
}

#[cfg(test)]
mod tests {
    use super::contact_matches_query;
    use crate::contacts::Contact;
    use uuid::Uuid;

    fn contact(name: &str, aliases: &[&str]) -> Contact {
        Contact {
            id: Uuid::new_v4(),
            display_name: name.into(),
            email: "person@example.com".into(),
            notes: "Design team".into(),
            search_aliases: aliases.iter().map(|alias| (*alias).into()).collect(),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn matches_direct_text_pinyin_and_initials() {
        let chinese = contact("张三", &["zhangsan", "zhang san", "zs"]);
        assert!(contact_matches_query(&chinese, "张"));
        assert!(contact_matches_query(&chinese, "zhang"));
        assert!(contact_matches_query(&chinese, "zs"));

        let english = contact("John Smith", &["js"]);
        assert!(contact_matches_query(&english, "john"));
        assert!(contact_matches_query(&english, "js"));
        assert!(contact_matches_query(&english, "design"));
        assert!(!contact_matches_query(&english, "zz"));
    }
}
