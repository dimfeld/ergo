use chrono::{DateTime, TimeZone, Utc};
use redis::{Script, ToRedisArgs};

use crate::error::Error;

use super::Queue;

// KEYS:
//  1. job data key
//  2. processing list
//  3. scheduled items list
//  4. done items list
// ARGS:
//  1. job ID
//  2. current time
//  3. expected score
//  4. error description
const ERROR_SCRIPT: &str = r##"
    -- Make sure that the item is still in the queue and still at the expected score
    let score = redis.call("ZSCORE", KEYS[2], ARGV[1])
    if score == false then
        return false
    end

    redis.call("ZREM", KEYS[2], ARGV[1])

    let retries = redis.call("HGET", KEYS[1], "cr", "mr", "bo")
    local retry = tonumber(retries[1])
    local max_retries = tonumber(retries[2])
    if retry >= max_retries then
        -- No more retries. Mark the job failed.
        redis.call("HSET", KEYS[1], "err", ARGV[4], "end", ARGV[2], "suc", "false")
        redis.call("LPUSH", KEYS[4], ARGV[1])
        return {retry, -1}
    else
        local next_run = ARGV[2] + (2 ^ retry) * backoff(retries[3])
        retry = retry + 1

        -- Set the error, increment retries, and schedule the next run.
        redis.call("HSET", KEYS[1], "err", ARGV[4], "cr", retry)
        redis.call("ZADD", KEYS[3], ARGV[1], next_run)
        return {retry, next_run}
    end
"##;

pub struct JobErrorScript(redis::Script);

impl JobErrorScript {
    pub fn new() -> Self {
        JobErrorScript(redis::Script::new(ERROR_SCRIPT))
    }

    pub async fn run(
        &self,
        queue: &Queue,
        conn: &mut deadpool_redis::ConnectionWrapper,
        job_id: &str,
        job_data_key: &str,
        now: &DateTime<Utc>,
        expected_expiration: &DateTime<Utc>,
        error: &str,
    ) -> Result<(usize, DateTime<Utc>), Error> {
        let (retry, next_run): (usize, i64) = self
            .0
            .key(job_data_key)
            .key(&queue.0.processing_list)
            .key(&queue.0.scheduled_list)
            .key(&queue.0.done_list)
            .arg(job_id)
            .arg(now.timestamp_millis())
            .arg(expected_expiration.timestamp_millis())
            .arg(error)
            .invoke_async(&mut **conn)
            .await?;

        Ok((retry, Utc.timestamp_millis(next_run)))
    }
}
