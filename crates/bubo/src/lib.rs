use sea_orm_migration::MigratorTrait;
use server::Hooks;

pub mod server;
pub mod utils;
pub mod controllers;
pub mod views;

pub async fn main<H: Hooks, M: MigratorTrait>() {
    server::main::<H, M>().await;
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
