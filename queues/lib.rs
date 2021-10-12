pub mod generic_stage;
pub mod job;
pub mod postgres_drain;
mod update_stage;
pub mod work_item;

mod dequeuer_loop;
mod enqueue_scheduled;
mod error;
mod get_job;
mod job_cancel;
mod job_done;
mod job_error;
mod redis_job_data;
mod start_work;
mod update_job;

use self::redis_job_data::{RedisJobField, RedisJobSetCmd};
pub use self::{
    dequeuer_loop::QueueJobProcessor,
    error::*,
    job::*,
    update_stage::{remove_pending_job, update_pending_job, JobUpdate},
    work_item::*,
};

use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::Duration,
};

use backoff::{backoff::Backoff, ExponentialBackoff};
use chrono::{DateTime, TimeZone, Utc};
use ergo_database::RedisPool;
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use itertools::Itertools;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::{sync::oneshot, task::JoinHandle};
use tracing::{event, Level};

pub struct Queue(Arc<QueueInner>);

impl std::fmt::Debug for Queue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Queue")
            .field("name", &self.0.name)
            .field("max_retries", &self.0.max_retries)
            .field("processing_timeout", &self.0.processing_timeout)
            .field("retry_backoff", &self.0.retry_backoff)
            .finish()
    }
}

struct QueueInner {
    pool: RedisPool,
    name: String,
    pending_list: String,
    scheduled_list: String,
    processing_list: String,
    done_list: String,
    stats_hash: String,
    job_data_prefix: String,
    processing_timeout: Duration,
    max_retries: u32,
    retry_backoff: Duration,
    enqueue_scheduled_script: enqueue_scheduled::EnqueueScript,
    dequeue_item_script: get_job::GetJobScript,
    start_work_script: start_work::StartWorkScript,
    done_script: job_done::JobDoneScript,
    error_script: job_error::JobErrorScript,
    cancel_script: job_cancel::JobCancelScript,
    update_script: update_job::UpdateJobScript,

    scheduled_job_enqueuer_task: Mutex<Option<(oneshot::Sender<()>, JoinHandle<()>)>>,
    job_dequeuer_task: Mutex<Option<(oneshot::Sender<()>, JoinHandle<()>)>>,
}

