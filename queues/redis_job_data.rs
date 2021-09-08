use chrono::{DateTime, Utc};
use std::time::Duration;

pub(super) enum RedisJobField {
    Payload,
    Timeout,
    CurrentRetries,
    MaxRetries,
    RetryBackoff,
    RunAt,
    EnqueuedAt,
    StartedAt,
    EndedAt,
    Succeeded,
    ErrorDetails,
}

impl RedisJobField {
    const fn as_str(&self) -> &'static str {
        match self {
            RedisJobField::Payload => "pay",
            RedisJobField::Timeout => "to",
            RedisJobField::CurrentRetries => "cr",
            RedisJobField::MaxRetries => "mr",
            RedisJobField::RetryBackoff => "bo",
            RedisJobField::RunAt => "ra",
            RedisJobField::EnqueuedAt => "qt",
            RedisJobField::StartedAt => "st",
            RedisJobField::EndedAt => "end",
            RedisJobField::Succeeded => "suc",
            RedisJobField::ErrorDetails => "err",
        }
    }
}

impl redis::ToRedisArgs for RedisJobField {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        out.write_arg(self.as_str().as_bytes())
    }
}

pub(super) struct RedisJobSetCmd(redis::Cmd);

impl RedisJobSetCmd {
    pub fn new(job_key: &str) -> Self {
        let mut cmd = redis::cmd("HSET");
        cmd.arg(job_key);
        RedisJobSetCmd(cmd)
    }

    pub fn build(self) -> redis::Cmd {
        self.0
    }

    pub fn increment_current_retries(job_key: &str) -> redis::Cmd {
        let mut cmd = redis::cmd("hincrby");
        cmd.arg(job_key).arg(RedisJobField::CurrentRetries).arg(1);
        cmd
    }

    pub fn payload(mut self, s: &[u8]) -> Self {
        self.0.arg(RedisJobField::Payload).arg(s);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.0
            .arg(RedisJobField::Timeout)
            .arg(timeout.as_millis() as u64);
        self
    }

    pub fn current_retries(mut self, retries: u32) -> Self {
        self.0.arg(RedisJobField::CurrentRetries).arg(retries);
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.0.arg(RedisJobField::MaxRetries).arg(retries);
        self
    }

    pub fn retry_backoff(mut self, backoff: Duration) -> Self {
        self.0
            .arg(RedisJobField::RetryBackoff)
            .arg(backoff.as_millis() as u64);
        self
    }

    pub fn run_at(mut self, run_at: &DateTime<Utc>) -> Self {
        self.0
            .arg(RedisJobField::RunAt)
            .arg(run_at.timestamp_millis() as u64);
        self
    }

    pub fn enqueued_at(mut self, enqueued_at: &DateTime<Utc>) -> Self {
        self.0
            .arg(RedisJobField::EnqueuedAt)
            .arg(enqueued_at.timestamp_millis());
        self
    }

    pub fn started_at(mut self, started_at: &DateTime<Utc>) -> Self {
        self.0
            .arg(RedisJobField::StartedAt)
            .arg(started_at.timestamp_millis());
        self
    }

    pub fn ended_at(mut self, ended_at: &DateTime<Utc>) -> Self {
        self.0
            .arg(RedisJobField::EndedAt)
            .arg(ended_at.timestamp_millis());
        self
    }

    pub fn clear_succeeded(mut self) -> Self {
        self.0.arg(RedisJobField::Succeeded).arg("");
        self
    }

    pub fn succeeded(mut self, succeeded: bool) -> Self {
        self.0.arg(RedisJobField::Succeeded).arg(succeeded);
        self
    }

    pub fn error_details(mut self, error: &str) -> Self {
        self.0.arg(RedisJobField::ErrorDetails).arg(error);
        self
    }
}
