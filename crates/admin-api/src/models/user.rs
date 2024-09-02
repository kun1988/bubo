use std::collections::HashSet;

use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHasher};
use bubo::utils::{database::ColOrd, error::{BuboError, BuboResult, BusinessErrorCode}, snowflake, time::now_utc_primitive};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use validator::Validate;

use crate::fill_active_model;

use super::{FillActiveModelTrait, _entities::{admin_user, admin_user_role, prelude::{AdminUser, AdminUserRole}}};
use bubo::utils::{serde::{to_i64, to_set_i64}, database::EntityExtension};



fill_active_model!(admin_user::ActiveModel,admin_user_role::ActiveModel);

#[derive(Debug, Deserialize, Default, Validate)]
pub(crate) struct AddUserParams {
    #[validate(length(min = 3, max = 20))]
    username: String,
    #[validate(length(min = 1, max = 20))]
    nick_name: String,
    #[validate(email)]
    email: String,
    #[validate(length(equal = 11))]
    phone_number: String,
    #[validate(range(min=0, max=2))]
    gender: i16,
    #[validate(length(equal = 64))]
    password: String,
    #[validate(length(max = 100))]
    remark: String,
    #[serde(deserialize_with = "to_set_i64")]
    role_ids: HashSet<i64>,
}

#[derive(Debug, Deserialize, Default, Validate)]
pub(crate) struct EditUserParams {
    #[serde(deserialize_with = "to_i64")]
    pub id: i64,
    #[validate(length(min = 1, max = 20))]
    pub nick_name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(equal = 11))]
    pub phone_number: String,
    #[validate(range(min=0, max=2))]
    pub gender: i16,
    #[validate(length(max = 100))]
    pub remark: String,
    #[serde(deserialize_with = "to_set_i64")]
    pub role_ids: HashSet<i64>,
}

#[derive(Debug, Deserialize, Validate)]
pub(crate) struct UserPageParams {
    username: Option<String>,
    #[validate(range(min=1))]
    page: u64,
    #[validate(range(min=1))]
    page_size: u64,
}

impl admin_user::Model {

    ///
    /// 新增后台用户
    /// 
    pub(crate) async fn add(db: &DatabaseConnection, params: AddUserParams, operator: i64) -> BuboResult<Self> {
        params.validate()?;
        //判断用户名是否唯一
        let condition = Condition::all().add(admin_user::Column::Username.eq(&params.username));

        // 开始事务
        let txn = db.begin().await?;
        let count = AdminUser::count(&txn, condition).await?;
        if count > 0 {
            return Err(BuboError::business_error(BusinessErrorCode::AlreadyExists, format!("User username {}", params.username)));
        }

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(params.password.as_bytes(), &salt)?.to_string();

        //params to ActiveModel
        let mut active_model = admin_user::ActiveModel {
            username: Set(params.username.clone()),
            nick_name: Set(params.nick_name.clone()),
            password: Set(password_hash),
            email: Set(params.email.clone()),
            phone_number: Set(params.phone_number.clone()),
            gender: Set(params.gender),
            state: Set(AdminUserState::Normal as i16),
            is_admin: Set(false),
            is_deleted: Set(false),
            remark: Set(params.remark.clone()),
            ..Default::default()
        };
        active_model.fill_insert(Some(operator));
        let model = active_model.insert(&txn).await?;

        // 创建用户角色model
        let user_role_models = create_user_role_model(model.id, params.role_ids, operator);

        // 保存用户角色菜单
        if !user_role_models.is_empty() {
            AdminUserRole::insert_many(user_role_models).exec(&txn).await?;
        }
        // 提交事务
        txn.commit().await?;
        Ok(model)
    }

    ///
    /// 编辑后台用户
    /// 
    pub(crate) async fn edit(db: &DatabaseConnection, params: EditUserParams, operator: i64) -> BuboResult<Self> {
        params.validate()?;
        // 开始事务
        let txn = db.begin().await?;
        let user = AdminUser::find_by_id(params.id).one(&txn).await?
            .ok_or(BuboError::business_error(BusinessErrorCode::NotFound, "用户不存在"))?;

        let mut active_model : admin_user::ActiveModel = user.into();
        active_model.nick_name = Set(params.nick_name.clone());
        active_model.email = Set(params.email.clone());
        active_model.phone_number = Set(params.phone_number.clone());
        active_model.gender = Set(params.gender);
        active_model.remark = Set(params.remark.clone());
        active_model.fill_update(Some(operator));

        // 创建用户角色model
        let user_role_models = create_user_role_model(params.id, params.role_ids, operator);
        let model = active_model.update(&txn).await?;

        // 删除用户绑定角色
        let condition = Condition::all().add(admin_user_role::Column::UserId.eq(params.id));
        AdminUserRole::delete_many().filter(condition).exec(&txn).await?;
        
        // 保存用户关联角色
        if !user_role_models.is_empty() {
            AdminUserRole::insert_many(user_role_models).exec(&txn).await?;
        }
        // 提交事务
        txn.commit().await?;
        Ok(model)
    }

    ///
    /// 后台用户分页
    /// 
    pub(crate) async fn page<'a, C: ConnectionTrait>(db: &'a C, params: UserPageParams) -> BuboResult<(Vec<admin_user::Model>, u64)> {
        params.validate()?;
        //查询条件
        let condition = Condition::all()
            .add_option(params.username.map(|username| admin_user::Column::Username.starts_with(username.as_str())))
            .add(admin_user::Column::IsAdmin.eq(false));
        let col_ord_vec = vec![ColOrd::new(admin_user::Column::Id, sea_orm::Order::Desc)];
        let (models, num_pages) = AdminUser::fetch_page(db, params.page.into(), params.page_size.into(), condition, 
        col_ord_vec).await?;
        

        Ok((models, num_pages))
    }
}

///
/// 创建用户角色关联model
/// 
fn create_user_role_model(user_id: i64, role_ids: HashSet<i64>, operator: i64) -> Vec<admin_user_role::ActiveModel> {
    let mut role_menus = Vec::new();
    if !role_ids.is_empty() {
        let now = now_utc_primitive();
        for role_id in role_ids.into_iter() {
            let active_model = admin_user_role::ActiveModel {
                id: Set(snowflake::new_id()),
                user_id: Set(user_id),
                role_id: Set(role_id),
                created_by: Set(operator),
                created_at: Set(now),
                ..Default::default()
            };
            role_menus.push(active_model);
        }
    }
    role_menus
}

///
/// 后台用户状态
/// 
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum AdminUserState {
    // 未知
    Unknown = 0,
    // 正常
    Normal,
    // 禁用
    Disable,
}

///
/// 性别
/// 
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Gender {
    // 未知
    Unknown = 0,
    // 男
    Male,
    // 女
    Female,
}