//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "admin_user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: i64,
    #[sea_orm(unique)]
    pub username: String,
    pub nick_name: String,
    pub email: String,
    pub phone_number: String,
    pub gender: i16,
    pub password: String,
    pub state: i16,
    pub is_deleted: bool,
    pub is_admin: bool,
    pub remark: String,
    pub created_by: i64,
    pub created_at: TimeDateTime,
    pub updated_by: i64,
    pub updated_at: TimeDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}