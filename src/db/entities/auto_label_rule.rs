use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "auto_label_rules")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub account_id: Option<String>,
    pub provider_id: Option<String>,
    pub name: String,
    pub label_ids_json: String,
    pub instructions: String,
    pub enabled: bool,
    pub apply_automatically: bool,
    pub source_subscription_id: Option<String>,
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
        on_delete = "SetNull"
    )]
    MailAccount,
    #[sea_orm(
        belongs_to = "super::ai_provider::Entity",
        from = "Column::ProviderId",
        to = "super::ai_provider::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    AiProvider,
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

impl Related<super::ai_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AiProvider.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
