use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{borrow::Cow, time::Duration};

#[derive(Default)]
pub struct Job<'a> {
    pub id: String,
    pub payload: Cow<'a, [u8]>,
    pub timeout: Option<Duration>,
    pub max_retries: Option<u32>,
    pub run_at: Option<DateTime<Utc>>,
    pub retry_backoff: Option<Duration>,
}

impl<'a> std::fmt::Debug for Job<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("payload", &String::from_utf8_lossy(&self.payload))
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("run_at", &self.run_at)
            .field("retry_backoff", &self.retry_backoff)
            .finish()
    }
}

/// Determines how to generate a job ID
pub enum JobId<'a> {
    /// Autogenerate a v4 UUID.
    Auto,
    /// Concatenate the given prefix with a v4 UUID.
    Prefix(&'a str),
    /// Use this value as the job ID. It's up to you to make sure that the ID
    /// is unique.
    Value(&'a str),
}

impl<'a> JobId<'a> {
    pub fn make_id(&self) -> String {
        match self {
            JobId::Auto => uuid::Uuid::new_v4().to_string(),
            JobId::Prefix(prefix) => format!("{}:{}", prefix, uuid::Uuid::new_v4()),
            JobId::Value(s) => s.to_string(),
        }
    }
}

impl<'a> Job<'a> {
    pub fn from_bytes(id: JobId<'_>, bytes: &'a [u8]) -> Job<'a> {
        Job {
            id: id.make_id(),
            payload: Cow::Borrowed(bytes),
            ..Default::default()
        }
    }

    pub fn from_json_payload<T: Serialize>(
        id: JobId<'_>,
        payload: &T,
    ) -> Result<Job<'static>, serde_json::Error> {
        let data = serde_json::to_vec(&payload)?;
        Ok(Job {
            id: id.make_id(),
            payload: Cow::Owned(data),
            ..Default::default()
        })
    }
}
