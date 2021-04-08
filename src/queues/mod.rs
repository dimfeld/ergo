use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use derivative::Derivative;
use futures::Future;
use redis::RedisError;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::Error;

pub mod postgres_drain;

pub struct Queue(Arc<QueueInner>);

#[derive(Derivative)]
#[derivative(Debug)]
struct QueueInner {
    #[derivative(Debug = "ignore")]
    pool: deadpool_redis::Pool,
    pending_list: String,
    scheduled_list: String,
    processing_list: String,
    data_hash: String,
    processing_timeout: std::time::Duration,
    max_retries: u32,
    enqueue_scheduled_script: redis::Script,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Job<T> {
    pub id: usize,
    pub payload: T,
    pub timeout: Option<std::time::Duration>,
    pub max_retries: Option<u32>,
    pub run_at: Option<DateTime<Utc>>,
}

impl Queue {
    pub fn new(
        pool: deadpool_redis::Pool,
        queue_name: &str,
        default_timeout: Option<Duration>,
        default_max_retries: Option<u32>,
    ) -> Queue {
        const ENQUEUE_SCHEDULED_SCRIPT: &str = r##"
          local move_items = redis.call("ZRANGEBYSCORE", KEYS[1], 0, ARGV[1])
          redis.call("ZREM", KEYS[1], unpack(move_items))
          redis.call("LPUSH", KEYS[2], unpack(move_items))
          return #move_items
          "##;

        Queue(Arc::new(QueueInner {
            pool,
            pending_list: format!("queue_{}_pending", queue_name),
            scheduled_list: format!("queue_{}_scheduled", queue_name),
            processing_list: format!("queue_{}_processing", queue_name),
            data_hash: format!("queue_{}_items", queue_name),
            processing_timeout: default_timeout.unwrap_or_else(|| Duration::from_secs_f64(30.0)),
            max_retries: default_max_retries.unwrap_or(3),
            enqueue_scheduled_script: redis::Script::new(ENQUEUE_SCHEDULED_SCRIPT),
        }))
    }

    fn add_id_to_queue<T>(&self, pipe: &mut deadpool_redis::Pipeline, job: &Job<T>) {
        if let Some(timestamp) = job.run_at {
            pipe.zadd(&self.0.scheduled_list, job.id, timestamp.timestamp_millis());
        } else {
            pipe.lpush(&self.0.pending_list, job.id);
        }
    }

    pub async fn enqueue<T: Serialize>(&self, item: &Job<T>) -> Result<(), Error> {
        let mut pipe = deadpool_redis::Pipeline::with_capacity(2);

        pipe.hset(&self.0.data_hash, item.id, serde_json::to_string(item)?);
        self.add_id_to_queue(&mut pipe, item);

        let mut conn = self.0.pool.get().await?;
        pipe.execute_async(&mut conn).await?;
        Ok(())
    }

    pub async fn enqueue_multiple<T: Serialize>(&self, items: &[Job<T>]) -> Result<(), Error> {
        let mut pipe = deadpool_redis::Pipeline::with_capacity(items.len() + 1);

        let hash_data = items
            .iter()
            .map(|item| Ok((item.id, serde_json::to_string(item)?)))
            .collect::<Result<Vec<_>, Error>>()?;
        pipe.hset_multiple(&self.0.data_hash, hash_data.as_slice());

        for item in items {
            self.add_id_to_queue(&mut pipe, &item);
        }

        let mut conn = self.0.pool.get().await?;
        pipe.execute_async(&mut conn).await?;

        Ok(())
    }

    /// Move each scheduled item that has reached its deadline to the pending list.
    pub async fn enqueue_scheduled_items(&self) -> Result<bool, Error> {
        let now = Utc::now().timestamp_millis();
        let mut conn = self.0.pool.get().await?;
        let result: usize = self
            .0
            .enqueue_scheduled_script
            .key(&self.0.scheduled_list)
            .key(&self.0.pending_list)
            .arg(now)
            .invoke_async(&mut **conn)
            .await?;
        Ok(result > 0)
    }

    pub async fn dequeue<T: DeserializeOwned + Send + Sync>(
    ) -> Result<Option<QueueWorkItem<T>>, Error> {
        unimplemented!();
    }

    async fn done_item(&self, id: usize) -> Result<(), RedisError> {
        unimplemented!();
    }

    async fn errored_item(&self, id: usize) -> Result<(), RedisError> {
        unimplemented!();
    }
}

impl Clone for Queue {
    fn clone(&self) -> Queue {
        Queue(self.0.clone())
    }
}

pub struct QueueWorkItem<T: Send + Sync> {
    queue: Queue,
    pub id: usize,
    pub data: T,

    finished: bool,
}

impl<T: DeserializeOwned + Serialize + Send + Sync> QueueWorkItem<T> {
    pub async fn process<F, Fut, R, E>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = Result<R, E>>,
        T: Send,
        E: Into<Error> + Send,
    {
        match f(&self.data).await {
            Ok(val) => {
                self.queue.done_item(self.id).await?;
                Ok(val)
            }
            Err(e) => {
                self.queue.errored_item(self.id).await?;
                Err(e.into())
            }
        }
    }
}
