use chrono::{DateTime, Utc};
use lazy_static::lazy_static;

use super::{JobStatus, Queue};
use crate::error::Error;

// KEYS:
//  1. job data key
//  2. processing list
//  3. pending list
//  4. scheduled items list
// ARGS:
//  1. job ID
//  2. current time
//  3. cancel the job if it has already started running
const CANCEL_SCRIPT: &str = r##"
    local was_pending = redis.call("LREM", KEYS[3], 1, ARGV[1])
    local was_processing = redis.call("ZREM", KEYS[2], ARGV[1])
    local was_scheduled = redis.call("ZREM", KEYS[4], ARGV[1])

    local suc = false
    if was_pending == false and was_processing == false and was_scheduled == false then
        -- If the job wasn't running or set to run, then it already finished.
        local job_data = redis.call("HGET", KEYS[1], "suc")
        suc = job_data[1]
    elseif was_processing = false or ARGV[3] == "1" then
        -- If we're allowed to cancel the job, then do so.
        -- Set end time. Leave success unset.
        redis.call("HSET", KEYS[1], "end", ARGV[2], "err", "canceled")
    end

    return { was_pending, was_processing, was_scheduled, suc }
    "##;

lazy_static! {
    static ref SCRIPT: redis::Script = redis::Script::new(CANCEL_SCRIPT);
}

pub struct JobCancelScript(&'static redis::Script);

impl JobCancelScript {
    pub fn new() -> Self {
        JobCancelScript(&SCRIPT)
    }

    pub async fn run(
        &self,
        queue: &Queue,
        conn: &mut deadpool_redis::Connection,
        job_id: &str,
        job_data_key: &str,
        now: &DateTime<Utc>,
        cancel_if_running: bool,
    ) -> Result<JobStatus, Error> {
        let result: (Option<usize>, Option<usize>, Option<usize>, Option<bool>) = self
            .0
            .key(job_data_key)
            .key(&queue.0.processing_list)
            .key(&queue.0.pending_list)
            .key(&queue.0.scheduled_list)
            .arg(job_id)
            .arg(now.timestamp_millis())
            .arg(cancel_if_running)
            .invoke_async(&mut **conn)
            .await?;

        let status = match result {
            (Some(1), _, _, _) => JobStatus::Pending,
            (_, Some(1), _, _) => JobStatus::Running,
            (_, _, Some(1), _) => JobStatus::Scheduled,
            (_, _, _, Some(true)) => JobStatus::Done,
            (_, _, _, Some(false)) => JobStatus::Errored,
            _ => JobStatus::Inactive,
        };

        Ok(status)
    }
}
