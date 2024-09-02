use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use tracing::info;
use validator::Validate;
use crate::fill_active_model;
use super::{FillActiveModelTrait, _entities::{admin_menu, prelude::AdminMenu}};
use bubo::{controllers::RemoveParams, utils::{database::{ColOrd, EntityExtension}, error::{BuboError, BuboResult, BusinessErrorCode}, serde::to_i64}};


fill_active_model!(admin_menu::ActiveModel);

#[derive(Debug, Deserialize)]
pub(crate) struct AddMenuParams {
    pub name: String,
    #[serde(deserialize_with = "to_i64")]
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
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditMenuParams {
    #[serde(deserialize_with = "to_i64")]
    pub id: i64,
    pub name: String,
    #[serde(deserialize_with = "to_i64")]
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
}

impl admin_menu::Model {
    pub(crate) async fn list(db: &DatabaseConnection) -> BuboResult<Vec<Self>> {
        //查询条件
        let condition = Condition::all();

        let col_ord_vec = vec![ColOrd::new(admin_menu::Column::Id, sea_orm::Order::Desc)];
        let models = AdminMenu::list(db, condition, col_ord_vec, Some(1000)).await?;
        Ok(models)
    }

    pub(crate) async fn add(db: &DatabaseConnection, params: AddMenuParams, operator: i64) -> BuboResult<Self> {
        // 创建菜单model
        let mut active_model = admin_menu::ActiveModel {
            name: Set(params.name.clone()),
            parent_id: Set(params.parent_id),
            url: Set(params.url.clone()),
            target: Set(params.target),
            menu_type: Set(params.menu_type),
            is_hidden: Set(params.is_hidden),
            is_refresh: Set(params.is_refresh),
            permission: Set(params.permission.clone()),
            icon: Set(params.icon.clone()),
            display_order: Set(params.display_order),
            remark: Set(params.remark.clone()),
            ..Default::default()
        };
        active_model.fill_insert(Some(operator));
        // 保存菜单
        let model = active_model.insert(db).await?;
        Ok(model)
    }

    pub(crate) async fn edit(db: &DatabaseConnection, params: EditMenuParams, operator: i64) -> BuboResult<Self> {
        let menu = AdminMenu::find_by_id(params.id).one(db).await?
        .ok_or(BuboError::business_error(BusinessErrorCode::NotFound, "菜单不存在"))?;
        //创建菜单更新model
        let mut active_model: admin_menu::ActiveModel = menu.into();
        active_model.name = Set(params.name.clone());
        active_model.parent_id = Set(params.parent_id);
        active_model.url = Set(params.url.clone());
        active_model.target = Set(params.target);
        active_model.menu_type = Set(params.menu_type);
        active_model.is_hidden = Set(params.is_hidden);
        active_model.is_refresh = Set(params.is_refresh);
        active_model.permission = Set(params.permission.clone());
        active_model.icon = Set(params.icon.clone());
        active_model.display_order = Set(params.display_order);
        active_model.remark = Set(params.remark.clone());
        active_model.fill_update(Some(operator));

        // 更新菜单
        let model = active_model.update(db).await?;
        Ok(model)
    }

    pub(crate) async fn remove(db: &DatabaseConnection, params: RemoveParams, operator: i64) -> BuboResult<()> {
        params.validate()?;
        // 查询要删除的菜单
        let condition = Condition::all().add(admin_menu::Column::Id.is_in(params.ids.clone()));
        let admin_menu_vec = AdminMenu::find()
        .filter(condition.clone())
        .all(db)
        .await?;
        if admin_menu_vec.is_empty() {
            return Err(BuboError::business_error(BusinessErrorCode::NotFound, "menu not found"));
        }
        // 遍历取出菜单id和名称
        let menu_vec: Vec<(i64, String)> = admin_menu_vec.iter().map(|x| (x.id, x.name.clone())).collect();

        // 删除菜单
        let result = AdminMenu::delete_many().filter(condition).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(BuboError::business_error(BusinessErrorCode::NotFound, "menu not found"));
        }
        info!("operator: {}, delete menu {:?}", operator, menu_vec);
        Ok(())
    }
}