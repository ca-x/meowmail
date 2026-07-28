use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "calendars")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub account_id: String,
    pub display_name: String,
    pub color: String,
    pub remote_href: String,
    pub sync_token: Option<String>,
    pub enabled: bool,
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
        belongs_to = "super::calendar_account::Entity",
        from = "Column::AccountId",
        to = "super::calendar_account::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    CalendarAccount,
    #[sea_orm(has_many = "super::calendar_event::Entity")]
    CalendarEvent,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::calendar_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CalendarAccount.def()
    }
}

impl Related<super::calendar_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CalendarEvent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