pub enum JobStatus {
    Inactive,
    Pending,
    Scheduled,
    Running,
    Done,
    Errored,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JobTrackingData {
    id: String,
    payload: Vec<u8>,
    #[serde(with = "serde_millis")]
    timeout: Duration,
    retry_count: u32,
    max_retries: u32,
    run_at: Option<DateTime<Utc>>,
    enqueued_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    succeeded: Option<bool>,
    error_details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueueStatus {
    pub current_running: usize,
    pub current_scheduled: usize,
    pub current_pending: usize,

    pub total_retrieved: usize,
    pub total_enqueued: usize,
    pub total_scheduled: usize,
    pub total_succeeded: usize,
    pub total_failed: usize,
    pub total_errored: usize,
}

impl Queue {
    pub fn new(
        pool: RedisPool,
        queue_name: String,
        default_timeout: Option<Duration>,
        default_max_retries: Option<u32>,
        default_retry_backoff: Option<Duration>,
    ) -> Queue {
        Queue(Arc::new(QueueInner {
            pool,
            pending_list: format!("erq:{}:pending", queue_name),
            scheduled_list: format!("erq:{}:scheduled", queue_name),
            processing_list: format!("erq:{}:processing", queue_name),
            done_list: format!("erq:{}:done", queue_name),
            stats_hash: format!("erq:{}:stats", queue_name),
            job_data_prefix: format!("erq:{}:job:", queue_name),
            processing_timeout: default_timeout.unwrap_or_else(|| Duration::from_secs_f64(120.0)),
            max_retries: default_max_retries.unwrap_or(3),
            retry_backoff: default_retry_backoff.unwrap_or(Duration::from_millis(30000)),
            enqueue_scheduled_script: enqueue_scheduled::EnqueueScript::new(),
            dequeue_item_script: get_job::GetJobScript::new(),
            start_work_script: start_work::StartWorkScript::new(),
            done_script: job_done::JobDoneScript::new(),
            error_script: job_error::JobErrorScript::new(),
            cancel_script: job_cancel::JobCancelScript::new(),
            update_script: update_job::UpdateJobScript::new(),
            scheduled_job_enqueuer_task: Mutex::new(None),
            job_dequeuer_task: Mutex::new(None),
            name: queue_name,
        }))
    }

    pub fn name(&self) -> &str {
        self.0.name.as_str()
    }

    fn add_id_to_queue(&self, pipe: &mut redis::Pipeline, job: &'_ Job<'_>) {
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
        format!("{}{}", self.0.job_data_prefix, job_id)
    }

    fn initial_job_data_cmd(&self, job: &Job) -> redis::Cmd {
        let key = self.job_data_key(job.id.as_str());
        let mut cmd = RedisJobSetCmd::new(&key)
            .payload(job.payload.as_ref())
            .timeout(job.timeout.unwrap_or(self.0.processing_timeout))
            .current_retries(0)
            .max_retries(job.max_retries.unwrap_or(self.0.max_retries))
            .retry_backoff(job.retry_backoff.unwrap_or(self.0.retry_backoff))
            .enqueued_at(&Utc::now());

        if let Some(r) = job.run_at.as_ref() {
            cmd = cmd.run_at(r);
        }

        cmd.build()
    }

    pub async fn status(&self) -> Result<QueueStatus, Error> {
        let mut conn = self.0.pool.get().await?;
        let (
            current_scheduled,
            current_running,
            current_pending,
            (
                total_retrieved,
                total_enqueued,
                total_scheduled,
                total_succeeded,
                total_failed,
                total_errored,
            ),
        ): (
            usize,
            usize,
            usize,
            (
                Option<usize>,
                Option<usize>,
                Option<usize>,
                Option<usize>,
                Option<usize>,
                Option<usize>,
            ),
        ) = redis::Pipeline::with_capacity(4)
            .cmd("ZCARD")
            .arg(&self.0.scheduled_list)
            .cmd("ZCARD")
            .arg(&self.0.processing_list)
            .cmd("LLEN")
            .arg(&self.0.pending_list)
            .cmd("HMGET")
            .arg(&[
                &self.0.stats_hash,
                "retrieved",
                "enqueued",
                "scheduled",
                "succeeded",
                "failed",
                "errored",
            ])
            .query_async(&mut conn)
            .await?;

        Ok(QueueStatus {
            current_running,
            current_scheduled,
            current_pending,
            total_retrieved: total_retrieved.unwrap_or(0),
            total_enqueued: total_enqueued.unwrap_or(0),
            total_scheduled: total_scheduled.unwrap_or(0),
            total_succeeded: total_succeeded.unwrap_or(0),
            total_failed: total_failed.unwrap_or(0),
            total_errored: total_errored.unwrap_or(0),
        })
    }

    pub async fn list_scheduled(&self) -> Result<Vec<(String, DateTime<Utc>)>, Error> {
        let list: Vec<String> = self
            .0
            .pool
            .get()
            .await?
            .zrange_withscores(&self.0.scheduled_list, 0, -1)
            .await?;

        list.into_iter()
            .tuples::<(_, _)>()
            .map(|chunk| Ok((chunk.0, Utc.timestamp_millis(chunk.1.parse::<i64>()?))))
            .collect::<Result<Vec<_>, Error>>()
    }

    pub async fn list_processing(&self) -> Result<Vec<(String, DateTime<Utc>)>, Error> {
        let list: Vec<String> = self
            .0
            .pool
            .get()
            .await?
            .zrange_withscores(&self.0.processing_list, 0, -1)
            .await?;

        list.into_iter()
            .tuples::<(_, _)>()
            .map(|chunk| Ok((chunk.0, Utc.timestamp_millis(chunk.1.parse::<i64>()?))))
            .collect::<Result<Vec<_>, Error>>()
    }

    pub async fn list_pending(&self) -> Result<Vec<String>, Error> {
        self.0
            .pool
            .get()
            .await?
            .lrange(&self.0.pending_list, 0, -1)
            .await
            .map_err(Error::from)
    }

    pub async fn enqueue(&self, item: &'_ Job<'_>) -> Result<(), Error> {
        let mut pipe = redis::Pipeline::with_capacity(2);

        pipe.add_command(self.initial_job_data_cmd(item));
        self.add_id_to_queue(&mut pipe, item);
        pipe.cmd("HINCRBY")
            .arg(&[&self.0.stats_hash, "enqueued", "1"]);

        let mut conn = self.0.pool.get().await?;
        pipe.query_async(&mut conn).await?;
        Ok(())
    }

    pub async fn enqueue_multiple(&self, items: &'_ [Job<'_>]) -> Result<(), Error> {
        let mut pipe = redis::Pipeline::with_capacity(items.len() * 2);

        for item in items {
            pipe.add_command(self.initial_job_data_cmd(item));
            self.add_id_to_queue(&mut pipe, &item);
        }

        let mut conn = self.0.pool.get().await?;
        pipe.query_async(&mut conn).await?;

        Ok(())
    }

    /// Move each scheduled item that has reached its deadline to the pending list.
    pub async fn enqueue_scheduled_items(&self) -> Result<usize, Error> {
        let mut conn = self.0.pool.get().await?;
        let num_queued = self
            .0
            .enqueue_scheduled_script
            .run(self, &mut conn, &Utc::now())
            .await?;
        Ok(num_queued)
    }

    /// Start the scheduled jobs enqueuer task. This task will automatically be stopped when the
    /// last reference to the queue is dropped.
    pub fn start_scheduled_jobs_enqueuer(&self, mut close: GracefulShutdownConsumer) {
        if self.0.scheduled_job_enqueuer_task.lock().unwrap().is_some() {
            return;
        }

        event!(Level::INFO, queue=%self.0.name, "Starting scheduled jobs checker");

        let queue = self.clone();
        let (closer_tx, closer_rx) = oneshot::channel::<()>();
        let task = tokio::spawn(async move {
            let shutdown_fut = close.wait_for_shutdown();
            tokio::pin!(shutdown_fut);
            tokio::pin!(closer_rx);

            let mut interval = tokio::time::interval(Duration::from_millis(1000));

            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown_fut => break,
                    _ = &mut closer_rx => break,
                    _ = interval.tick() => {},
                };

                match queue.enqueue_scheduled_items().await {
                    Ok(num) => {
                        if num > 0 {
                            event!(Level::INFO, queue=%queue.0.name, count=%num, "Enqueued scheduled jobs");
                        }
                    }
                    Err(e) => {
                        event!(Level::ERROR, queue=%queue.0.name, error=%e, "Error enqueueing scheduled jobs");
                    }
                };
            }
        });

