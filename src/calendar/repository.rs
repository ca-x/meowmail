use fast_dav_rs::CalendarInfo;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set, sea_query::OnConflict,
};
use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{
        Database,
        entities::{calendar, calendar_account, calendar_event, local_calendar_event, preference},
    },
    error::AppError,
    security::CredentialVault,
};

use super::model::{
    Calendar, CalendarAccount, CalendarAccountInput, CalendarEvent, CalendarPreferences,
    CalendarUpdate, LocalCalendarEvent, LocalCalendarEventInput, ParsedEvent,
};

const CALENDAR_PREFERENCES_KEY: &str = "calendar.preferences.v1";

#[derive(Clone)]
pub struct CalendarRepository {
    db: Database,
    vault: CredentialVault,
}

pub struct CalendarAccountSecrets {
    pub password: SecretString,
}

impl CalendarRepository {
    pub fn new(db: Database, vault: CredentialVault) -> Self {
        Self { db, vault }
    }

    pub async fn preferences(&self, user_id: Uuid) -> Result<CalendarPreferences, AppError> {
        let value = preference::Entity::find()
            .filter(preference::Column::UserId.eq(user_id.to_string()))
            .filter(preference::Column::Key.eq(CALENDAR_PREFERENCES_KEY))
            .one(self.db.connection())
            .await?
            .map(|model| model.value);
        let mut preferences: CalendarPreferences = value
            .map(|value| serde_json::from_str(&value).map_err(AppError::internal))
            .transpose()?
            .unwrap_or_default();
        preferences.normalize();
        Ok(preferences)
    }

