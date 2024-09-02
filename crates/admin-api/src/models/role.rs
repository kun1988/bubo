use std::collections::HashSet;
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use tracing::info;
use validator::Validate;
use crate::fill_active_model;
use super::{FillActiveModelTrait, _entities::{admin_role, admin_role_menu, prelude::{AdminRole, AdminRoleMenu}}};
use bubo::{controllers::RemoveParams, utils::{database::{ColOrd, EntityExtension}, error::{BuboError, BuboResult, BusinessErrorCode}, serde::{to_i64, to_set_i64}, snowflake, time::now_utc_primitive}};


fill_active_model!(admin_role::ActiveModel,admin_role_menu::ActiveModel);

#[derive(Debug, Deserialize)]
pub(crate) struct AddRoleParams {
    pub name: String,
    pub code: String,
    pub display_order: i16,
    pub state: i16,
    pub remark: String,
    #[serde(deserialize_with = "to_set_i64")]
    pub menu_ids: HashSet<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditRoleParams {
    #[serde(deserialize_with = "to_i64")]
    pub id: i64,
    pub name: String,
    pub code: String,
    pub display_order: i16,
    pub state: i16,
    pub remark: String,
    #[serde(deserialize_with = "to_set_i64")]
    pub menu_ids: HashSet<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RolePageParams {
    pub name: Option<String>,
    pub page: u16,
    pub page_size: u16,
}

impl admin_role::Model {
    pub(crate) async fn list(db: &DatabaseConnection) -> BuboResult<Vec<Self>> {
        //查询条件
        let condition = Condition::all();
        let col_ord_vec = vec![ColOrd::new(admin_role::Column::Id, sea_orm::Order::Desc)];
        let models = AdminRole::list(db, condition, col_ord_vec, Some(1000)).await?;

        Ok(models)
    }

    pub(crate) async fn page(db: &DatabaseConnection, params: RolePageParams) -> BuboResult<(Vec<Self>, u64)> {
        //查询条件
        let condition = Condition::all()
        .add_option(params.name.map(|name| admin_role::Column::Name.starts_with(name.as_str())));

        let col_ord_vec = vec![ColOrd::new(admin_role::Column::Id, sea_orm::Order::Desc)];

        let (models, num_pages) = AdminRole::fetch_page(db, params.page.into(), params.page_size.into(), condition,
            col_ord_vec).await?;
        Ok((models, num_pages))
    }

    pub(crate) async fn add(db: &DatabaseConnection, params: AddRoleParams, operator: i64) -> BuboResult<Self> {
        //判断角色编码是否唯一
        let condition = Condition::all().add(admin_role::Column::Code.eq(params.code.as_str()));
        // 开始事务
        let txn = db.begin().await?;
        
        let count = AdminRole::count( &txn, condition).await?;
        if count > 0 {
            return Err(BuboError::business_error(BusinessErrorCode::AlreadyExists, format!("Role code {} already exists", params.code)));
        }

        // 创建角色model
        let mut active_model = admin_role::ActiveModel {
            name: Set(params.name.clone()),
            code: Set(params.code.clone()),
            display_order: Set(params.display_order),
            state: Set(params.state),
            remark: Set(params.remark.clone()),
            ..Default::default()
        };
        active_model.fill_insert(Some(operator));

        
        // 保存角色
        let model = active_model.insert(&txn).await?;

        // 创建角色菜单model
        let role_menu_models = create_role_menu_model(model.id, params.menu_ids, operator);

        // 保存角色关联菜单
        if !role_menu_models.is_empty() {
            AdminRoleMenu::insert_many(role_menu_models).exec(&txn).await?;
        }

        // 提交事务
        txn.commit().await?;
        Ok(model)
    }

    pub(crate) async fn edit(db: &DatabaseConnection, params: EditRoleParams, operator: i64) -> BuboResult<Self> {
        let role = AdminRole::find_by_id(params.id).one(db).await?
        .ok_or(BuboError::business_error(BusinessErrorCode::NotFound, "角色不存在"))?;
        //判断角色编码是否唯一
        let condition = Condition::all().add(admin_role::Column::Code.eq(params.code.as_str()))
        .add(admin_role::Column::Id.ne(params.id))
        ;

        //创建角色更新model
        let mut active_model: admin_role::ActiveModel = role.into();
        active_model.name = Set(params.name.clone());
        active_model.code = Set(params.code.clone());
        active_model.display_order = Set(params.display_order);
        active_model.state = Set(params.state);
        active_model.remark = Set(params.remark.clone());
        active_model.fill_update(Some(operator));

        // 创建角色菜单model
        let role_menu_models = create_role_menu_model(params.id, params.menu_ids, operator);

        // 事务外创建对象，节省事务资源
        let txn = db.begin().await?;
        
        let count = AdminRole::count(&txn, condition).await?;
        if count > 0 {
            return Err(BuboError::business_error(BusinessErrorCode::AlreadyExists, format!("Role code {} already exists", params.code)));
        }

        // 更新角色
        let model = active_model.update(&txn).await?;

        // 删除角色绑定菜单
        let condition = Condition::all().add(admin_role_menu::Column::RoleId.eq(params.id));
        AdminRoleMenu::delete_many().filter(condition).exec(&txn).await?;
        
        // 保存角色关联菜单
        if !role_menu_models.is_empty() {
            AdminRoleMenu::insert_many(role_menu_models).exec(&txn).await?;
        }

        // 提交事务
        txn.commit().await?;
        Ok(model)
    }

    pub(crate) async fn remove(db: &DatabaseConnection, params: RemoveParams, operator: i64) -> BuboResult<()> {
        params.validate()?;
        // 查询要删除的角色
        let expr = admin_role::Column::Id.is_in(params.ids.clone());
        let admin_role_vec = AdminRole::find()
        .filter(expr.clone())
        .all(db)
        .await?;
        if admin_role_vec.is_empty() {
            return Err(BuboError::business_error(BusinessErrorCode::NotFound, "要删除的角色不存在"));
        }
        // 遍历取出角色id和名称
        let role_vec: Vec<(i64, String)> = admin_role_vec.iter().map(|x| (x.id, x.name.clone())).collect();

        // 删除角色
        let result = AdminRole::delete_many().filter(expr).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(BuboError::business_error(BusinessErrorCode::NotFound, "要删除的角色不存在"));
        }
        info!("operator:{}, delete role {:?}", operator, role_vec);
        Ok(())
    }

}

///
/// 创建角色菜单关联model
/// 
fn create_role_menu_model(role_id: i64, menu_ids: HashSet<i64>, operator: i64) -> Vec<admin_role_menu::ActiveModel> {
    let mut role_menus = Vec::new();
    if !menu_ids.is_empty() {
        let now = now_utc_primitive();
        for v in menu_ids.into_iter() {
            let active_model = admin_role_menu::ActiveModel {
                id: Set(snowflake::new_id()),
                role_id: Set(role_id),
                menu_id: Set(v),
                created_by: Set(operator),
                created_at: Set(now),
                ..Default::default()
            };
            role_menus.push(active_model);
        }
    }
    role_menus
}