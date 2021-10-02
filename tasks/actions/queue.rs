use std::{borrow::Cow, ops::Deref};

use crate::error::Error;

use ergo_database::RedisPool;
use ergo_queues::{
    generic_stage::{enqueue_jobs, QueueJob},
    Queue,
};
use smallvec::SmallVec;
use sqlx::PgConnection;

use super::ActionInvocations;

const QUEUE_NAME: &str = "er-action";

#[derive(Clone)]
pub struct ActionQueue(Queue);
impl Deref for ActionQueue {
    type Target = Queue;

    fn deref(&self) -> &Queue {
        &self.0
    }
}

impl ActionQueue {
    pub fn new(redis_pool: RedisPool) -> ActionQueue {
        let queue_name = match redis_pool.key_prefix() {
            Some(prefix) => format!("{}-{}", prefix, QUEUE_NAME),
            None => QUEUE_NAME.to_string(),
        };

        ActionQueue(Queue::new(redis_pool, queue_name, None, None, None))
    }
}

pub async fn enqueue_actions(
    tx: &mut PgConnection,
    actions: &ActionInvocations,
    key_prefix: &Option<String>,
) -> Result<(), Error> {
    let queue_name = key_prefix
        .as_ref()
        .map(|prefix| Cow::Owned(format!("{}-{}", prefix, QUEUE_NAME)))
        .unwrap_or(Cow::Borrowed(QUEUE_NAME));
    let jobs = actions
        .iter()
        .map(|inv| QueueJob {
            timeout: None,
            id: None,
            queue: queue_name.as_ref(),
            run_at: None,
            max_retries: None,
            retry_backoff: None,
            payload: inv,
        })
        .collect::<SmallVec<[QueueJob<_>; 4]>>();

    enqueue_jobs(tx, jobs.as_slice()).await?;
    Ok(())
}
