use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use time::OffsetDateTime;

use crate::models::_entities::admin_user;

#[serde_as]
#[derive(Debug, Serialize)]
pub(crate) struct AdminUserResponse {
    #[serde_as(as = "DisplayFromStr")]
    pub id: i64,
    pub username: String,
    pub nick_name: String,
    pub email: String,
    pub phone_number: String,
    pub gender: i16,
    pub state: i16,
    pub remark: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl AdminUserResponse {
    pub(crate) fn new(model: admin_user::Model) -> Self {
        Self { 
            id: model.id, 
            username: model.username, 
            nick_name: model.nick_name, 
            email: model.email, 
            phone_number: model.phone_number, 
            gender: model.gender, 
            state: model.state, 
            remark: model.remark, 
            created_at: model.created_at.assume_utc(),
        }
    }
}