        // We don't have to do anything with `closer_tx` except keep it alive, then when
        // the queue is dropped it'll automatically drop the channel as well and lead to
        // the task closing.
        *self.0.scheduled_job_enqueuer_task.lock().unwrap() = Some((closer_tx, task));
    }

    /// Stop the scheduled job enqueuer task, if it was started. This can be used to shut down the
    /// task early, but is not necessary to call as the task will be automatically stopped when the
    /// last reference to the queue is dropped.
    pub fn stop_scheduled_jobs_enqueuer(&self) -> Option<JoinHandle<()>> {
        let (_, task_handle) = self.0.scheduled_job_enqueuer_task.lock().unwrap().take()?;

        // Just let the closer Sender drop, which will cause the queuer task to stop.
        Some(task_handle)
    }

    pub fn default_backoff() -> ExponentialBackoff {
        ExponentialBackoff {
            current_interval: Duration::from_millis(50),
            initial_interval: Duration::from_millis(50),
            max_interval: Duration::from_millis(1000),
            max_elapsed_time: None,
            ..Default::default()
        }
    }

    pub fn start_dequeuer_loop<P, T>(
        &self,
        shutdown: GracefulShutdownConsumer,
        backoff: Option<Box<dyn Backoff + Send>>,
        max_jobs: Option<NonZeroU32>,
        processor: P,
    ) where
        P: QueueJobProcessor<Payload = T> + 'static,
        T: DeserializeOwned + Send + Sync + 'static,
    {
        if self.0.job_dequeuer_task.lock().unwrap().is_some() {
            return;
        }

        event!(Level::INFO, queue=%self.0.name, "Starting job processor");

        let backoff = backoff.unwrap_or(Box::new(Queue::default_backoff()));

        let max_jobs = max_jobs
            .map(|n| n.get() as usize)
            .unwrap_or_else(|| num_cpus::get() * 2);
        let queue = self.clone();
        let (closer_tx, closer_rx) = oneshot::channel::<()>();

        let task =
            dequeuer_loop::dequeuer_loop(queue, shutdown, closer_rx, backoff, max_jobs, processor);

        *self.0.job_dequeuer_task.lock().unwrap() = Some((closer_tx, task));
    }

    /// Stop the job dequeuer task, if it was started. This can be used to shut down the
    /// task early, but is not necessary to call as the task will be automatically stopped when the
    /// last reference to the queue is dropped.
    pub fn stop_dequeuer_loop(&self) -> Option<JoinHandle<()>> {
        let (_, task_handle) = self.0.job_dequeuer_task.lock().unwrap().take()?;

        // Just let the closer Sender drop, which will cause the queuer task to stop.
        Some(task_handle)
    }

    async fn start_working<T: DeserializeOwned + Send + Sync>(
        &self,
        conn: &mut deadpool_redis::Connection,
        job_id: &str,
        job_id_key: &str,
        now: &DateTime<Utc>,
    ) -> Result<QueueWorkItem<T>, Error> {
        let (payload, expiration, current_retry, max_retries) = self
            .0
            .start_work_script
            .run(self, conn, job_id, job_id_key, now)
            .await?;

        let item = QueueWorkItem::new(
            self.clone(),
            job_id,
            expiration,
            current_retry,
            max_retries,
            payload,
        );

        match item {
            Ok(item) => Ok(item),
            Err(e) => {
                let e = Error::from(e);
                let err_str = format!("Failed to start job: {}", e);
                self.errored_job(job_id, &expiration, err_str.as_str())
                    .await?;
                Err(e)
            }
        }
    }

    pub async fn job_info(&self, job_id: &str) -> Result<Option<JobTrackingData>, Error> {
        let job_data_key = self.job_data_key(job_id);
        let mut conn = self.0.pool.get().await?;
        let (
            payload,
            timeout,
            current_retries,
            max_retries,
            _retry_backoff,
            run_at,
            enqueued_at,
            started_at,
            ended_at,
            succeeded,
            error,
        ): (
            Option<Vec<u8>>,
            Option<u64>,
            Option<u32>,
            Option<u32>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<String>,
            Option<String>,
        ) = redis::cmd("HMGET")
            .arg(&job_data_key)
            .arg(RedisJobField::Payload)
            .arg(RedisJobField::Timeout)
            .arg(RedisJobField::CurrentRetries)
            .arg(RedisJobField::MaxRetries)
            .arg(RedisJobField::RetryBackoff)
            .arg(RedisJobField::RunAt)
            .arg(RedisJobField::EnqueuedAt)
            .arg(RedisJobField::StartedAt)
            .arg(RedisJobField::EndedAt)
            .arg(RedisJobField::Succeeded)
            .arg(RedisJobField::ErrorDetails)
            .query_async(&mut conn)
            .await?;

        match (payload, timeout, current_retries, max_retries, enqueued_at) {
            (
                Some(payload),
                Some(timeout),
                Some(current_retries),
                Some(max_retries),
                Some(enqueued_at),
            ) => Ok(Some(JobTrackingData {
                id: String::from(job_id),
                payload,
                timeout: Duration::from_millis(timeout),
                retry_count: current_retries,
                max_retries,
                run_at: run_at.map(|d| Utc.timestamp_millis(d)),
                enqueued_at: Utc.timestamp_millis(enqueued_at),
                started_at: started_at.map(|d| Utc.timestamp_millis(d)),
                ended_at: ended_at.map(|d| Utc.timestamp_millis(d)),
                succeeded: succeeded.map(|val| val.parse::<bool>()).transpose()?,
                error_details: error,
            })),
            _ => Ok(None),
        }
    }

    pub async fn get_job<T: DeserializeOwned + Send + Sync>(
        &self,
    ) -> Result<Option<QueueWorkItem<T>>, Error> {
        // 1. Run dequeue script
        let now = Utc::now();
        let mut conn = self.0.pool.get().await?;
        let result: Option<String> = self
            .0
            .dequeue_item_script
            .run(self, &mut conn, &now)
            .await?;

        // Unwrap the Option or just exit if there was no job.
        let job_id = match result {
            Some(id) => id,
            None => {
                return Ok(None);
            }
        };
        let job_id_key = self.job_data_key(&job_id);
        self.start_working(&mut conn, &job_id, &job_id_key, &now)
            .await
            .map(Some)
    }

    /// Cancel a job if it hasn't started yet.
    pub async fn cancel_pending_job(&self, id: &str) -> Result<JobStatus, Error> {
        let key = self.job_data_key(id);
        let mut conn = self.0.pool.get().await?;

        self.0
            .cancel_script
            .run(self, &mut conn, id, &key, &Utc::now(), false)
            .await
    }

    /// Cancel a job, and try to mark it cancelled even if it's running.
    pub async fn cancel_job(&self, id: &str) -> Result<JobStatus, Error> {
        let key = self.job_data_key(id);
        let mut conn = self.0.pool.get().await?;

        self.0
            .cancel_script
            .run(self, &mut conn, id, &key, &Utc::now(), true)
            .await
    }

    pub async fn update_job(
        &self,
        id: &str,
        run_at: Option<DateTime<Utc>>,
        new_payload: Option<&[u8]>,
    ) -> Result<bool, Error> {
        let key = self.job_data_key(id);
        let mut conn = self.0.pool.get().await?;

        self.0
            .update_script
            .run(self, &mut conn, id, &key, run_at, new_payload)
            .await
    }

    async fn done_job(&self, id: &str, expected_expiration: &DateTime<Utc>) -> Result<bool, Error> {
        let job_data_key = self.job_data_key(id);
        let now = Utc::now();

        let mut conn = self.0.pool.get().await?;
        self.0
            .done_script
            .run(
                self,
                &mut conn,
                id,
                &job_data_key,
                &now,
                expected_expiration,
            )
            .await
    }

    async fn errored_job(
        &self,
        id: &str,
        expected_expiration: &DateTime<Utc>,
        error: &str,
    ) -> Result<(), Error> {
        let job_data_key = self.job_data_key(id);
        let now = Utc::now();

        let mut conn = self.0.pool.get().await?;
        self.0
            .error_script
            .run(
                self,
                &mut conn,
                id,
                &job_data_key,
                &now,
                expected_expiration,
                error,
            )
            .await?;
        Ok(())
    }

    pub async fn job_expires_at(&self, id: &str) -> Result<Option<DateTime<Utc>>, Error> {
        let mut conn = self.0.pool.get().await?;
        let score: Option<i64> = redis::cmd("ZSCORE").arg(id).query_async(&mut conn).await?;
        Ok(score.map(|s| Utc.timestamp_millis(s)))
    }
}

