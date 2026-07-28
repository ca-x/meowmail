use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "message_labels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub message_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub label_id: String,
    pub user_id: String,
    pub created_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::message::Entity",
        from = "Column::MessageId",
        to = "super::message::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Message,
    #[sea_orm(
        belongs_to = "super::label::Entity",
        from = "Column::LabelId",
        to = "super::label::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Label,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl Related<super::label::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Label.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
