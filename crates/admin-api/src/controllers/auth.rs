use std::collections::{HashMap, HashSet};

use argon2::{password_hash::{rand_core::OsRng, SaltString}, Argon2, PasswordHash, PasswordVerifier, PasswordHasher};
use axum::{debug_handler, extract::State, middleware, response::IntoResponse, routing::{get, post}, Extension, Json, Router};
use bubo::{controllers::middlewares::auth::{self, create_token, AuthUser}, server::AppState, 
utils::{error::{BuboError, BuboResult, BusinessErrorCode, SystemErrorCode}, redis, time::now_utc_primitive, validator::JsonValid}, 
views::auth::AuthUserResponse};
use sea_orm::{ActiveModelTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, Set};
use serde::{Deserialize, Serialize};
use admin_migration::sea_orm::ColumnTrait;
use serde_json::json;
use sea_orm::QueryOrder;
use validator::Validate;

use crate::models::_entities::{admin_menu, admin_role, admin_role_menu, admin_user, admin_user_role, 
    prelude::{AdminMenu, AdminRoleMenu, AdminUser, AdminUserRole, AdminRole}};

pub(crate) fn init_routes(state: AppState) -> Router {
    Router::new()
        //登录
        .route("/auth/login/account", post(account_login_handler))
        .route("/auth/refresh-token",post(refresh_token_handler)
            .route_layer(middleware::from_fn_with_state(state.clone(),auth::refresh))
        )
        .route("/auth/logout",post(logout_handler)
            .route_layer(middleware::from_fn_with_state(state.clone(),auth::auth))
        )
        .route("/auth/user-info",get(user_info_handler)
            .route_layer(middleware::from_fn_with_state(state.clone(),auth::auth))
        )
        .route("/auth/user-routes", get(user_routes_handler)
            .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
        )
        .route("/auth/change-pwd", post(change_password_handler)
            .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
        )
        .with_state(state)
}

#[derive(Debug, Deserialize, Validate)]
pub(crate) struct LoginUserParams {
    #[validate(length(min = 3, max = 20))]
    username: String,
    #[validate(length(equal = 64))]
    password: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChangePasswordParams {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize)]
struct Route {
    pub path: String,
    pub name: String,
    pub icon: String,
    pub routes: Vec<Route>,
}

///
/// 帐号登录
/// 
#[debug_handler]
pub(crate) async fn account_login_handler(
    State(state): State<AppState>,
    JsonValid(params): JsonValid<LoginUserParams>,
) -> BuboResult<impl IntoResponse> {
    let admin_user_model: Option<admin_user::Model> = AdminUser::find()
        .filter(admin_user::Column::Username.eq(params.username))
        .filter(admin_user::Column::State.eq(1))
        .filter(admin_user::Column::IsDeleted.eq(false))
        // .filter(AdminUserColumn::Password.eq(create_md5(&body.password)))
        .one(&state.db)
        .await?;

    match admin_user_model {
        Some(admin_user) => {
            let is_valid = match PasswordHash::new(&admin_user.password) {
                Ok(parsed_hash) => Argon2::default()
                    .verify_password(params.password.as_bytes(), &parsed_hash)
                    .is_ok(),
                Err(_) => false,
            };
            if !is_valid {
                return Err(BuboError::business_error(BusinessErrorCode::UserOrPasswordNotMatch, "用户名或密码错误"));
            }

            let (roles, permissions, menu_ids) = get_user_roles_and_permissions(&state.db, admin_user.id).await?;
            let auth_user = AuthUser::new(admin_user.id, admin_user.username, admin_user.nick_name, admin_user.is_admin, 0, 
                0, roles, permissions, menu_ids);
            let (access_token, refresh_token, token_type, expires_in) = create_token(&state, auth_user).await?;

            let result = json!({
                "status":  true,
                "access_token": access_token,
                "refresh_token": refresh_token,
                "token_type": token_type, 
                "expires_in": expires_in,
            });
            Ok(Json(result))
        }
        None => {
            Err(BuboError::business_error(BusinessErrorCode::UserOrPasswordNotMatch, "用户名或密码错误"))
        }
    }
}

