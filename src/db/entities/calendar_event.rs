use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "calendar_events")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub user_id: String,
    pub calendar_id: String,
    pub uid: String,
    pub summary: String,
    pub description: String,
    pub location: String,
    pub starts_at: i64,
    pub ends_at: i64,
    pub all_day: bool,
    pub timezone: Option<String>,
    pub remote_href: Option<String>,
    pub etag: Option<String>,
    pub ics: String,
    pub deleted: bool,
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
        belongs_to = "super::calendar::Entity",
        from = "Column::CalendarId",
        to = "super::calendar::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Calendar,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::calendar::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Calendar.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
