use chrono::{DateTime, Utc};
use deadpool_redis::Connection;
use lazy_static::lazy_static;

use super::Queue;
use crate::Error;

// KEYS:
//  1. pending items list
//  2. scheduled items list
//  3. job data key
// ARGV:
//  1. Job ID
//  2. Optional new time to run
//  3. Optional new payload
const UPDATE_JOB_SCRIPT: &str = r##"
    local is_scheduled = redis.call("ZSCORE", KEYS[2], ARGV[1])
    local is_pending = false
    local updates_time = string.len(ARGV[2]) > 0

    -- Items being updated will usually be in the scheduled list, and accessing the pending list is O(N), so
    -- look up in the pending list only if we have to, and combine with the removal operation if appropriate.
    if is_scheduled == false then
        if updates_time then
            -- If we're updating the scheduled time then we unconditionally move the item to the scheduled list,
            -- so remove it here.
            is_pending = redis.call("LREM", KEYS[1], 1, ARGV[1]) > 0
        else
            is_pending = redis.call("LPOS", KEYS[1], ARGV[1]) ~= false
        end
    end

    if is_pending == false and is_scheduled == false then
        -- If the job already started running then it's too late to update
        return false
    end

    if updates_time then
        -- Put the task on the scheduled list at the new time.
        redis.call("ZADD", KEYS[2], ARGV[2], ARGV[1])
        redis.call("HSET", KEYS[3], "ra", ARGV[2])
    end

    if string.len(ARGV[3]) > 0 then
        -- Update the payload
        redis.call("HSET", KEYS[3], "pay", ARGV[3])
    end

    return true
"##;

lazy_static! {
    static ref SCRIPT: redis::Script = redis::Script::new(UPDATE_JOB_SCRIPT);
}

pub struct UpdateJobScript(&'static redis::Script);

impl UpdateJobScript {
    pub fn new() -> Self {
        UpdateJobScript(&SCRIPT)
    }

    pub async fn run(
        &self,
        queue: &Queue,
        conn: &mut Connection,
        job_id: &str,
        job_data_key: &str,
        new_time: Option<DateTime<Utc>>,
        new_payload: Option<&[u8]>,
    ) -> Result<bool, Error> {
        let success: bool = self
            .0
            .key(&queue.0.pending_list)
            .key(&queue.0.scheduled_list)
            .key(job_data_key)
            .arg(job_id)
            .arg(
                new_time
                    .map(|t| t.timestamp_millis().to_string())
                    .unwrap_or_else(String::new), // Send an empty string if it's None
            )
            .arg(new_payload.unwrap_or(&[]))
            .invoke_async(&mut **conn)
            .await?;

        Ok(success)
    }
}
