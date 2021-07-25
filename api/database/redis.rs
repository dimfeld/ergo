use crate::error::Result;
use std::{env, ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct RedisPool(Arc<RedisPoolInner>);

struct RedisPoolInner {
    pub pool: deadpool_redis::Pool,
    pub key_prefix: Option<String>,
}

impl Deref for RedisPool {
    type Target = deadpool_redis::Pool;

    fn deref(&self) -> &Self::Target {
        &self.0.pool
    }
}

impl RedisPool {
    pub fn new(connection: Option<String>, key_prefix: Option<String>) -> Result<RedisPool> {
        let redis_host =
            connection.unwrap_or_else(|| env::var("REDIS_URL").expect("REDIS_URL is required"));

        let pool = deadpool_redis::Config {
            url: Some(redis_host),
            connection: None,
            pool: None,
        }
        .create_pool()?;

        Ok(RedisPool(Arc::new(RedisPoolInner { pool, key_prefix })))
    }

    pub fn pool(&self) -> &deadpool_redis::Pool {
        &self.0.pool
    }

    pub fn key_prefix(&self) -> Option<&str> {
        self.0.key_prefix.as_ref().map(|s| s.as_str())
    }
}
