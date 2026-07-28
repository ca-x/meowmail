use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, Set, sea_query::Expr,
};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{
        Database,
        entities::{email_draft, mail_account},
    },
    error::AppError,
    messages::{ComposeAttachmentInput, ComposeInput, ThreadingHeaders},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDraft {
    pub id: Uuid,
    pub account_id: Uuid,
    pub reply_to_message_id: Option<Uuid>,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
    pub editor_document: Option<Value>,
    pub attachments: Vec<ComposeAttachmentInput>,
    pub signature_id: Option<Uuid>,
    pub apply_signature: bool,
    pub scheduled_at: Option<i64>,
    pub last_error: Option<String>,
    pub status: EmailDraftStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EmailDraftStatus {
    Draft,
    Sending,
    Ambiguous,
    Sent,
}

impl EmailDraftStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Sending => "sending",
            Self::Ambiguous => "ambiguous",
            Self::Sent => "sent",
        }
    }
}

impl TryFrom<&str> for EmailDraftStatus {
    type Error = AppError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "draft" => Ok(Self::Draft),
            "sending" => Ok(Self::Sending),
            "ambiguous" => Ok(Self::Ambiguous),
            "sent" => Ok(Self::Sent),
            _ => Err(AppError::internal(anyhow::anyhow!(
                "unknown email draft status: {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoredDraft {
    pub draft: EmailDraft,
    pub threading: ThreadingHeaders,
}

impl StoredDraft {
    pub fn into_compose(self) -> ComposeInput {
        ComposeInput {
            account_id: self.draft.account_id,
            to: self.draft.to,
            cc: self.draft.cc,
            bcc: self.draft.bcc,
            subject: self.draft.subject,
            text_body: self.draft.text_body,
            html_body: self.draft.html_body,
            editor_document: self.draft.editor_document,
            attachments: self.draft.attachments,
            signature_id: self.draft.signature_id,
            apply_signature: self.draft.apply_signature,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledDraft {
    pub user_id: Uuid,
    pub stored: StoredDraft,
}

#[derive(Clone)]
pub struct DraftRepository {
    db: Database,
}

impl DraftRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn reconcile_interrupted_sends(&self) -> Result<u64, AppError> {
        let result = email_draft::Entity::update_many()
            .col_expr(
                email_draft::Column::Status,
                Expr::value(EmailDraftStatus::Ambiguous.as_str()),
            )
            .col_expr(
                email_draft::Column::UpdatedAt,
                Expr::value(OffsetDateTime::now_utc().unix_timestamp()),
            )
            .filter(email_draft::Column::Status.eq(EmailDraftStatus::Sending.as_str()))
            .exec(self.db.connection())
            .await?;
        Ok(result.rows_affected)
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        mut compose: ComposeInput,
        reply_to_message_id: Option<Uuid>,
        threading: ThreadingHeaders,
        scheduled_at: Option<i64>,
    ) -> Result<EmailDraft, AppError> {
        if scheduled_at.is_some() {
            compose.validate()?;
        } else {
            compose.validate_draft()?;
        }
        self.ensure_account(user_id, compose.account_id).await?;
        let id = Uuid::new_v4();
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let model = email_draft::ActiveModel {
            id: Set(id.to_string()),
            user_id: Set(user_id.to_string()),
            account_id: Set(compose.account_id.to_string()),
            reply_to_message_id: Set(reply_to_message_id.map(|id| id.to_string())),
            to_json: Set(serde_json::to_string(&compose.to).map_err(AppError::internal)?),
            cc_json: Set(serde_json::to_string(&compose.cc).map_err(AppError::internal)?),
            bcc_json: Set(serde_json::to_string(&compose.bcc).map_err(AppError::internal)?),
            subject: Set(compose.subject),
            text_body: Set(compose.text_body),
            html_body: Set(compose.html_body),
            editor_document: Set(compose
                .editor_document
                .map(|value| serde_json::to_string(&value).map_err(AppError::internal))
                .transpose()?),
            attachments_json: Set(
                serde_json::to_string(&compose.attachments).map_err(AppError::internal)?
            ),
            signature_id: Set(compose.signature_id.map(|id| id.to_string())),
            apply_signature: Set(compose.apply_signature),
            in_reply_to: Set(threading.in_reply_to),
            references_header: Set(Some(
                serde_json::to_string(&threading.references).map_err(AppError::internal)?,
            )),
            status: Set(EmailDraftStatus::Draft.as_str().into()),
            scheduled_at: Set(scheduled_at),
            last_error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(self.db.connection())
        .await?;
        EmailDraft::try_from(model)
    }

    pub async fn list(&self, user_id: Uuid, limit: u64) -> Result<Vec<EmailDraft>, AppError> {
        email_draft::Entity::find()
            .filter(email_draft::Column::UserId.eq(user_id.to_string()))
            .order_by_desc(email_draft::Column::UpdatedAt)
            .limit(limit.clamp(1, 100))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(EmailDraft::try_from)
            .collect()
    }

    pub async fn update(
        &self,
        user_id: Uuid,
        id: Uuid,
        mut compose: ComposeInput,
        scheduled_at: Option<i64>,
    ) -> Result<EmailDraft, AppError> {
        if scheduled_at.is_some() {
            compose.validate()?;
        } else {
            compose.validate_draft()?;
        }
        self.ensure_account(user_id, compose.account_id).await?;
        let model = email_draft::Entity::find()
            .filter(email_draft::Column::Id.eq(id.to_string()))
            .filter(email_draft::Column::UserId.eq(user_id.to_string()))
            .filter(email_draft::Column::Status.eq(EmailDraftStatus::Draft.as_str()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let mut active = model.into_active_model();
        active.account_id = Set(compose.account_id.to_string());
        active.to_json = Set(serde_json::to_string(&compose.to).map_err(AppError::internal)?);
        active.cc_json = Set(serde_json::to_string(&compose.cc).map_err(AppError::internal)?);
        active.bcc_json = Set(serde_json::to_string(&compose.bcc).map_err(AppError::internal)?);
        active.subject = Set(compose.subject);
        active.text_body = Set(compose.text_body);
        active.html_body = Set(compose.html_body);
        active.editor_document = Set(compose
            .editor_document
            .map(|value| serde_json::to_string(&value).map_err(AppError::internal))
            .transpose()?);
        active.attachments_json =
            Set(serde_json::to_string(&compose.attachments).map_err(AppError::internal)?);
        active.signature_id = Set(compose.signature_id.map(|id| id.to_string()));
        active.apply_signature = Set(compose.apply_signature);
        active.scheduled_at = Set(scheduled_at);
        active.last_error = Set(None);
        active.updated_at = Set(OffsetDateTime::now_utc().unix_timestamp());
        EmailDraft::try_from(active.update(self.db.connection()).await?)
    }

    pub async fn delete(&self, user_id: Uuid, id: Uuid) -> Result<(), AppError> {
        let result = email_draft::Entity::delete_many()
            .filter(email_draft::Column::Id.eq(id.to_string()))
            .filter(email_draft::Column::UserId.eq(user_id.to_string()))
            .filter(email_draft::Column::Status.ne(EmailDraftStatus::Sending.as_str()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            let exists = email_draft::Entity::find()
                .filter(email_draft::Column::Id.eq(id.to_string()))
                .filter(email_draft::Column::UserId.eq(user_id.to_string()))
                .one(self.db.connection())
                .await?
                .is_some();
            return Err(if exists {
                AppError::Conflict
            } else {
                AppError::NotFound
            });
        }
        Ok(())
    }

    pub async fn list_due_scheduled(&self, limit: u64) -> Result<Vec<ScheduledDraft>, AppError> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        email_draft::Entity::find()
            .filter(email_draft::Column::Status.eq(EmailDraftStatus::Draft.as_str()))
            .filter(email_draft::Column::ScheduledAt.is_not_null())
            .filter(email_draft::Column::ScheduledAt.lte(now))
            .order_by_asc(email_draft::Column::ScheduledAt)
            .limit(limit.clamp(1, 50))
            .all(self.db.connection())
            .await?
            .into_iter()
            .map(|model| {
                let user_id = Uuid::parse_str(&model.user_id).map_err(AppError::internal)?;
                let threading = ThreadingHeaders {
                    in_reply_to: model.in_reply_to.clone(),
                    references: parse_references(model.references_header.as_deref())?,
                    ..ThreadingHeaders::default()
                };
                Ok(ScheduledDraft {
                    user_id,
                    stored: StoredDraft {
                        draft: EmailDraft::try_from(model)?,
                        threading,
                    },
                })
            })
            .collect()
    }

    pub async fn claim_for_send(&self, user_id: Uuid, id: Uuid) -> Result<StoredDraft, AppError> {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let result = email_draft::Entity::update_many()
            .col_expr(
                email_draft::Column::Status,
                Expr::value(EmailDraftStatus::Sending.as_str()),
            )
            .col_expr(email_draft::Column::UpdatedAt, Expr::value(now))
            .filter(email_draft::Column::Id.eq(id.to_string()))
            .filter(email_draft::Column::UserId.eq(user_id.to_string()))
            .filter(email_draft::Column::Status.eq(EmailDraftStatus::Draft.as_str()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            let exists = email_draft::Entity::find()
                .filter(email_draft::Column::Id.eq(id.to_string()))
                .filter(email_draft::Column::UserId.eq(user_id.to_string()))
                .one(self.db.connection())
                .await?
                .is_some();
            return Err(if exists {
                AppError::Conflict
            } else {
                AppError::NotFound
            });
        }
        self.get(user_id, id).await
    }

    async fn get(&self, user_id: Uuid, id: Uuid) -> Result<StoredDraft, AppError> {
        let model = email_draft::Entity::find()
            .filter(email_draft::Column::Id.eq(id.to_string()))
            .filter(email_draft::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .ok_or(AppError::NotFound)?;
        let threading = ThreadingHeaders {
            in_reply_to: model.in_reply_to.clone(),
            references: parse_references(model.references_header.as_deref())?,
            ..ThreadingHeaders::default()
        };
        Ok(StoredDraft {
            draft: EmailDraft::try_from(model)?,
            threading,
        })
    }

    pub async fn finish_sent(&self, user_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        let result = email_draft::Entity::delete_many()
            .filter(email_draft::Column::Id.eq(id.to_string()))
            .filter(email_draft::Column::UserId.eq(user_id.to_string()))
            .filter(email_draft::Column::Status.eq(EmailDraftStatus::Sending.as_str()))
            .exec(self.db.connection())
            .await?;
        if result.rows_affected == 0 {
            let exists = email_draft::Entity::find()
                .filter(email_draft::Column::Id.eq(id.to_string()))
                .filter(email_draft::Column::UserId.eq(user_id.to_string()))
                .one(self.db.connection())
                .await?
                .is_some();
            return Ok(!exists);
        }
        Ok(true)
    }

    pub async fn mark_after_send_failure(
        &self,
        user_id: Uuid,
        id: Uuid,
        status: EmailDraftStatus,
    ) -> Result<(), AppError> {
        debug_assert!(matches!(
            status,
            EmailDraftStatus::Draft | EmailDraftStatus::Ambiguous | EmailDraftStatus::Sent
        ));
        email_draft::Entity::update_many()
            .col_expr(email_draft::Column::Status, Expr::value(status.as_str()))
            .col_expr(
                email_draft::Column::LastError,
                Expr::value(if status == EmailDraftStatus::Draft {
                    None::<String>
                } else {
                    Some("send failed".to_owned())
                }),
            )
            .col_expr(email_draft::Column::ScheduledAt, Expr::value(None::<i64>))
            .col_expr(
                email_draft::Column::UpdatedAt,
                Expr::value(OffsetDateTime::now_utc().unix_timestamp()),
            )
            .filter(email_draft::Column::Id.eq(id.to_string()))
            .filter(email_draft::Column::UserId.eq(user_id.to_string()))
            .filter(email_draft::Column::Status.eq(EmailDraftStatus::Sending.as_str()))
            .exec(self.db.connection())
            .await?;
        Ok(())
    }

    async fn ensure_account(&self, user_id: Uuid, account_id: Uuid) -> Result<(), AppError> {
        let owns_account = mail_account::Entity::find()
            .filter(mail_account::Column::Id.eq(account_id.to_string()))
            .filter(mail_account::Column::UserId.eq(user_id.to_string()))
            .one(self.db.connection())
            .await?
            .is_some();
        if owns_account {
            Ok(())
        } else {
            Err(AppError::NotFound)
        }
    }
}

impl TryFrom<email_draft::Model> for EmailDraft {
    type Error = AppError;

    fn try_from(model: email_draft::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: Uuid::parse_str(&model.id).map_err(AppError::internal)?,
            account_id: Uuid::parse_str(&model.account_id).map_err(AppError::internal)?,
            reply_to_message_id: model
                .reply_to_message_id
                .map(|id| Uuid::parse_str(&id).map_err(AppError::internal))
                .transpose()?,
            to: serde_json::from_str(&model.to_json).map_err(AppError::internal)?,
            cc: serde_json::from_str(&model.cc_json).map_err(AppError::internal)?,
            bcc: serde_json::from_str(&model.bcc_json).map_err(AppError::internal)?,
            subject: model.subject,
            text_body: model.text_body,
            html_body: model.html_body,
            editor_document: model
                .editor_document
                .map(|value| serde_json::from_str(&value).map_err(AppError::internal))
                .transpose()?,
            attachments: serde_json::from_str(&model.attachments_json)
                .map_err(AppError::internal)?,
            signature_id: model
                .signature_id
                .map(|id| Uuid::parse_str(&id).map_err(AppError::internal))
                .transpose()?,
            apply_signature: model.apply_signature,
            scheduled_at: model.scheduled_at,
            last_error: model.last_error,
            status: EmailDraftStatus::try_from(model.status.as_str())?,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }
}

fn parse_references(value: Option<&str>) -> Result<Vec<String>, AppError> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };
    serde_json::from_str(value).or_else(|_| {
        Ok(value
            .split_whitespace()
            .map(|item| item.trim_matches(['<', '>']).to_owned())
            .filter(|item| !item.is_empty())
            .collect())
    })
}
