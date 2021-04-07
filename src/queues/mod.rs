use std::{sync::Arc, time::Duration};

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
    processing_list: String,
    data_hash: String,
    processing_timeout: std::time::Duration,
}

impl Queue {
    pub fn new(pool: deadpool_redis::Pool, queue_name: &str, timeout: Option<Duration>) -> Queue {
        Queue(Arc::new(QueueInner {
            pool,
            pending_list: format!("queue_{}_pending", queue_name),
            processing_list: format!("queue_{}_processing", queue_name),
            data_hash: format!("queue_{}_items", queue_name),
            processing_timeout: timeout.unwrap_or_else(|| Duration::from_secs_f64(30.0)),
        }))
    }

    pub async fn enqueue<T: Serialize>(&self, id: usize, item: &T) -> Result<(), Error> {
        unimplemented!();
    }

    pub async fn enqueue_multiple<T: Serialize>(&self, items: &[(usize, T)]) -> Result<(), Error> {
        unimplemented!();
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
