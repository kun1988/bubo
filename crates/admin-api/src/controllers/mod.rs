use axum::Router;
use bubo::server::AppState;

mod auth;
mod system;

pub(crate) fn init_routes(state: AppState) -> Router {
    Router::new()
    .merge(auth::init_routes(state.clone()))
    .merge(system::init_routes(state.clone()))
}