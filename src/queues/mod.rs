use std::{borrow::Cow, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use derivative::Derivative;
use futures::Future;
use redis::{AsyncCommands, RedisError};
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
    job_data_prefix: String,
    processing_timeout: std::time::Duration,
    max_retries: u32,
    enqueue_scheduled_script: redis::Script,
    dequeue_item_script: redis::Script,
    start_work_script: redis::Script,
}

#[derive(Debug, Default)]
pub struct Job<'a> {
    pub id: String,
    pub payload: Cow<'a, [u8]>,
    pub timeout: Option<std::time::Duration>,
    pub max_retries: Option<u32>,
    pub run_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JobTrackingData {
    id: String,
    payload: String,
    #[serde(with = "serde_millis")]
    timeout: std::time::Duration,
    retry_count: u32,
    max_retries: u32,
    run_at: Option<DateTime<Utc>>,
    enqueued_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    succeeded: Option<bool>,
    error_details: Option<String>,
}

enum RedisJobField {
    Payload,
    Timeout,
    CurrentRetries,
    MaxRetries,
    RunAt,
    EnqueuedAt,
    StartedAt,
    EndedAt,
    Succeeded,
    ErrorDetails,
}

impl redis::ToRedisArgs for RedisJobField {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let value = match self {
            RedisJobField::Payload => "pay",
            RedisJobField::Timeout => "to",
            RedisJobField::CurrentRetries => "cr",
            RedisJobField::MaxRetries => "mr",
            RedisJobField::RunAt => "ra",
            RedisJobField::EnqueuedAt => "qt",
            RedisJobField::StartedAt => "start",
            RedisJobField::EndedAt => "end",
            RedisJobField::Succeeded => "suc",
            RedisJobField::ErrorDetails => "err",
        };
        out.write_arg(value.as_bytes())
    }
}

struct RedisJobSetCmd(deadpool_redis::Cmd);

impl RedisJobSetCmd {
    fn new(job_key: &str) -> Self {
        let mut cmd = deadpool_redis::cmd("HSET");
        cmd.arg(job_key);
        RedisJobSetCmd(cmd)
    }

    fn build(self) -> deadpool_redis::Cmd {
        self.0
    }

    fn increment_current_retries(job_key: &str) -> deadpool_redis::Cmd {
        let mut cmd = deadpool_redis::cmd("hincrby");
        cmd.arg(job_key).arg(RedisJobField::CurrentRetries).arg(1);
        cmd
    }

    fn payload(&mut self, s: &[u8]) -> &mut Self {
        self.0.arg(RedisJobField::Payload).arg(s);
        self
    }

    fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.0
            .arg(RedisJobField::Timeout)
            .arg(timeout.as_millis() as u64);
        self
    }

    fn current_retries(&mut self, retries: u32) -> &mut Self {
        self.0.arg(RedisJobField::CurrentRetries).arg(retries);
        self
    }

    fn max_retries(&mut self, retries: u32) -> &mut Self {
        self.0.arg(RedisJobField::MaxRetries).arg(retries);
        self
    }

    fn run_at(&mut self, run_at: &DateTime<Utc>) -> &mut Self {
        self.0
            .arg(RedisJobField::RunAt)
            .arg(run_at.timestamp_millis() as u64);
        self
    }

    fn enqueued_at(&mut self, enqueued_at: &DateTime<Utc>) -> &mut Self {
        self.0
            .arg(RedisJobField::EnqueuedAt)
            .arg(enqueued_at.timestamp_millis());
        self
    }

    fn started_at(&mut self, started_at: &DateTime<Utc>) -> &mut Self {
        self.0
            .arg(RedisJobField::StartedAt)
            .arg(started_at.timestamp_millis());
        self
    }

    fn ended_at(&mut self, ended_at: &DateTime<Utc>) -> &mut Self {
        self.0
            .arg(RedisJobField::EndedAt)
            .arg(ended_at.timestamp_millis());
        self
    }

    fn clear_succeeded(&mut self) -> &mut Self {
        self.0.arg(RedisJobField::Succeeded).arg("");
        self
    }

    fn succeeded(&mut self, succeeded: bool) -> &mut Self {
        self.0.arg(RedisJobField::Succeeded).arg(succeeded);
        self
    }

    fn error_details(&mut self, error: &str) -> &mut Self {
        self.0.arg(RedisJobField::ErrorDetails).arg(error);
        self
    }
}