///
/// 刷新令牌
/// 
#[debug_handler]
pub(crate) async fn refresh_token_handler(
    State(state): State<AppState>, 
    Extension(auth_user): Extension<AuthUser>
) -> BuboResult<impl IntoResponse> {
    // let mut new_auth_user = auth_user.clone();
    let (access_token, refresh_token, token_type, expires_in) = create_token(&state, auth_user).await?;
    
    let result = json!({
        "status":  true,
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": token_type,
        "expires_in": expires_in,
    });
    Ok(Json(result))
}

///
/// 退出登录
/// 
#[debug_handler]
pub(crate) async fn logout_handler(
    State(state): State<AppState>, 
    Extension(auth_user): Extension<AuthUser>
) -> BuboResult<impl IntoResponse> {
    let key = redis::gen_key(state.app_name, "adminlogin", auth_user.id);
    redis::del(&state.redis, key.as_str()).await?;
    let result = json!({
        "status":  true,
    });
    Ok(Json(result))
}

///
/// 获取用户信息
/// 
#[debug_handler]
pub(crate) async fn user_info_handler(
    Extension(auth_user): Extension<AuthUser>
) -> BuboResult<impl IntoResponse> {
    let result = json!({
        "status":  true,
        "data": AuthUserResponse::new(auth_user),
    });
    Ok(Json(result))
}

///
/// 获取用户路由
/// 
#[debug_handler]
pub(crate) async fn user_routes_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
) -> BuboResult<impl IntoResponse> {
    let mut datas = Vec::new();
    if auth_user.is_admin {
        // 管理员查询全部菜单
        let condition = Condition::all()
        .add(admin_menu::Column::IsHidden.eq(false));
        let models = AdminMenu::find().filter(condition).order_by_asc(admin_menu::Column::DisplayOrder).all(&state.db).await?;
        
        if !models.is_empty() {
            let mut parent_models = HashMap::new();
            for model in models.into_iter() {
                let mut vec = parent_models.remove(&model.parent_id).unwrap_or(Vec::new());
                let pid = model.parent_id.clone();
                vec.push(model);
                parent_models.insert(pid, vec);
            }
            datas = get_child_routes(&mut parent_models, 0);
        }
    } else if !auth_user.menu_ids.is_empty() {
        // 查询菜单
        let condition = Condition::all().add(admin_menu::Column::Id.is_in(auth_user.menu_ids))
        .add(admin_menu::Column::IsHidden.eq(false));
        let models = AdminMenu::find().filter(condition).order_by_asc(admin_menu::Column::DisplayOrder).all(&state.db).await?;
        
        if !models.is_empty() {
            let mut parent_models = HashMap::new();
            for model in models.into_iter() {
                let mut vec = parent_models.remove(&model.parent_id).unwrap_or(Vec::new());
                let pid = model.parent_id.clone();
                vec.push(model);
                parent_models.insert(pid, vec);
            }
            datas = get_child_routes(&mut parent_models, 0);
        }
    }
    
    let result = json!({
        "status":  true,
        "data": datas,
    });
    Ok(Json(result))
}

///
/// 获取子路由
/// 
fn get_child_routes(menus: &mut HashMap<i64, Vec<admin_menu::Model>>, parent_id: i64) -> Vec<Route> {
    let mut routes = Vec::new();
    if let Some(models) = menus.remove(&parent_id) {
        for model in models.into_iter() {
            let route = Route { 
                path: model.url, 
                name: model.name, 
                icon: model.icon, 
                routes: get_child_routes(menus, model.id),
            };
            routes.push(route)
        }
    }
    routes
}

