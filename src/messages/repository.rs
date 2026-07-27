use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseTransaction, EntityTrait, FromQueryResult,
    IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    accounts::MailAccount,
    db::{
        Database,
        entities::{message, message_attachment},
    },
    error::AppError,
    mail::{MailAttachment, ParsedMail},
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
    #[serde(skip)]
    pub uid_validity: Option<i64>,
    pub sender_name: Option<String>,
    pub sender_email: String,
    pub subject: String,
    pub thread_key: String,
    pub preview: String,
    pub received_at: i64,
    pub is_read: bool,
    pub is_starred: bool,
    pub attachment_count: i32,
    pub raw_size: i64,
    pub is_promotional: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDetail {
    #[serde(flatten)]
    pub summary: MessageSummary,
    pub message_id: Option<String>,
    pub reply_to_email: Option<String>,
    pub references: Vec<String>,
    pub recipients: Vec<String>,
    pub cc_recipients: Vec<String>,
    pub body_text: String,
    pub body_html: Option<String>,
    pub attachments: Vec<MessageAttachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageAttachment {
    pub id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub available: bool,
}

pub struct AttachmentContent {
    pub filename: String,
    pub content_type: String,
    pub content: Vec<u8>,
}

#[derive(FromQueryResult)]
struct AttachmentMetadataRow {
    id: String,
    filename: String,
    content_type: String,
    size: i64,
    available: bool,
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
    pub uid_validity: Option<i64>,
    pub mail: ParsedMail,
    pub is_read: bool,
    pub is_starred: bool,
}

pub struct MessageInsertResult {
    pub notification: Option<NotificationEvent>,
    pub created: bool,
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
        let cc_recipients =
            serde_json::from_str(&model.cc_recipients_json).map_err(AppError::internal)?;
        let references =
            serde_json::from_str(&model.references_header).map_err(AppError::internal)?;
        let attachments = message_attachment::Entity::find()
            .select_only()
            .columns([
                message_attachment::Column::Id,
                message_attachment::Column::Filename,
                message_attachment::Column::ContentType,
                message_attachment::Column::Size,
            ])
            .column_as(
                Expr::col(message_attachment::Column::Content).is_not_null(),
                "available",
            )
            .filter(message_attachment::Column::MessageId.eq(&model.id))
            .order_by_asc(message_attachment::Column::Position)
            .into_model::<AttachmentMetadataRow>()
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(MessageAttachment::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(MessageDetail {
            summary: MessageSummary::try_from(model.clone())?,
            message_id: model.message_id,
            reply_to_email: model.reply_to_email,
            references,
            recipients,
            cc_recipients,
            body_text: model.body_text,
            body_html: model.body_html,
            attachments,
        })
    }

    pub async fn thread(&self, user_id: Uuid, id: Uuid) -> Result<Vec<MessageDetail>, AppError> {
        let anchor = message::Entity::find()
            .filter(message::Column::Id.eq(id.to_string()))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        if anchor.thread_key.is_empty() {
            return Ok(vec![self.get(user_id, id).await?]);
        }
        let models = message::Entity::find()
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .filter(message::Column::AccountId.eq(&anchor.account_id))
            .filter(message::Column::Folder.eq(&anchor.folder))
            .filter(message::Column::ThreadKey.eq(&anchor.thread_key))
            .order_by_asc(message::Column::ReceivedAt)
            .limit(100)
            .all(self.db.connection())
            .await?;
        let mut messages = Vec::with_capacity(models.len());
        for model in models {
            let id = Uuid::parse_str(&model.id).map_err(AppError::internal)?;
            messages.push(self.get(user_id, id).await?);
        }
        Ok(messages)
    }

    pub async fn get_attachment(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<AttachmentContent, AppError> {
        let message_exists = message::Entity::find()
            .filter(message::Column::Id.eq(message_id.to_string()))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .is_some();
        if !message_exists {
            return Err(AppError::NotFound);
        }
        let attachment = message_attachment::Entity::find()
            .filter(message_attachment::Column::Id.eq(attachment_id.to_string()))
            .filter(message_attachment::Column::MessageId.eq(message_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        Ok(AttachmentContent {
            filename: attachment.filename,
            content_type: attachment.content_type,
            content: attachment.content.ok_or(AppError::NotFound)?,
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

    pub async fn delete_local(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = message::Entity::delete_many()
            .filter(message::Column::Id.eq(id.to_string()))
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            return Err(AppError::NotFound);
        }
        Ok(())
    }

    pub async fn insert_if_new(
        &self,
        user_id: Uuid,
        account: &MailAccount,
        input: NewMessage,
    ) -> Result<MessageInsertResult, AppError> {
        let NewMessage {
            folder,
            uid,
            uid_validity,
            mail,
            is_read,
            is_starred,
        } = input;
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
        let transaction = self.db.connection().begin().await?;
        let existing = message::Entity::find()
            .filter(message::Column::UserId.eq(user_id.to_string()))
            .filter(message::Column::AccountId.eq(account.id.to_string()))
            .filter(message::Column::Folder.eq(&folder))
            .filter(message::Column::Uid.eq(uid))
            .one(&transaction)
            .await?;
        if let Some(existing) = existing {
            let message_id = existing.id.clone();
            let validity_changed = existing.uid_validity.is_some()
                && uid_validity.is_some()
                && existing.uid_validity != uid_validity;
            let mut active = existing.into_active_model();
            active.uid_validity = Set(uid_validity);
            active.message_id = Set(mail.message_id.clone());
            active.reply_to_email = Set(mail.reply_to_email.clone());
            active.references_header =
                Set(serde_json::to_string(&mail.references).map_err(AppError::internal)?);
            active.sender_name = Set(mail.sender_name.clone());
            active.sender_email = Set(mail.sender_email.clone());
            active.recipients_json =
                Set(serde_json::to_string(&mail.recipients).map_err(AppError::internal)?);
            active.cc_recipients_json =
                Set(serde_json::to_string(&mail.cc_recipients).map_err(AppError::internal)?);
            active.subject = Set(mail.subject.clone());
            active.thread_key = Set(mail.thread_key.clone());
            active.preview = Set(mail.preview.clone());
            active.body_text = Set(mail.body_text.clone());
            active.body_html = Set(mail.body_html.clone());
            active.received_at = Set(mail.received_at);
            active.is_read = Set(is_read);
            active.is_starred = Set(is_starred);
            active.attachment_count = Set(mail.attachment_count as i32);
            active.raw_size = Set(i64::try_from(mail.raw_size).map_err(AppError::internal)?);
            active.is_promotional = Set(mail.is_promotional);
            active.auto_response_allowed = Set(mail.auto_response_allowed);
            active.update(&transaction).await?;
            replace_attachments(&transaction, &message_id, mail.attachments).await?;
            transaction.commit().await?;
            return Ok(MessageInsertResult {
                notification: validity_changed.then_some(event),
                created: false,
            });
        }
        let message_id = Uuid::new_v4().to_string();
        message::ActiveModel {
            id: Set(message_id.clone()),
            user_id: Set(Some(user_id.to_string())),
            account_id: Set(account.id.to_string()),
            folder: Set(folder),
            uid: Set(uid),
            uid_validity: Set(uid_validity),
            message_id: Set(mail.message_id),
            reply_to_email: Set(mail.reply_to_email),
            references_header: Set(
                serde_json::to_string(&mail.references).map_err(AppError::internal)?
            ),
            sender_name: Set(mail.sender_name),
            sender_email: Set(mail.sender_email),
            recipients_json: Set(
                serde_json::to_string(&mail.recipients).map_err(AppError::internal)?
            ),
            cc_recipients_json: Set(
                serde_json::to_string(&mail.cc_recipients).map_err(AppError::internal)?
            ),
            subject: Set(mail.subject),
            thread_key: Set(mail.thread_key),
            preview: Set(mail.preview),
            body_text: Set(mail.body_text),
            body_html: Set(mail.body_html),
            received_at: Set(mail.received_at),
            is_read: Set(is_read),
            is_starred: Set(is_starred),
            attachment_count: Set(mail.attachment_count as i32),
            raw_size: Set(i64::try_from(mail.raw_size).map_err(AppError::internal)?),
            is_promotional: Set(mail.is_promotional),
            auto_response_allowed: Set(mail.auto_response_allowed),
            created_at: Set(OffsetDateTime::now_utc().unix_timestamp()),
        }
        .insert(&transaction)
        .await?;
        replace_attachments(&transaction, &message_id, mail.attachments).await?;
        transaction.commit().await?;
        Ok(MessageInsertResult {
            notification: Some(event),
            created: true,
        })
    }
}

async fn replace_attachments(
    transaction: &DatabaseTransaction,
    message_id: &str,
    attachments: Vec<MailAttachment>,
) -> Result<(), AppError> {
    let existing = message_attachment::Entity::find()
        .filter(message_attachment::Column::MessageId.eq(message_id))
        .order_by_asc(message_attachment::Column::Position)
        .all(transaction)
        .await?;
    let created_at = OffsetDateTime::now_utc().unix_timestamp();
    let mut stale_ids = existing
        .iter()
        .map(|model| model.id.clone())
        .collect::<Vec<_>>();
    for (position, attachment) in attachments.into_iter().enumerate() {
        let position = i32::try_from(position).map_err(AppError::internal)?;
        let size = i64::try_from(attachment.size).map_err(AppError::internal)?;
        if let Some(model) = existing.iter().find(|model| model.position == position) {
            stale_ids.retain(|id| id != &model.id);
            if model.filename != attachment.filename
                || model.content_type != attachment.content_type
                || model.size != size
                || model.content != attachment.content
            {
                let mut active = model.clone().into_active_model();
                active.filename = Set(attachment.filename);
                active.content_type = Set(attachment.content_type);
                active.size = Set(size);
                active.content = Set(attachment.content);
                active.update(transaction).await?;
            }
        } else {
            message_attachment::ActiveModel {
                id: Set(Uuid::new_v4().to_string()),
                message_id: Set(message_id.to_owned()),
                position: Set(position),
                filename: Set(attachment.filename),
                content_type: Set(attachment.content_type),
                size: Set(size),
                content: Set(attachment.content),
                created_at: Set(created_at),
            }
            .insert(transaction)
            .await?;
        }
    }
    if !stale_ids.is_empty() {
        message_attachment::Entity::delete_many()
            .filter(message_attachment::Column::Id.is_in(stale_ids))
            .exec(transaction)
            .await?;
    }
    Ok(())
}

impl TryFrom<message::Model> for MessageSummary {
    type Error = AppError;

    fn try_from(model: message::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            account_id: Uuid::parse_str(&model.account_id).map_err(AppError::internal)?,
            folder: model.folder,
            uid: model.uid,
            uid_validity: model.uid_validity,
            sender_name: model.sender_name,
            sender_email: model.sender_email,
            subject: model.subject,
            thread_key: model.thread_key,
            preview: model.preview,
            received_at: model.received_at,
            is_read: model.is_read,
            is_starred: model.is_starred,
            attachment_count: model.attachment_count,
            raw_size: model.raw_size,
            is_promotional: model.is_promotional,
        })
    }
}

impl TryFrom<AttachmentMetadataRow> for MessageAttachment {
    type Error = AppError;

    fn try_from(model: AttachmentMetadataRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            filename: model.filename,
            content_type: model.content_type,
            size: model.size,
            available: model.available,
        })
    }
}
