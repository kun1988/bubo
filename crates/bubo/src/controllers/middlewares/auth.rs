use std::collections::HashSet;

use axum::{extract::{Request, State}, middleware::Next, response::IntoResponse, Extension};
use axum_extra::{headers::{authorization::Bearer, Authorization}, TypedHeader};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tracing::{debug, warn};

use crate::{server::AppState, utils::{error::{BuboError, BuboResult, BusinessErrorCode, SystemErrorCode}, redis, snowflake, time::{current_timestamp_sec, now_utc}}};

pub const TOKEN_TYPE: &str = "Bearer";
pub const ACCESS_TYPE: &str = "ACCESS";
pub const REFRESH_TYPE: &str = "REFRESH";
pub const ACCESS_EXP: i64 = 7200;
pub const REFRESH_EXP: i64 = 604800;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub nick_name: String,
    pub is_admin: bool,
    pub access_token_id: i64,
    pub refresh_token_id: i64,
    pub last_access_token_id: Option<i64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub refreshed_at: Option<OffsetDateTime>,
    pub roles: HashSet<String>,
    pub permissions: HashSet<String>,
    pub menu_ids: HashSet<i64>,
}

impl AuthUser {
    pub fn new(id: i64, username: impl Into<String>, nick_name: impl Into<String>, is_admin: bool, access_token_id: i64, 
        refresh_token_id: i64, roles: HashSet<String>, permissions: HashSet<String>, menu_ids: HashSet<i64>) -> Self {
        AuthUser { 
            id: id, 
            username: username.into(), 
            nick_name: nick_name.into(), 
            is_admin, 
            access_token_id,
            refresh_token_id,
            last_access_token_id: None,
            refreshed_at: None,
            roles,
            permissions,
            menu_ids,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    // 主题
    pub sub: i64,
    // 签发时间
    pub iat: i64,
    // 过期时间
    pub exp: i64,
    // 受众
    pub aud: String,
    // 签发人
    pub iss: String,
    // 编号
    pub jti: i64,
}

pub async fn refresh(
    TypedHeader(authorization): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> BuboResult<impl IntoResponse> {
    let token = authorization.token();
    let auth_user = auth_token(state.clone(), token, REFRESH_TYPE).await?;
    req.extensions_mut().insert(auth_user);
    let result = next.run(req).await;
    Ok(result)
}

pub async fn auth(
    TypedHeader(authorization): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> BuboResult<impl IntoResponse> {
    let token = authorization.token();
    let auth_user = auth_token(state.clone(), token, ACCESS_TYPE).await?;
    req.extensions_mut().insert(auth_user);
    let result = next.run(req).await;
    Ok(result)
}

pub async fn  auth_token(
    state: AppState,
    token: &str,
    token_type: &str,
) -> BuboResult<AuthUser> {
    let mut key = state.jwt_secret_access.as_bytes();
    if token_type == REFRESH_TYPE {
        key = state.jwt_secret_refresh.as_bytes();
    }
    let mut validation = Validation::default();
    validation.set_audience(&[state.app_name]);
    // 验证jwt token
    let claims = decode::<Claims>(token, &DecodingKey::from_secret(key), &validation)
        .map_err(|e| {
            warn!("jwt decode error:{:?}", e);
            BuboError::business_error(BusinessErrorCode::Unauthorized, "unauthorized")
        })?.claims;
    let key = redis::gen_key(state.app_name, "auth-user", claims.sub);

    let auth_user: AuthUser = redis::get(&state.redis, key).await?.ok_or(BuboError::business_error(BusinessErrorCode::Unauthorized, "unauthorized"))?;
    if token_type == ACCESS_TYPE {
        // 5分钟内旧access_token可以使用
        if auth_user.access_token_id != claims.jti 
        && !auth_user.last_access_token_id.is_some_and(|x| { 
            x == claims.jti && auth_user.refreshed_at.unwrap().unix_timestamp() + 300 > current_timestamp_sec()
        }) {
            warn!("access token not equal");
            return Err(BuboError::business_error(BusinessErrorCode::Unauthorized, "unauthorized"));
        }
        
    } else if token_type == REFRESH_TYPE && auth_user.refresh_token_id != claims.jti {
        warn!("refresh token not equal");
        return Err(BuboError::business_error(BusinessErrorCode::Unauthorized, "unauthorized"));
    }

    Ok(auth_user)
}

pub fn encode_token(id: i64, aud: impl Into<String>, iss: impl Into<String>, jti: i64, exp: i64, key: &[u8]) -> BuboResult<String> {
    // let mut exp = 7200;
    // if token_type == REFRESH_TYPE {
    //     exp = 604800;
    // }
    let now = now_utc();
    let exp = (now + Duration::seconds(exp)).unix_timestamp();
    let claims = Claims { sub: id, iat: (now + Duration::seconds(7200)).unix_timestamp(), 
        exp, aud: aud.into(), iss: iss.into(), jti };
    
    encode(&Header::default(), &claims, &EncodingKey::from_secret(key))
        .map_err(|_e| BuboError::system_error(SystemErrorCode::JwtEncodeError, "jwt claims encode error"))
}

pub async fn create_token(state: &AppState, mut auth_user: AuthUser) -> BuboResult<(String, String, &'static str, i64)> {
    if auth_user.access_token_id != 0 {
        auth_user.last_access_token_id = Some(auth_user.access_token_id);
        auth_user.refreshed_at = Some(now_utc());
    }
    auth_user.access_token_id = snowflake::new_id();
    let access_token = encode_token(auth_user.id, state.app_name, state.app_name, auth_user.access_token_id, ACCESS_EXP, state.jwt_secret_access.as_bytes())?;
    auth_user.refresh_token_id = snowflake::new_id();
    let refresh_token = encode_token(auth_user.id, state.app_name, state.app_name, auth_user.refresh_token_id, REFRESH_EXP, state.jwt_secret_refresh.as_bytes())?;
    
    let key = redis::gen_key(state.app_name, "auth-user", auth_user.id);
    
    redis::set(&state.redis, key, &auth_user, Some(fred::types::Expiration::EX(604800))).await?;
    Ok((access_token, refresh_token, TOKEN_TYPE, ACCESS_EXP))
}

///
///  权限验证
/// 
pub async fn permission(
    // State(state): State<Arc<AppState>>,
    Extension(auth_user): Extension<AuthUser>,
    req: Request,
    next: Next,
) -> BuboResult<impl IntoResponse> {
    // 不是管管理员需验证权限
    if !auth_user.is_admin {
        let mut permission = req.uri().path().replace("/", ":");
        if !permission.is_empty() {
            permission.remove(0);
        }
        debug!("permission:{}", permission);
        if !auth_user.permissions.contains(permission.as_str()) {
            return Err(BuboError::business_error(BusinessErrorCode::Forbidden, "forbidden"));
        }
    }
    Ok(next.run(req).await)
}