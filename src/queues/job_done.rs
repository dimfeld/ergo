use chrono::{DateTime, Utc};
use redis::Script;

use crate::error::Error;

use super::Queue;

// Mark a job done
// KEYS:
//  1. job data key
//  2. processing list
//  3. done list
// ARGS:
//  1. job id
//  2. current time
//  3. expected expiration
const DONE_SCRIPT: &str = r##"
    local score = redis.call("ZSCORE", KEYS[2], ARGV[1])
    if score ~= ARGV[3] then
        -- We no longer own this item, so don't mess with it.
        return {score, false}
    end

    redis.call("ZREM", KEYS[2], ARGV[1])
    redis.call("LPUSH", KEYS[3], ARGV[1])
    redis.call("HSET", KEYS[1], "end", ARGV[2], "suc", "true")
    return {score, true}
"##;

pub struct JobDoneScript(redis::Script);

impl JobDoneScript {
    pub fn new() -> Self {
        JobDoneScript(redis::Script::new(DONE_SCRIPT))
    }

    pub async fn run(
        &self,
        queue: &Queue,
        conn: &mut deadpool_redis::ConnectionWrapper,
        job_id: &str,
        job_data_key: &str,
        now: &DateTime<Utc>,
        expected_expiration: &DateTime<Utc>,
    ) -> Result<bool, Error> {
        let (found_score, marked_done): (String, bool) = self
            .0
            .key(job_data_key)
            .key(&queue.0.processing_list)
            .key(&queue.0.done_list)
            .arg(job_id)
            .arg(now.timestamp_millis())
            .arg(expected_expiration.timestamp_millis())
            .invoke_async(&mut **conn)
            .await?;

        Ok(marked_done)
    }
}
