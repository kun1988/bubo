use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use time::OffsetDateTime;

use crate::models::_entities::admin_role;

#[serde_as]
#[derive(Debug, Serialize)]
pub(crate) struct RoleResponse {
    #[serde_as(as = "DisplayFromStr")]
    pub id: i64,
    pub name: String,
    pub code: String,
    pub state: i16,
    pub display_order: i16,
    pub remark: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl RoleResponse {
    pub(crate) fn new(model: admin_role::Model) -> Self {
        Self { 
            id: model.id, 
            name: model.name, 
            code: model.code, 
            state: model.state, 
            display_order: model.display_order, 
            remark: model.remark, 
            created_at: model.created_at.assume_utc(),
        }
    }
}