use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "messages")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub account_id: String,
    pub folder: String,
    pub uid: i64,
    pub message_id: Option<String>,
    pub sender_name: Option<String>,
    pub sender_email: String,
    pub recipients_json: String,
    pub subject: String,
    pub preview: String,
    pub body_text: String,
    pub body_html: Option<String>,
    pub received_at: i64,
    pub is_read: bool,
    pub is_starred: bool,
    pub attachment_count: i32,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::mail_account::Entity",
        from = "Column::AccountId",
        to = "super::mail_account::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    MailAccount,
}

impl Related<super::mail_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MailAccount.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
