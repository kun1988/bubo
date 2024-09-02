use axum::{debug_handler, extract::State, middleware, response::IntoResponse, routing::{get, post}, Extension, Json, Router};
use bubo::{controllers::{middlewares::auth::{self, AuthUser}, RemoveParams}, server::AppState, utils::error::BuboResult};

use serde_json::json;
use crate::{models::{_entities::admin_menu, menu::{AddMenuParams, EditMenuParams}}, views::menu::MenuResponse};


pub(crate) fn init_routes(state: AppState) -> Router {
    // 菜单
    Router::new()
     .route("/system/menu/list", get(menu_list)
     .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
     .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/menu/add", post(add_menu)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/menu/edit", post(edit_menu)
        .route_layer(middleware::from_fn_with_state(state.clone(),auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/menu/remove", post(remove_menu)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .with_state(state)
}



///
/// 菜单列表
/// 
#[debug_handler]
pub(crate) async fn menu_list(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
) -> BuboResult<impl IntoResponse> {
    let models = admin_menu::Model::list(&state.db).await?;
    // 转换返回对象
    let models: Vec<MenuResponse> = models.into_iter().map(|model| MenuResponse::new(model)).collect();

    let result = json!({
        "status": true,
        "data": models,
    });
    
    Ok(Json(result))
}

///
/// 新增菜单
/// 
#[debug_handler]
pub(crate) async fn add_menu(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<AddMenuParams>,
) -> BuboResult<impl IntoResponse> {

    let model = admin_menu::Model::add(&state.db, params, auth_user.id).await?;

    let result = json!({
        "status": true,
        "data": MenuResponse::new(model),
    });
    Ok(Json(result))
}

///
/// 编辑菜单
/// 
#[debug_handler]
pub(crate) async fn edit_menu(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<EditMenuParams>,
) -> BuboResult<impl IntoResponse> {
    let model = admin_menu::Model::edit(&state.db, params, auth_user.id).await?;
    
    let result = json!({
        "status": true,
        "data": MenuResponse::new(model),
    });
    Ok(Json(result))
}

///
/// 删除菜单
/// 
#[debug_handler]
pub(crate) async fn remove_menu(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<RemoveParams>,
) -> BuboResult<impl IntoResponse> {
    
    admin_menu::Model::remove(&state.db, params, auth_user.id).await?;

    let result = json!({
        "status": true,
    });
    Ok(Json(result))
}
