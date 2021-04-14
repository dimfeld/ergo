use chrono::{DateTime, Utc};
use deadpool_redis::ConnectionWrapper;
use redis::Script;

use crate::error::Error;

use super::Queue;

// KEYS:
//  1. pending items list
//  2. processing list
//  3. job data hash
// ARGV:
//  1. queue-default expiration time
const DEQUEUE_ITEM_SCRIPT: &str = r##"
    local latest_item = redis.call("LPOP", KEYS[1])
    if latest_item == false then
        return false
    end

    -- Set the default queue expiration. The job worker will update it if needed
    redis.call("ZADD", KEYS[2], tonumber(ARGV[1]), latest_item)
    redis.call("HINCRBY", KEYS[3], "retrieved", 1)
    return latest_item
"##;

pub struct GetJobScript(redis::Script);

impl GetJobScript {
    pub fn new() -> Self {
        GetJobScript(redis::Script::new(DEQUEUE_ITEM_SCRIPT))
    }

    pub async fn run(
        &self,
        queue: &Queue,
        conn: &mut ConnectionWrapper,
        now: &DateTime<Utc>,
    ) -> Result<Option<String>, Error> {
        let now_millis = now.timestamp_millis();
        let job_id: Option<String> = self
            .0
            .key(&queue.0.pending_list)
            .key(&queue.0.processing_list)
            .key(&queue.0.stats_hash)
            .arg(now_millis + queue.0.processing_timeout.as_millis() as i64)
            .invoke_async(&mut **conn)
            .await?;

        Ok(job_id)
    }
}
