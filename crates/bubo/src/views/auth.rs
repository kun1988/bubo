use std::collections::HashSet;

use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

use crate::controllers::middlewares::auth::AuthUser;

#[serde_as]
#[derive(Debug, Serialize)]
pub struct AuthUserResponse {
    #[serde_as(as = "DisplayFromStr")]
    pub id: i64,
    pub username: String,
    pub nick_name: String,
    pub is_admin: bool,
    pub roles: HashSet<String>,
    pub permissions: HashSet<String>,
}

impl AuthUserResponse {
    pub fn new(value: AuthUser) -> Self {
        Self { 
            id: value.id, 
            username: value.username, 
            nick_name: value.nick_name, 
            is_admin: value.is_admin, 
            roles: value.roles, 
            permissions: value.permissions, 
        }
    }
}