use fred::{prelude::RedisPool, types::Expiration};
use sea_orm::{entity::*, query::*, sea_query::SimpleExpr, Condition, Database, DatabaseConnection, QueryFilter};
use sea_orm_migration::MigratorTrait;
use serde::{Deserialize, Serialize};
use tracing::info;
use async_trait::async_trait;

use crate::{server::AppState, utils::redis};

use super::error::BuboResult;

const EXPIRE: i64 = 60 * 5;

pub struct ColOrd {
    col: SimpleExpr,
    ord: Order,
}

impl ColOrd {
    pub fn new(col: impl IntoSimpleExpr, ord: Order) -> Self{
        Self { 
            col: col.into_simple_expr(), 
            ord, 
        }
    }
}

///
/// 初始化数据库
/// 
pub async fn init<M: MigratorTrait>() -> DatabaseConnection {
    let db_url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let db: DatabaseConnection = Database::connect(db_url)
        .await
        .expect("Database connection failed");
    info!("Connected to Database");
    M::up(&db, None).await.unwrap();
    db 
}


#[async_trait]
pub trait EntityExtension: EntityTrait{
    async fn list<'a, C: ConnectionTrait>(
        db: &'a C,
        condition: Condition,
        col_ord_vec: Vec<ColOrd>,
        limit: Option<u64>,
    ) -> BuboResult<Vec<Self::Model>>
        where
            <Self as EntityTrait>::Model: Send + Unpin + Sync,
    {
        let mut query = Self::find().filter(condition);
        for v in col_ord_vec.into_iter() {
            query = query.order_by(v.col, v.ord);
        }
        if let Some(n) = limit {
            query = query.limit(n);
        }
        let models = query.all(db).await?;

        Ok(models)
    }
    async fn fetch_page<'a, C: ConnectionTrait>(
        db: &'a C,
        page: u64,
        page_size: u64,
        condition: Condition,
        col_ord_vec: Vec<ColOrd>,
    ) -> BuboResult<(Vec<Self::Model>, u64)>
        where
            <Self as EntityTrait>::Model: Send + Unpin + Sync,
    {
        let mut query = Self::find().filter(condition);
        for v in col_ord_vec.into_iter() {
            query = query.order_by(v.col, v.ord);
        }
        let paginator = query.paginate(db, page_size);
        let num_pages = paginator.num_pages().await?;

        let models = if num_pages < page {
            vec![]
        } else {
            paginator.fetch_page(page - 1).await?
        };

        Ok((models, num_pages))
    }

    #[allow(dead_code)]
    async fn cache_get(state: AppState, id: i64) -> BuboResult<Option<Self::Model>>
    where
        <Self as sea_orm::EntityTrait>::Model: Send + Unpin + Sync,
        for<'de> Self::Model: Deserialize<'de> + Serialize,
        <Self::PrimaryKey as PrimaryKeyTrait>::ValueType: From<i64>,

    {
        let entity = Self::default();
        let cache_key = redis::gen_key(state.app_name, format!("cache-{}", Self::table_name(&entity)), id);
        tracing::debug!("get: {}", cache_key);
        let mut model = redis::get(&state.redis, &cache_key).await?;
        if model.is_none() {
            model = match Self::find_by_id(id).one(&state.db).await {
                Ok(v) => match v {
                    Some(m) => {
                        redis::set(&state.redis, &cache_key, &m, Some(fred::types::Expiration::EX(EXPIRE))).await?;
                        Some(m)
                    }
                    None => None,
                },
                Err(e) => return Err(e.into()),
            };
        }
        Ok(model)
    }

    // 根据primary key 清除缓存
    #[allow(dead_code)]
    async fn clear_cache(state: AppState, id: i64) -> BuboResult<()> 
    {
        let entity = Self::default();
        let cache_key = redis::gen_key(state.app_name, format!("cache-{}", Self::table_name(&entity)), id);
        tracing::debug!("clear_cache: {}", cache_key);
        redis::del(&state.redis, &cache_key).await?;
        Ok(())
    }

    async fn count<'a, C: ConnectionTrait>(db: &'a C, condition: Condition) -> BuboResult<u64>
    where
        <Self as EntityTrait>::Model: Send + Unpin + Sync,
    {
        let paginator = Self::find().filter(condition).paginate(db, 1);
        Ok(paginator.num_items().await?)
    }

}

impl<T: EntityTrait> EntityExtension for T {

}


#[async_trait]
pub trait ActiveModelExtension: ActiveModelTrait{

    #[allow(unused)]
    async fn persist<C: ConnectionTrait>(mut self, db: &C, is_update: bool) -> BuboResult<<Self::Entity as EntityTrait>::Model>
    where
        Self: ActiveModelBehavior
        + TryIntoModel<<Self::Entity as EntityTrait>::Model>
        + std::marker::Send
        + Unpin
        + Sync,
        for<'de> <Self::Entity as EntityTrait>::Model: Deserialize<'de> + Serialize,
        <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType:
            From<i64>,
        <Self::Entity as EntityTrait>::Model: IntoActiveModel<Self> + std::marker::Send + Unpin + Sync,
    {
        let model = self.clone().try_into_model().unwrap();
        if is_update {
            self.update(db).await?;
        } else {
            self.insert(db).await?;
        }
        Ok(model)
    }

    #[allow(unused)]
    async fn persist_cache<C: ConnectionTrait>(mut self, app_name: &str, db: &C, redis: &RedisPool, 
        is_update: bool) -> BuboResult<<Self::Entity as EntityTrait>::Model>
    where
        Self: ActiveModelBehavior
        + TryIntoModel<<Self::Entity as EntityTrait>::Model>
        + std::marker::Send
        + Unpin
        + Sync,
        for<'de> <Self::Entity as EntityTrait>::Model: Deserialize<'de> + Serialize,
        <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType:
            From<i64>,
        <Self::Entity as EntityTrait>::Model: IntoActiveModel<Self> + std::marker::Send + Unpin + Sync,
    {
        let entity = Self::Entity::default();
        
        let model = if is_update {
            self.update(db).await?
        } else {
            self.insert(db).await?
        };

        let mut id = 0i64;
        if let Some(primary_key) =  <Self::Entity as EntityTrait>::PrimaryKey::iter().next() {
            let col = primary_key.into_column();
            id = model.get(col).unwrap();
        }

        let cache_key = redis::gen_key(app_name, format!("cache-{}", entity.table_name()), id);
        tracing::debug!("set: {}", cache_key);
        // 数据库更新成功后才更新缓存，如果缓存更新成功数据库更新失败会有数据不一致的情况
        let _ = redis::set(redis, &cache_key, &model, Some(Expiration::EX(EXPIRE))).await;
        Ok(model)
    }
}

impl<T: ActiveModelTrait> ActiveModelExtension for T {

}