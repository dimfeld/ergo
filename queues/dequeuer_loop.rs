use async_trait::async_trait;
use backoff::backoff::Backoff;
use ergo_graceful_shutdown::GracefulShutdownConsumer;
use futures::{
    future::ready,
    stream::{FuturesUnordered, StreamExt},
};
use serde::de::DeserializeOwned;
use tokio::{sync::oneshot, task::JoinHandle};
use tracing::{event, Level};

use std::time::Duration;

use super::QueueWorkItem;
use crate::error::Error;

#[async_trait]
pub trait QueueJobProcessor: Clone + Sync + Send {
    type Payload: DeserializeOwned + Send + Sync;
    type Error: Send + Sync + std::error::Error;

    async fn process(&self, item: &QueueWorkItem<Self::Payload>) -> Result<(), Self::Error>;
}

pub fn dequeuer_loop<P, T>(
    queue: super::Queue,
    mut shutdown: GracefulShutdownConsumer,
    closer_rx: oneshot::Receiver<()>,
    mut backoff: Box<dyn Backoff + Send>,
    max_jobs: usize,
    processor: P,
) -> JoinHandle<()>
where
    P: QueueJobProcessor<Payload = T> + 'static,
    T: DeserializeOwned + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let shutdown_fut = shutdown.wait_for_shutdown();
        tokio::pin!(shutdown_fut);
        tokio::pin!(closer_rx);

        let mut active_tasks = FuturesUnordered::<JoinHandle<()>>::new();
        let mut sleep_time = Duration::default();

        loop {
            let wait_for_task = active_tasks.len() >= max_jobs;
            let do_backoff = sleep_time > Duration::default();
            if wait_for_task || do_backoff {
                tokio::select! {
                    biased;

                    _ = &mut shutdown_fut => break,
                    _ = &mut closer_rx => break,
                    res = active_tasks.select_next_some(), if wait_for_task => {
                        if let Err(e) = res {
                            event!(Level::ERROR, error=%e, "Job task panicked");
                        }
                    },
                    _ = tokio::time::sleep(sleep_time), if do_backoff => {},
                };
            }

            match queue.get_job::<T>().await {
                Ok(Some(job)) => {
                    backoff.reset();
                    sleep_time = Duration::default();

                    let p = processor.clone();
                    let queue_name = queue.0.name.clone();
                    let job_task = tokio::spawn(async move {
                        match job.process(|item| p.process(item)).await {
                            Ok(_) => {}
                            Err(e) => {
                                event!(Level::ERROR, error=?e, job=%job.id, queue=%queue_name, "Job error");
                            }
                        };
                    });
                    active_tasks.push(job_task);
                }
                Ok(None) => match backoff.next_backoff() {
                    Some(next_sleep_time) => {
                        sleep_time = next_sleep_time;
                    }
                    None => break,
                },
                Err(e) => {
                    event!(Level::ERROR, error=%e, queue=%queue.0.name, "Error dequeueing job");
                    match backoff.next_backoff() {
                        Some(next_sleep_time) => {
                            sleep_time = next_sleep_time;
                        }
                        None => break,
                    }
                }
            }

            // Make sure we call this periodically so that futures are processed.
            tokio::select! {
                biased;
                r = active_tasks.next() => match r {
                    Some(Err(e)) => {
                        event!(Level::ERROR, error=%e, "Job task panicked");
                    }
                    _ => {}
                },
                _ = ready(()) => {}
            };
        }
    })
}
