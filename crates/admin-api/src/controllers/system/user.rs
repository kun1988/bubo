use axum::{debug_handler, extract::{Query, State}, middleware, response::IntoResponse, routing::{get, post}, Extension, Json, Router};
use bubo::{controllers::middlewares::auth::{self, AuthUser}, server::AppState, utils::error::BuboResult};
use serde_json::json;

use crate::{models::{_entities::admin_user, user::{AddUserParams, EditUserParams, UserPageParams}}, views::user::AdminUserResponse};


pub(crate) fn init_routes(state: AppState) -> Router {
    // 用户
    Router::new()
    .route("/system/user/page", get(user_page)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/user/add", post(add_user)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .route("/system/user/edit", post(edit_user)
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::permission))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth))
    )
    .with_state(state)
}

///
/// 用户分页
/// 
#[debug_handler]
pub(crate) async fn user_page(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    Query(params): Query<UserPageParams>
) -> BuboResult<impl IntoResponse> {

    let (models, num_pages) = admin_user::Model::page(&state.db, params).await?;

    // 转换返回对象
    let datas: Vec<AdminUserResponse> = models.into_iter().map(|model| AdminUserResponse::new(model)).collect();

    let result = json!({
        "status":  true,
        "data": datas,
        "num_pages": num_pages,
    });
    
    Ok(Json(result))
}

///
/// 新增用户
/// 
#[debug_handler]
pub(crate) async fn add_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<AddUserParams>,
) -> BuboResult<impl IntoResponse> {
    let model = admin_user::Model::add(&state.db, params, auth_user.id).await?;
    
    let result = json!({
        "status":  true,
        "data": AdminUserResponse::new(model),
    });
    Ok(Json(result))
}

///
/// 编辑用户
/// 
#[debug_handler]
pub(crate) async fn edit_user(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(params): Json<EditUserParams>,
) -> BuboResult<impl IntoResponse> {
    
    let model = admin_user::Model::edit(&state.db, params, auth_user.id).await?;

    let result = json!({
        "status":  true,
        "data": AdminUserResponse::new(model),
    });
    Ok(Json(result))
}

