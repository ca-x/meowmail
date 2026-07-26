use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    accounts::MailAccount,
    db::{Database, entities::message},
    error::AppError,
    mail::ParsedMail,
    notifications::NotificationEvent,
};

#[derive(Clone)]
pub struct MessageRepository {
    db: Database,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSummary {
    pub id: Uuid,
    pub account_id: Uuid,
    pub folder: String,
    pub uid: i64,
    pub sender_name: Option<String>,
    pub sender_email: String,
    pub subject: String,
    pub preview: String,
    pub received_at: i64,
    pub is_read: bool,
    pub is_starred: bool,
    pub attachment_count: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDetail {
    #[serde(flatten)]
    pub summary: MessageSummary,
    pub message_id: Option<String>,
    pub recipients: Vec<String>,
    pub body_text: String,
    pub body_html: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MessageFilter {
    pub account_id: Option<Uuid>,
    pub folder: String,
    pub unread: bool,
    pub starred: bool,
    pub has_attachment: bool,
    pub query: Option<String>,
    pub limit: u64,
}

pub struct NewMessage {
    pub folder: String,
    pub uid: i64,
    pub mail: ParsedMail,
    pub is_read: bool,
    pub is_starred: bool,
}

impl MessageRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        filter: MessageFilter,
    ) -> Result<Vec<MessageSummary>, AppError> {
        let mut query = message::Entity::find()
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .filter(message::Column::Folder.eq(filter.folder));
        if let Some(account_id) = filter.account_id {
            query = query.filter(message::Column::AccountId.eq(account_id.to_string()));
        }
        if filter.unread {
            query = query.filter(message::Column::IsRead.eq(false));
        }
        if filter.starred {
            query = query.filter(message::Column::IsStarred.eq(true));
        }
        if filter.has_attachment {
            query = query.filter(message::Column::AttachmentCount.gt(0));
        }
        if let Some(search) = filter.query.filter(|value| !value.is_empty()) {
            query = query.filter(
                Condition::any()
                    .add(message::Column::Subject.contains(&search))
                    .add(message::Column::SenderName.contains(&search))
                    .add(message::Column::SenderEmail.contains(&search))
                    .add(message::Column::Preview.contains(&search)),
            );
        }
        query
            .order_by_desc(message::Column::ReceivedAt)
            .limit(filter.limit.min(200))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(MessageSummary::try_from)
            .collect()
    }

    pub async fn get(&self, user_id: Uuid, id: Uuid) -> Result<MessageDetail, AppError> {
        let model = message::Entity::find()
            .filter(message::Column::Id.eq(id.to_string()))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let recipients =
            serde_json::from_str(&model.recipients_json).map_err(AppError::internal)?;
        Ok(MessageDetail {
            summary: MessageSummary::try_from(model.clone())?,
            message_id: model.message_id,
            recipients,
            body_text: model.body_text,
            body_html: model.body_html,
        })
    }

    pub async fn update_flags(
        &self,
        user_id: Uuid,
        id: Uuid,
        is_read: Option<bool>,
        is_starred: Option<bool>,
    ) -> Result<MessageSummary, AppError> {
        let model = message::Entity::find()
            .filter(message::Column::Id.eq(id.to_string()))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let mut active = model.into_active_model();
        if let Some(value) = is_read {
            active.is_read = Set(value);
        }
        if let Some(value) = is_starred {
            active.is_starred = Set(value);
        }
        MessageSummary::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn insert_if_new(
        &self,
        user_id: Uuid,
        account: &MailAccount,
        input: NewMessage,
    ) -> Result<Option<NotificationEvent>, AppError> {
        let NewMessage {
            folder,
            uid,
            mail,
            is_read,
            is_starred,
        } = input;
        let exists = message::Entity::find()
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .filter(message::Column::AccountId.eq(account.id.to_string()))
            .filter(message::Column::Folder.eq(&folder))
            .filter(message::Column::Uid.eq(uid))
            .one(self.db.connection())
            .await?
            .is_some();
        if exists {
            return Ok(None);
        }
        let sender = mail
            .sender_name
            .clone()
            .unwrap_or_else(|| mail.sender_email.clone());
        let event = NotificationEvent {
            user_id,
            account: account.display_name.clone(),
            email: account.email.clone(),
            sender,
            sender_email: mail.sender_email.clone(),
            subject: mail.subject.clone(),
            preview: mail.preview.clone(),
        };
        message::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            user_id: Set(Some(user_id.to_string())),
            account_id: Set(account.id.to_string()),
            folder: Set(folder),
            uid: Set(uid),
            message_id: Set(mail.message_id),
            sender_name: Set(mail.sender_name),
            sender_email: Set(mail.sender_email),
            recipients_json: Set(
                serde_json::to_string(&mail.recipients).map_err(AppError::internal)?
            ),
            subject: Set(mail.subject),
            preview: Set(mail.preview),
            body_text: Set(mail.body_text),
            body_html: Set(mail.body_html),
            received_at: Set(mail.received_at),
            is_read: Set(is_read),
            is_starred: Set(is_starred),
            attachment_count: Set(mail.attachment_count as i32),
            created_at: Set(OffsetDateTime::now_utc().unix_timestamp()),
        }
        .insert(self.db.connection())
        .await?;
        Ok(Some(event))
    }
}

impl TryFrom<message::Model> for MessageSummary {
    type Error = AppError;

    fn try_from(model: message::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            account_id: Uuid::parse_str(&model.account_id).map_err(AppError::internal)?,
            folder: model.folder,
            uid: model.uid,
            sender_name: model.sender_name,
            sender_email: model.sender_email,
            subject: model.subject,
            preview: model.preview,
            received_at: model.received_at,
            is_read: model.is_read,
            is_starred: model.is_starred,
            attachment_count: model.attachment_count,
        })
    }
}
