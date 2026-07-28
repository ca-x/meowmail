use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "email_drafts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub account_id: String,
    pub reply_to_message_id: Option<String>,
    pub to_json: String,
    pub cc_json: String,
    pub bcc_json: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
    pub attachments_json: String,
    pub signature_id: Option<String>,
    pub apply_signature: bool,
    pub in_reply_to: Option<String>,
    pub references_header: Option<String>,
    pub status: String,
    pub scheduled_at: Option<i64>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::mail_account::Entity",
        from = "Column::AccountId",
        to = "super::mail_account::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    MailAccount,
    #[sea_orm(
        belongs_to = "super::message::Entity",
        from = "Column::ReplyToMessageId",
        to = "super::message::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Message,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::mail_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MailAccount.def()
    }
}

impl Related<super::message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