impl Clone for Queue {
    fn clone(&self) -> Queue {
        Queue(self.0.clone())
    }
}

#[cfg(all(test))]
mod tests {
    use super::*;
    use crate::error::Error;
    use chrono::{Duration, DurationRound};
    use futures::{Future, FutureExt};
    use std::borrow::Cow;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct SimplePayload {
        data: String,
    }

    impl SimplePayload {
        fn generate() -> Result<Cow<'static, [u8]>, Error> {
            Self::with_value("A test string")
        }

        fn with_value(s: &str) -> Result<Cow<'static, [u8]>, Error> {
            let p = SimplePayload {
                data: s.to_string(),
            };
            Ok(Cow::Owned(serde_json::to_vec(&p)?))
        }
    }

    async fn run_queue_test<T, Fut, E>(test: T) -> ()
    where
        T: Send + Sync + FnOnce(Queue) -> Fut,
        Fut: Future<Output = Result<(), E>>,
        E: std::fmt::Debug,
    {
        dotenv::dotenv().ok();
        let queue_name = format!("test-{}", uuid::Uuid::new_v4());
        let pool = ergo_database::RedisPool::new(None, None).expect("Creating connection pool");
        let queue = Queue::new(pool.clone(), queue_name.clone(), None, None, None);

        let result = std::panic::AssertUnwindSafe(test(queue))
            .catch_unwind()
            .await;

        // Clean up the test keys.
        let mut conn = pool.get().await.expect("Cleanup: Acquiring connection");

        let key_pattern = format!("erq:{}:*", queue_name);
        let mut cmd = redis::cmd("SCAN");
        let mut iter: redis::AsyncIter<String> = cmd
            .cursor_arg(0)
            .arg("MATCH")
            .arg(&key_pattern)
            .arg("COUNT")
            .arg(100)
            .clone()
            .iter_async(&mut *conn)
            .await
            .expect("Cleanup: Scanning keyspace");

        let mut del_cmd = redis::cmd("DEL");
        while let Some(key) = iter.next_item().await {
            del_cmd.arg(&key);
        }

        del_cmd
            .query_async::<_, ()>(&mut conn)
            .await
            .expect("Cleanup: deleting keys");

        // Unwrap the results from catch_unwind and the test itself.
        result.expect("Panicked").expect("Error");
    }

    #[tokio::test]
    async fn enqueue() {
        run_queue_test(|queue| async move {
            let job = Job {
                id: String::from("a-test-id"),
                payload: SimplePayload::generate()?,
                ..Default::default()
            };
            queue.enqueue(&job).await?;

            match queue.get_job::<SimplePayload>().await? {
                Some(mut job) => {
                    job.process(|item, data| async move {
                        assert_eq!(item.id, "a-test-id");
                        assert_eq!(data.data, "A test string");
                        Ok::<(), Error>(())
                    })
                    .await?;
                }
                None => panic!("Did not see a job after enqueueing it"),
            }

            Ok::<(), Error>(())
        })
        .await;
    }

    #[tokio::test]
    async fn scheduled_task() {
        run_queue_test(|queue| async move {
            // Create a future date and round it so that it survives serialization fully intact.
            let initial_run_at = (Utc::now() + Duration::days(100))
                .duration_round(Duration::milliseconds(100))
                .expect("creating date");
            let id = "a-test".to_string();
            let job = Job {
                id: id.clone(),
                run_at: Some(initial_run_at.clone()),
                payload: SimplePayload::generate()?,
                ..Default::default()
            };

            queue.enqueue(&job).await.expect("enqueueing job");

            assert!(
                queue
                    .get_job::<SimplePayload>()
                    .await
                    .expect("retrieving job with empy pending list")
                    .is_none(),
                "future job should not be dequeued yet"
            );

            let scheduled = queue.list_scheduled().await?;
            println!("{:?}", scheduled);
            assert_eq!(
                scheduled[0],
                (id.clone(), initial_run_at),
                "initial schedule matches"
            );
            assert_eq!(scheduled.len(), 1, "just one scheduled task");

            let moved_to_pending = queue
                .enqueue_scheduled_items()
                .await
                .expect("Enqueueing scheduled items");
            assert_eq!(
                moved_to_pending, 0,
                "scheduled job in the future is not moved to pending list"
            );

            // Reschedule the job further in the future.
            let new_run_at = initial_run_at + Duration::days(50);
            queue
                .update_job(id.as_str(), Some(new_run_at.clone()), None)
                .await?;

            let rescheduled = queue.list_scheduled().await.expect("Listing scheduled");
            println!("Rescheduled: {:?}", rescheduled);
            assert_eq!(
                rescheduled[0],
                (id.clone(), new_run_at),
                "rescheduled task matches"
            );
            assert_eq!(rescheduled.len(), 1, "just one scheduled task");

            let moved_to_pending = queue
                .enqueue_scheduled_items()
                .await
                .expect("Enqueueing scheduled items");
            assert_eq!(
                moved_to_pending, 0,
                "scheduled job in the future is not moved to pending list"
            );

            // Reschedule the job in the past.
            let schedule_in_past = (Utc::now() - Duration::seconds(1))
                .duration_round(Duration::milliseconds(100))
                .expect("creating past date");
            let new_payload = SimplePayload::with_value("new value").unwrap();
            eprintln!("{:?}", new_payload);
            queue
                .update_job(
                    id.as_str(),
                    Some(schedule_in_past.clone()),
                    Some(new_payload.as_ref()),
                )
                .await
                .expect("Updating job and payload");

            let moved_to_pending = queue.enqueue_scheduled_items().await?;
            assert_eq!(moved_to_pending, 1, "ready job should be enqueued");

            let info = queue
                .job_info(&id)
                .await
                .expect("Getting job info")
                .expect("Job info should exist");
            println!("{:?}", info);
            assert_eq!(
                info.payload, br##"{"data":"new value"}"##,
                "payload is updated"
            );
            assert_eq!(
                info.run_at,
                Some(schedule_in_past),
                "date in job info is updated"
            );

            let job = queue
                .get_job::<SimplePayload>()
                .await
                .expect("Dequeueing ready job")
                .expect("Job should be ready");

            assert_eq!(
                job.data,
                Some(SimplePayload {
                    data: "new value".to_string()
                })
            );

            Ok::<(), Error>(())
        })
        .await;
    }
}