impl Queue {
    pub fn new(
        pool: deadpool_redis::Pool,
        queue_name: &str,
        default_timeout: Option<Duration>,
        default_max_retries: Option<u32>,
    ) -> Queue {
        // KEYS: scheduled items list, pending items list
        // ARGV: current time
        const ENQUEUE_SCHEDULED_SCRIPT: &str = r##"
            local move_items = redis.call('ZRANGEBYSCORE', KEYS[1], 0, ARGV[1])
            if #move_items == 0 then
                return 0
            end

            redis.call('ZREM', KEYS[1], unpack(move_items))
            redis.call('LPUSH', KEYS[2], unpack(move_items))
            return #move_items
            "##;

        // KEYS: pending items list, processing list
        // ARGV: queue-default expiration time
        const DEQUEUE_ITEM_SCRIPT: &str = r##"
            local latest_item = redis.call("LPOP", KEYS[1])
            if latest_item == false then
                return false
            end

            -- Set the default queue expiration. The job worker will update it if needed
            redis.call("ZADD", KEYS[2], latest_item, ARGV[1])
            return latest_item
        "##;

        // KEYS:
        //  1. job data key
        //  2. processing list
        // ARGS:
        //  1. job ID
        //  2. current time
        //  3. timeout hash key name
        //  4. default expiration,
        //  5. started_at hash key name
        //  6. payload hash key name
        const START_WORK_SCRIPT: &str = r##"
            -- If the job has a different timeout from the queue default, update it here.
            local {job_timeout, payload} = tonumber(redis.call("HMGET", KEYS[1], ARGV[2], ARGV[6]))
            if job_timeout != ARGV[3] then
                redis.call("ZADD", KEYS[2], ARGV[1], ARGV[2] + ARGV[4])
            end

            -- Set started time
            redis.call("HSET", KEYS[1], ARGV[5], ARGV[2])
            return payload
        "##;

        Queue(Arc::new(QueueInner {
            pool,
            pending_list: format!("erq:{}:pending", queue_name),
            scheduled_list: format!("erq:{}:scheduled", queue_name),
            processing_list: format!("erq:{}:processing", queue_name),
            job_data_prefix: format!("erq:{}:job:", queue_name),
            processing_timeout: default_timeout.unwrap_or_else(|| Duration::from_secs_f64(30.0)),
            max_retries: default_max_retries.unwrap_or(3),
            enqueue_scheduled_script: redis::Script::new(ENQUEUE_SCHEDULED_SCRIPT),
            dequeue_item_script: redis::Script::new(DEQUEUE_ITEM_SCRIPT),
            start_work_script: redis::Script::new(START_WORK_SCRIPT),
        }))
    }

    fn add_id_to_queue(&self, pipe: &mut deadpool_redis::Pipeline, job: &'_ Job<'_>) {
        if let Some(timestamp) = job.run_at {
            pipe.zadd(
                &self.0.scheduled_list,
                &job.id,
                timestamp.timestamp_millis(),
            );
        } else {
            pipe.lpush(&self.0.pending_list, &job.id);
        }
    }

    fn job_data_key(&self, job_id: &str) -> String {
        format!("{}:{}", self.0.job_data_prefix, job_id)
    }

    fn set_job_data(&self, job_id: &str) -> redis::Cmd {
        let mut cmd = redis::cmd("HSET");
        cmd.arg(self.job_data_key(&job_id));
        cmd
    }

