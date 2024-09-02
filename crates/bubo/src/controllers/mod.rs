use serde::Deserialize;
use validator::Validate;
use crate::utils::{serde::to_vec_i64};

pub mod middlewares;

#[derive(Debug, Deserialize, Validate)]
pub struct RemoveParams {
    #[serde(deserialize_with = "to_vec_i64")]
    #[validate(length(min = 1, max = 100))]
    pub ids: Vec<i64>,
}