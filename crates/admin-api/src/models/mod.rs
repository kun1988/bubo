pub(crate) mod _entities;
pub(crate) mod user;
pub(crate) mod role;
pub(crate) mod menu;

pub(crate) trait FillActiveModelTrait {
    fn fill_insert(&mut self, operator: Option<i64>);
    fn fill_update(&mut self, operator: Option<i64>);
}

#[macro_export]
macro_rules! fill_active_model {
    ($($ty: ty),*) => {
        $(
        impl crate::models::FillActiveModelTrait for $ty {
            fn fill_insert(&mut self, operator: Option<i64>) {
                if self.id.is_not_set() {
                    let id = bubo::utils::snowflake::new_id();
                    self.id = sea_orm::ActiveValue::Set(id);
                }
                self.created_by = sea_orm::ActiveValue::Set(operator.unwrap_or(0));
                self.created_at = sea_orm::ActiveValue::Set(bubo::utils::time::now_utc_primitive());
            }

            fn fill_update(&mut self, operator: Option<i64>) {
                self.updated_by = sea_orm::ActiveValue::Set(operator.unwrap_or(0));
                self.updated_at = sea_orm::ActiveValue::Set(bubo::utils::time::now_utc_primitive());
            }
        }
        )*
    };
}