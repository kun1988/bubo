use axum::{debug_handler, extract::{Query, State}, middleware, response::IntoResponse, routing::{get, post}, Extension, Json, Router};
use bubo::{controllers::{middlewares::auth::{self, AuthUser}, RemoveParams}, server::AppState, utils::error::BuboResult};
use serde_json::json;

use crate::{models::{_entities::admin_role, role::{AddRoleParams, EditRoleParams, RolePageParams}}, views::role::RoleResponse};


pub(crate) fn init_routes(state: AppState) -> Router {
    // 角色
    Router::new()
    .route("/system/role/list", get(role_list)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/role/page", get(role_page)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/role/add", post(add_role)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/role/edit", post(edit_role)
        .route_layer(middleware::from_fn_with_state(state.clone(),auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/role/remove", post(remove_role)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .with_state(state)
}

///
/// 角色列表
/// 
#[debug_handler]
pub(crate) async fn role_list(
    State(state): State<AppState>,
    // Extension(_auth_user): Extension<AuthUser>,
) -> BuboResult<impl IntoResponse> {

    let models = admin_role::Model::list(&state.db).await?;
    // 转换返回对象
    let datas: Vec<RoleResponse> = models.into_iter().map(|model| RoleResponse::new(model)).collect();

    let result = json!({
        "status":  true,
        "data": datas,
    });
    
    Ok(Json(result))
}

///
/// 角色分页
/// 
#[debug_handler]
pub(crate) async fn role_page(
    State(state): State<AppState>,
    // Extension(auth_user): Extension<AuthUser>,
    Query(params): Query<RolePageParams>
) -> BuboResult<impl IntoResponse> {

    let (models, num_pages) = admin_role::Model::page(&state.db, params).await?;

    // 转换返回对象
    let datas: Vec<RoleResponse> = models.into_iter().map(|model| RoleResponse::new(model)).collect();

    let result = json!({
        "status":  true,
        "data": datas,
        "num_pages": num_pages,
    });
    
    Ok(Json(result))
}

///
/// 新增角色
/// 
#[debug_handler]
pub(crate) async fn add_role(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<AddRoleParams>,
) -> BuboResult<impl IntoResponse> {
    
    let model = admin_role::Model::add(&state.db, params, auth_user.id).await?;

    let result = json!({
        "status": true,
        "data": RoleResponse::new(model),
    });
    Ok(Json(result))
}

///
/// 编辑角色
/// 
#[debug_handler]
pub(crate) async fn edit_role(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<EditRoleParams>,
) -> BuboResult<impl IntoResponse> {
    let model = admin_role::Model::edit(&state.db, params, auth_user.id).await?;

    let result = json!({
        "status":  true,
        "data": RoleResponse::new(model),
    });
    Ok(Json(result))
}

///
/// 删除角色
/// 
#[debug_handler]
pub(crate) async fn remove_role(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<RemoveParams>,
) -> BuboResult<impl IntoResponse> {
    
    admin_role::Model::remove(&state.db, params, auth_user.id).await?;

    let result = json!({
        "status":  true,
    });
    Ok(Json(result))
}