    fn initial_job_data_cmd<'a>(&self, job: &'a Job) -> deadpool_redis::Cmd {
        let key = self.job_data_key(job.id.as_str());
        let mut cmd = RedisJobSetCmd::new(&key);
        cmd.payload(job.payload.as_ref())
            .timeout(job.timeout.unwrap_or(self.0.processing_timeout))
            .current_retries(0)
            .max_retries(job.max_retries.unwrap_or(self.0.max_retries))
            .enqueued_at(&Utc::now());

        if let Some(r) = job.run_at.as_ref() {
            cmd.run_at(r);
        }

        cmd.build()
    }

    pub async fn enqueue(&self, item: &'_ Job<'_>) -> Result<(), Error> {
        let mut pipe = deadpool_redis::Pipeline::with_capacity(2);

        pipe.add_command(self.initial_job_data_cmd(item));
        self.add_id_to_queue(&mut pipe, item);

        let mut conn = self.0.pool.get().await?;
        pipe.execute_async(&mut conn).await?;
        Ok(())
    }

    pub async fn enqueue_multiple(&self, items: &'_ [Job<'_>]) -> Result<(), Error> {
        let mut pipe = deadpool_redis::Pipeline::with_capacity(items.len() * 2);

        for item in items {
            pipe.add_command(self.initial_job_data_cmd(item));
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

    async fn start_working<T: DeserializeOwned + Send + Sync>(
        &self,
        conn: &mut deadpool_redis::ConnectionWrapper,
        job_id: &str,
        job_id_key: &str,
        now: &DateTime<Utc>,
        now_millis: i64,
    ) -> Result<QueueWorkItem<T>, Error> {
        let payload: Vec<u8> = self
            .0
            .start_work_script
            .key(&self.0.processing_list)
            .key(job_id_key)
            .arg(job_id)
            .arg(now_millis)
            .arg(RedisJobField::Timeout)
            .arg(self.0.processing_timeout.as_millis() as i64)
            .arg(RedisJobField::StartedAt)
            .arg(RedisJobField::Payload)
            .invoke_async(&mut **conn)
            .await?;

        let item = QueueWorkItem::new(self.clone(), job_id, payload)?;
        Ok(item)
    }

    pub async fn dequeue<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<Option<QueueWorkItem<T>>, Error> {
        // 1. Run dequeue script
        let now = Utc::now();
        let now_millis = now.timestamp_millis();
        let mut conn = self.0.pool.get().await?;
        let result: Option<String> = self
            .0
            .dequeue_item_script
            .key(&self.0.pending_list)
            .key(&self.0.processing_list)
            .arg(now_millis + self.0.processing_timeout.as_millis() as i64)
            .invoke_async(&mut **conn)
            .await?;

        // Unwrap the Option or just exit if there was no job.
        let job_id = match result {
            Some(id) => id,
            None => {
                return Ok(None);
            }
        };
        let job_id_key = self.job_data_key(&job_id);
        let work_item = self
            .start_working(&mut conn, job_id.as_str(), &job_id_key, &now, now_millis)
            .await;

        match work_item {
            Ok(work_item) => Ok(Some(work_item)),
            Err(e) => {
                let e = Error::from(e);
                let err_str = format!("Failed to start job: {}", e);
                self.errored_item(job_id.as_str(), err_str.as_str()).await?;
                Err(e)
            }
        }
    }

    async fn done_item(&self, id: &str) -> Result<(), RedisError> {
        // Set ended at = now, success = true
        // Remove from processing list
        unimplemented!();
    }

    async fn errored_item(&self, id: &str, error: &str) -> Result<(), RedisError> {
        // Set error
        // If max retries, move to failed list, set ended_at and success = false
        // Otherwise calculate backoff and move to retry list
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
    pub id: String,
    pub data: T,

    finished: bool,
}

// Eventually I can enable these. For now, you'll have to use Box<serde_json::value::RawValue> if
// you don't want your value parsed as JSON.
//
// impl QueueWorkItem<Vec<u8>> {
//     fn new(queue: Queue, job_id: &str, data: Vec<u8>) -> Result<Self, Error> {
//         Ok(QueueWorkItem {
//             queue,
//             id: String::from(job_id),
//             data,
//             finished: false,
//         })
//     }
// }
//
// impl QueueWorkItem<String> {
//     fn new(queue: Queue, job_id: &str, data: Vec<u8>) -> Result<Self, Error> {
//         Ok(QueueWorkItem {
//             queue,
//             id: String::from(job_id),
//             data: String::from_utf8(data)?,
//             finished: false,
//         })
//     }
// }

impl<T: DeserializeOwned + Send + Sync> QueueWorkItem<T> {
    fn new(queue: Queue, job_id: &str, data: Vec<u8>) -> Result<Self, Error> {
        let converted: T = serde_json::from_slice(data.as_slice())?;
        Ok(QueueWorkItem {
            queue,
            id: String::from(job_id),
            data: converted,
            finished: false,
        })
    }
}

impl<T: Send + Sync> QueueWorkItem<T> {
    pub async fn process<F, Fut, R, E>(&self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&T) -> Fut,
        Fut: Future<Output = Result<R, E>>,
        T: Send,
        E: Into<Error> + Send,
    {
        match f(&self.data).await {
            Ok(val) => {
                self.queue.done_item(self.id.as_str()).await?;
                Ok(val)
            }
            Err(e) => {
                let e: Error = e.into();
                self.queue
                    .errored_item(self.id.as_str(), &e.to_string().as_str())
                    .await?;
                Err(e)
            }
        }
    }
}
