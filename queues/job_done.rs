use chrono::{DateTime, Utc};
use lazy_static::lazy_static;

use crate::error::Error;

use super::Queue;

// Mark a job done
// KEYS:
//  1. job data key
//  2. processing list
//  3. done list
//  4. queue stats hash
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
    redis.call("HINCRBY", KEYS[4], "succeeded", 1)
    return {score, true}
"##;

lazy_static! {
    static ref SCRIPT: redis::Script = redis::Script::new(DONE_SCRIPT);
}

pub struct JobDoneScript(&'static redis::Script);

impl JobDoneScript {
    pub fn new() -> Self {
        JobDoneScript(&SCRIPT)
    }

    pub async fn run(
        &self,
        queue: &Queue,
        conn: &mut deadpool_redis::Connection,
        job_id: &str,
        job_data_key: &str,
        now: &DateTime<Utc>,
        expected_expiration: &DateTime<Utc>,
    ) -> Result<bool, Error> {
        let (_found_score, marked_done): (String, bool) = self
            .0
            .key(job_data_key)
            .key(&queue.0.processing_list)
            .key(&queue.0.done_list)
            .key(&queue.0.stats_hash)
            .arg(job_id)
            .arg(now.timestamp_millis())
            .arg(expected_expiration.timestamp_millis())
            .invoke_async(&mut **conn)
            .await?;

        Ok(marked_done)
    }
}
