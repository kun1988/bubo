use axum::Router;
use bubo::server::AppState;

pub(crate) mod menu;
pub(crate) mod role;
pub(crate) mod user;

pub(crate) fn init_routes(state: AppState) -> Router {
    Router::new()
    .merge(menu::init_routes(state.clone()))
    .merge(role::init_routes(state.clone()))
    .merge(user::init_routes(state.clone()))
}