///
/// 修改密码
/// 
#[debug_handler]
pub(crate) async fn change_password_handler(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<ChangePasswordParams>
) -> BuboResult<impl IntoResponse> {
    let admin_user_model: Option<admin_user::Model> = AdminUser::find_by_id(auth_user.id).one(&state.db).await?;

    match admin_user_model {
        Some(admin_user) => {
            let is_valid = match PasswordHash::new(&admin_user.password) {
                Ok(parsed_hash) => Argon2::default()
                    .verify_password(params.old_password.as_bytes(), &parsed_hash)
                    .is_ok(),
                Err(_) => false,
            };
            if !is_valid {
                return Err(BuboError::business_error(BusinessErrorCode::PasswordNotMatch, "旧密码错误"));
            }
            
            let salt = SaltString::generate(&mut OsRng);
            let argon2 = Argon2::default();
            let password_hash = argon2.hash_password(params.new_password.as_bytes(), &salt)
            .map_err(|_| BuboError::system_error(SystemErrorCode::Argon2HashError, "argon2 hash error"))?.to_string();
            let active_model = admin_user::ActiveModel {
                id: Set(auth_user.id.clone()),
                password: Set(password_hash),
                updated_by: Set(auth_user.id),
                updated_at: Set(now_utc_primitive()),
                ..Default::default()
            };

            active_model.update(&state.db).await?;
        }
        None => {
            return Err(BuboError::system_error(SystemErrorCode::UnknownError, "why user not exists?"));
        }
    }

    let result = json!({
        "status": true,
    });
    Ok(Json(result))
}

// 获取用户的角色和权限
pub(crate) async fn get_user_roles_and_permissions(
    db: &DatabaseConnection,
    user_id: i64,
) -> BuboResult<(HashSet<String>, HashSet<String>, HashSet<i64>)> {
    // 查询用户绑定的角色
    let condition = Condition::all().add(admin_user_role::Column::UserId.eq(user_id));
    let role_ids: Vec<i64> = AdminUserRole::find().select_only().column(admin_user_role::Column::RoleId).filter(condition).into_tuple().all(db).await?;

    // 查询角色id和编码
    let condition = Condition::all().add(admin_role::Column::Id.is_in(role_ids))
    .add(admin_role::Column::State.eq(1));
    let roles: Vec<(i64, String)> = AdminRole::find().select_only().column(admin_role::Column::Id).column(admin_role::Column::Code)
    .filter(condition).into_tuple().all(db).await?;
    
    let mut role_ids = HashSet::new();
    let mut role_codes = HashSet::new();
    let mut menu_ids = HashSet::new();
    let mut permissions = HashSet::new();

    // 如果角色不为空，继续查询菜单权限
    if !roles.is_empty() {
        for role in roles.into_iter() {
            role_ids.insert(role.0);
            role_codes.insert(role.1);
        }
    
        // 查询角色绑定的菜单
        let condition = Condition::all().add(admin_role_menu::Column::RoleId.is_in(role_ids));
        let mids: Vec<i64> = AdminRoleMenu::find().select_only().column(admin_role_menu::Column::MenuId).filter(condition).into_tuple().all(db).await?;
        // 去重
        let mids: HashSet<i64> = mids.into_iter().collect();

        // 查询菜单权限
        let condition = Condition::all().add(admin_menu::Column::Id.is_in(mids))
        .add(admin_menu::Column::IsHidden.eq(false));
        let menus: Vec<(i64, String)> = AdminMenu::find().select_only().column(admin_menu::Column::Id).column(admin_menu::Column::Permission)
        .filter(condition).into_tuple().all(db).await?;
        if !menus.is_empty() {
            for menu in menus.into_iter() {
                menu_ids.insert(menu.0);
                if !menu.1.is_empty() {
                    permissions.insert(menu.1);
                }
            }
        }
    }

    Ok((role_codes, permissions, menu_ids))
}