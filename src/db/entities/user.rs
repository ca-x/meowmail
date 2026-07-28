use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub username: String,
    pub nickname: String,
    pub email: Option<String>,
    pub role: String,
    pub password_hash: Option<String>,
    pub pin_hash: Option<String>,
    pub avatar_mime: Option<String>,
    pub avatar_data: Option<Vec<u8>>,
    pub ai_enabled: bool,
    pub auto_lock_minutes: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_identity::Entity")]
    Identity,
    #[sea_orm(has_many = "super::mail_account::Entity")]
    MailAccount,
}

impl Related<super::user_identity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Identity.def()
    }
}

impl Related<super::mail_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MailAccount.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
