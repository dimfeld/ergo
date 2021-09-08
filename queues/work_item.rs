use super::Queue;
use crate::error::Error;
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use std::future::Future;

#[derive(Debug)]
pub struct QueueWorkItem<T: Send + Sync> {
    queue: Queue,
    pub id: String,
    pub data: T,
    pub expires: DateTime<Utc>,

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
    pub(super) fn new(
        queue: Queue,
        job_id: &str,
        expires: DateTime<Utc>,
        data: Vec<u8>,
    ) -> Result<Self, Error> {
        let converted: T = serde_json::from_slice(data.as_slice())?;
        Ok(QueueWorkItem {
            queue,
            id: String::from(job_id),
            data: converted,
            expires,
            finished: false,
        })
    }
}

impl<'a, T: Send + Sync> QueueWorkItem<T> {
    pub async fn process<F, Fut, R, E>(&'a self, f: F) -> Result<R, Error>
    where
        F: FnOnce(&'a Self) -> Fut,
        Fut: Future<Output = Result<R, E>>,
        T: Send,
        E: 'static + std::error::Error + Send + Sync,
    {
        match f(&self).await {
            Ok(val) => {
                self.queue.done_job(self.id.as_str(), &self.expires).await?;
                Ok(val)
            }
            Err(e) => {
                let e = anyhow!(e);
                self.queue
                    .errored_job(self.id.as_str(), &self.expires, &e.to_string().as_str())
                    .await?;
                Err(Error::JobError(e))
            }
        }
    }

    /// Check if this job is still active and owned by us. Can be useful for long-running jobs
    /// that may want to cancel.
    pub async fn active(&self) -> Result<bool, Error> {
        match self.queue.job_expires_at(&self.id).await? {
            Some(e) => Ok(e == self.expires),
            None => Ok(false),
        }
    }
}