    pub async fn update_preferences(
        &self,
        user_id: Uuid,
        mut preferences: CalendarPreferences,
    ) -> Result<CalendarPreferences, AppError> {
        preferences.normalize();
        let value = serde_json::to_string(&preferences).map_err(AppError::internal)?;
        preference::Entity::insert(preference::ActiveModel {
            user_id: Set(user_id.to_string()),
            key: Set(CALENDAR_PREFERENCES_KEY.into()),
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

    pub async fn list_accounts(&self, user_id: Uuid) -> Result<Vec<CalendarAccount>, AppError> {
        calendar_account::Entity::find()
            .filter(calendar_account::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(calendar_account::Column::CreatedAt)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(CalendarAccount::try_from)
            .collect()
    }

    pub async fn get_account(&self, user_id: Uuid, id: Uuid) -> Result<CalendarAccount, AppError> {
        CalendarAccount::try_from(self.account_model(user_id, id).await?)
    }

    pub async fn get_account_with_secrets(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<(calendar_account::Model, CalendarAccountSecrets), AppError> {
        let model = self.account_model(user_id, id).await?;
        let secrets = CalendarAccountSecrets {
            password: self
                .vault
                .open(&model.password_cipher)
                .map_err(AppError::internal)?,
        };
        Ok((model, secrets))
    }

    pub async fn create_account(
        &self,
        user_id: Uuid,
        mut input: CalendarAccountInput,
    ) -> Result<CalendarAccount, AppError> {
        input.normalize(true)?;
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
        CalendarAccount::try_from(
            calendar_account::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                name: Set(input.name),
                base_url: Set(input.base_url),
                username: Set(input.username),
                password_cipher: Set(password_cipher),
                enabled: Set(input.enabled),
                last_synced_at: Set(None),
                last_error: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(self.db.connection())
            .await?,
        )
    }

    pub async fn update_account(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: CalendarAccountInput,
    ) -> Result<CalendarAccount, AppError> {
        input.normalize(false)?;
        let existing = self.account_model(user_id, id).await?;
        let password_cipher = match input.password.as_deref() {
            Some(password) => self.vault.seal(password).map_err(AppError::internal)?,
            None => existing.password_cipher.clone(),
        };
        let mut active = existing.into_active_model();
        active.name = Set(input.name);
        active.base_url = Set(input.base_url);
        active.username = Set(input.username);
        active.password_cipher = Set(password_cipher);
        active.enabled = Set(input.enabled);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        CalendarAccount::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn delete_account(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = calendar_account::Entity::delete_many()
            .filter(calendar_account::Column::Id.eq(id.to_string()))
            .filter(calendar_account::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn list_calendars(&self, user_id: Uuid) -> Result<Vec<Calendar>, AppError> {
        calendar::Entity::find()
            .filter(calendar::Column::UserId.eq(user_id.to_string()))
            .order_by_asc(calendar::Column::DisplayName)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(Calendar::try_from)
            .collect()
    }

    pub async fn update_calendar(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: CalendarUpdate,
    ) -> Result<Calendar, AppError> {
        input.normalize()?;
        let model = calendar::Entity::find()
            .filter(calendar::Column::Id.eq(id.to_string()))
            .filter(calendar::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let mut active = model.into_active_model();
        active.display_name = Set(input.display_name);
        active.color = Set(input.color);
        active.enabled = Set(input.enabled);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        Calendar::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn upsert_remote_calendars(
        &self,
        user_id: Uuid,
        account_id: Uuid,
        calendars: Vec<CalendarInfo>,
    ) -> Result<Vec<Calendar>, AppError> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        for item in calendars {
            let name = item
                .displayname
                .clone()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| item.href.clone());
            let color = item.color.unwrap_or_else(|| "var(--accent)".into());
            calendar::Entity::insert(calendar::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                account_id: Set(account_id.to_string()),
                display_name: Set(name),
                color: Set(color),
                remote_href: Set(item.href),
                sync_token: Set(item.sync_token),
                enabled: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .on_conflict(
                OnConflict::columns([calendar::Column::AccountId, calendar::Column::RemoteHref])
                    .update_columns([
                        calendar::Column::DisplayName,
                        calendar::Column::Color,
                        calendar::Column::SyncToken,
                        calendar::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.db.connection())
            .await?;
        }
        self.list_calendars(user_id).await
    }

    pub async fn enabled_calendars_for_account(
        &self,
        user_id: Uuid,
        account_id: Uuid,
    ) -> Result<Vec<calendar::Model>, AppError> {
        calendar::Entity::find()
            .filter(calendar::Column::UserId.eq(user_id.to_string()))
            .filter(calendar::Column::AccountId.eq(account_id.to_string()))
            .filter(calendar::Column::Enabled.eq(true))
            .all(self.db.connection())
            .await
            .map_err(AppError::from)
    }

    pub async fn upsert_event(
        &self,
        user_id: Uuid,
        calendar_id: Uuid,
        remote_href: Option<String>,
        etag: Option<String>,
        ics: String,
        parsed: ParsedEvent,
    ) -> Result<(), AppError> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        calendar_event::Entity::insert(calendar_event::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            user_id: Set(user_id.to_string()),
            calendar_id: Set(calendar_id.to_string()),
            uid: Set(parsed.uid),
            summary: Set(parsed.summary),
            description: Set(parsed.description),
            location: Set(parsed.location),
            starts_at: Set(parsed.starts_at),
            ends_at: Set(parsed.ends_at),
            all_day: Set(parsed.all_day),
            timezone: Set(parsed.timezone),
            remote_href: Set(remote_href),
            etag: Set(etag),
            ics: Set(ics),
            deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .on_conflict(
            OnConflict::columns([
                calendar_event::Column::CalendarId,
                calendar_event::Column::Uid,
            ])
            .update_columns([
                calendar_event::Column::Summary,
                calendar_event::Column::Description,
                calendar_event::Column::Location,
                calendar_event::Column::StartsAt,
                calendar_event::Column::EndsAt,
                calendar_event::Column::AllDay,
                calendar_event::Column::Timezone,
                calendar_event::Column::RemoteHref,
                calendar_event::Column::Etag,
                calendar_event::Column::Ics,
                calendar_event::Column::Deleted,
                calendar_event::Column::UpdatedAt,
            ])
            .to_owned(),
        )
        .exec(self.db.connection())
        .await?;
        Ok(())
    }

    pub async fn list_events(
        &self,
        user_id: Uuid,
        start: i64,
        end: i64,
    ) -> Result<Vec<CalendarEvent>, AppError> {
        calendar_event::Entity::find()
            .filter(calendar_event::Column::UserId.eq(user_id.to_string()))
            .filter(calendar_event::Column::Deleted.eq(false))
            .filter(calendar_event::Column::StartsAt.lt(end))
            .filter(calendar_event::Column::EndsAt.gt(start))
            .order_by_asc(calendar_event::Column::StartsAt)
            .limit(500)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(CalendarEvent::try_from)
            .collect()
    }

    pub async fn list_local_events(
        &self,
        user_id: Uuid,
        start: i64,
        end: i64,
    ) -> Result<Vec<LocalCalendarEvent>, AppError> {
        local_calendar_event::Entity::find()
            .filter(local_calendar_event::Column::UserId.eq(user_id.to_string()))
            .filter(local_calendar_event::Column::StartsAt.lt(end))
            .filter(local_calendar_event::Column::EndsAt.gt(start))
            .order_by_asc(local_calendar_event::Column::StartsAt)
            .limit(500)
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(LocalCalendarEvent::try_from)
            .collect()
    }

    pub async fn create_local_event(
        &self,
        user_id: Uuid,
        mut input: LocalCalendarEventInput,
    ) -> Result<LocalCalendarEvent, AppError> {
        input.normalize()?;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        LocalCalendarEvent::try_from(
            local_calendar_event::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                user_id: Set(user_id.to_string()),
                summary: Set(input.summary),
                description: Set(input.description),
                location: Set(input.location),
                starts_at: Set(input.starts_at),
                ends_at: Set(input.ends_at),
                all_day: Set(input.all_day),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(self.db.connection())
            .await?,
        )
    }

    pub async fn update_local_event(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut input: LocalCalendarEventInput,
    ) -> Result<LocalCalendarEvent, AppError> {
        input.normalize()?;
        let model = self.local_event_model(user_id, id).await?;
        let mut active = model.into_active_model();
        active.summary = Set(input.summary);
        active.description = Set(input.description);
        active.location = Set(input.location);
        active.starts_at = Set(input.starts_at);
        active.ends_at = Set(input.ends_at);
        active.all_day = Set(input.all_day);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        LocalCalendarEvent::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn delete_local_event(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = local_calendar_event::Entity::delete_many()
            .filter(local_calendar_event::Column::Id.eq(id.to_string()))
            .filter(local_calendar_event::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn mark_account_synced(
        &self,
        user_id: Uuid,
        id: Uuid,
        error: Option<String>,
    ) -> Result<(), AppError> {
        let model = self.account_model(user_id, id).await?;
        let mut active = model.into_active_model();
        active.last_synced_at = Set(Some(OffsetDateTime::now_utc().unix_timestamp()));
        active.last_error = Set(error);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        active.update(self.db.connection()).await?;
        Ok(())
    }

    async fn account_model(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<calendar_account::Model, AppError> {
        calendar_account::Entity::find()
            .filter(calendar_account::Column::Id.eq(id.to_string()))
            .filter(calendar_account::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)
    }

    async fn local_event_model(
        &self,
        user_id: Uuid,
        id: Uuid,
    ) -> Result<local_calendar_event::Model, AppError> {
        local_calendar_event::Entity::find()
            .filter(local_calendar_event::Column::Id.eq(id.to_string()))
            .filter(local_calendar_event::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)
    }
}

impl CalendarAccountSecrets {
    pub fn password(&self) -> &str {
        self.password.expose_secret()
    }
}

impl TryFrom<calendar_account::Model> for CalendarAccount {
    type Error = AppError;

    fn try_from(model: calendar_account::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            name: model.name,
            base_url: model.base_url,
            username: model.username,
            enabled: model.enabled,
            has_password: true,
            last_synced_at: model.last_synced_at,
            last_error: model.last_error,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

impl TryFrom<calendar::Model> for Calendar {
    type Error = AppError;

    fn try_from(model: calendar::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            account_id: Uuid::parse_str(&model.account_id).map_err(AppError::internal)?,
            display_name: model.display_name,
            color: model.color,
            remote_href: model.remote_href,
            sync_token: model.sync_token,
            enabled: model.enabled,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

impl TryFrom<calendar_event::Model> for CalendarEvent {
    type Error = AppError;

    fn try_from(model: calendar_event::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            calendar_id: Uuid::parse_str(&model.calendar_id).map_err(AppError::internal)?,
            uid: model.uid,
            summary: model.summary,
            description: model.description,
            location: model.location,
            starts_at: model.starts_at,
            ends_at: model.ends_at,
            all_day: model.all_day,
            timezone: model.timezone,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

impl TryFrom<local_calendar_event::Model> for LocalCalendarEvent {
    type Error = AppError;

    fn try_from(model: local_calendar_event::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            summary: model.summary,
            description: model.description,
            location: model.location,
            starts_at: model.starts_at,
            ends_at: model.ends_at,
            all_day: model.all_day,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}
