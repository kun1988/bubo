use std::time::Duration;

use fred::{prelude::{ClientLike, KeysInterface, RedisPool}, types::{Builder, Expiration, ReconnectPolicy, RedisConfig}};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use super::error::BuboResult;


///
/// 初始化redis
/// 
pub async fn init() -> RedisPool {
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL is not set in .env file");
    let pool_size = std::env::var("REDIS_POOL_SIZE")
    .ok()
    .and_then(|v| v.parse::<usize>().ok())
    .unwrap_or(8);
    let config = RedisConfig::from_url(&redis_url).expect("Failed to create redis config from url");
    let pool = Builder::from_config(config)
    .with_connection_config(|config| {
      config.connection_timeout = Duration::from_secs(10);
    })
    // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
    .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
    .build_pool(pool_size)
    .expect("Failed to create redis pool");

    pool.init().await.expect("Failed to connect to redis");
    info!("Connected to Redis");
    pool
}

///
/// 生成通用的key格式
/// 
pub fn gen_key(app_name: impl AsRef<str>, biz: impl AsRef<str>, id: i64) -> String {
    format!("{}:{}:{}", app_name.as_ref(), biz.as_ref(), id)
}

pub async fn get_string(redis: &RedisPool, key: impl AsRef<str>) -> BuboResult<Option<String>> {
    Ok(redis.get(key.as_ref()).await?)
}

pub async fn get<T>(redis: &RedisPool, key: impl AsRef<str>) -> BuboResult<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let value_option: Option<Value> = redis.get(key.as_ref()).await?;
    if let Some(value) = value_option {
        let t: T = serde_json::from_value(value)?;
        return Ok(Some(t));
    }
    Ok(None)
}

pub async fn set<T>(redis: &RedisPool, key: impl AsRef<str>, t: &T, expire: Option<Expiration>) -> BuboResult<()>
where 
    T: ?Sized + Serialize,
{
    redis.set(key.as_ref(), serde_json::to_string(t)?, expire, None, false).await?;
    Ok(())
}

pub async fn del(redis: &RedisPool, key: impl AsRef<str>) -> BuboResult<()> {
    redis.del(key.as_ref()).await?;
    Ok(())
}