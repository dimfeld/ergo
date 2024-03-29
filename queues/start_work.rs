use chrono::{DateTime, TimeZone, Utc};
use deadpool_redis::Connection;
use lazy_static::lazy_static;

use crate::error::Error;

use super::Queue;

// Start work on an item. This assumes that the item has already been placed into the
// processing list.
// KEYS:
//  1. job data key
//  2. processing list
// ARGS:
//  1. job ID
//  2. current time
//  3. default expiration,
const START_WORK_SCRIPT: &str = r##"
    local job_data = redis.call("HMGET", KEYS[1], "to", "pay", "cr", "mr")
    local expiration = ARGV[2] + ARGV[3]
    -- If the job has a different timeout from the queue default, update it here.
    if job_data[1] ~= ARGV[3] then
        redis.call("ZADD", KEYS[2], expiration, ARGV[1])
    end

    -- Set started time
    redis.call("HSET", KEYS[1], "st", ARGV[2])
    return {job_data[2], expiration, job_data[3], job_data[4]}
"##;

lazy_static! {
    static ref SCRIPT: redis::Script = redis::Script::new(START_WORK_SCRIPT);
}

pub struct StartWorkScript(&'static redis::Script);

impl StartWorkScript {
    pub fn new() -> Self {
        StartWorkScript(&SCRIPT)
    }

    pub async fn run(
        &self,
        queue: &Queue,
        conn: &mut Connection,
        job_id: &str,
        job_id_key: &str,
        now: &DateTime<Utc>,
    ) -> Result<(Vec<u8>, DateTime<Utc>, usize, usize), Error> {
        let (payload, expiration, current_retry, max_retries): (Vec<u8>, i64, usize, usize) = self
            .0
            .key(job_id_key)
            .key(&queue.0.processing_list)
            .arg(job_id)
            .arg(now.timestamp_millis())
            .arg(queue.0.processing_timeout.as_millis() as i64)
            .invoke_async(&mut **conn)
            .await?;

        Ok((
            payload,
            Utc.timestamp_millis(expiration),
            current_retry,
            max_retries,
        ))
    }
}
