use admin_migration::Migrator;
use axum::Router;
use bubo::server::{AppState, Hooks};

mod controllers;
mod models;
mod views;

struct App;

impl Hooks for App {
    fn app_name() ->  &'static str {
        env!("CARGO_PKG_NAME")
    }

    fn router(state: AppState) -> Router {
        Router::new().nest("/admin", controllers::init_routes(state))
    }

    fn clean_up() {
    }
}

pub async fn main() {
    bubo::main::<App, Migrator>().await;
}




