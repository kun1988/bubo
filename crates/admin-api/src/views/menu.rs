use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use time::OffsetDateTime;

use crate::models::_entities::admin_menu;

#[serde_as]
#[derive(Debug, Serialize)]
pub(crate) struct MenuResponse {
    #[serde_as(as = "DisplayFromStr")]
    pub id: i64,
    pub name: String,
    pub parent_id: i64,
    pub url: String,
    pub target: i16,
    pub menu_type: i16,
    pub is_hidden: bool,
    pub is_refresh: bool,
    pub permission: String,
    pub icon: String,
    pub display_order: i16,
    pub remark: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl MenuResponse {
    pub(crate) fn new(model: admin_menu::Model) -> Self {
        Self { 
            id: model.id, 
            name: model.name, 
            parent_id: model.parent_id, 
            url: model.url,
            target: model.target,
            menu_type: model.menu_type,
            is_hidden: model.is_hidden,
            is_refresh: model.is_refresh,
            permission: model.permission,
            icon: model.icon, 
            display_order: model.display_order, 
            remark: model.remark, 
            created_at: model.created_at.assume_utc(),
        }
    }
}