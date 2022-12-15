use std::{fmt::Debug, pin::Pin, sync::Arc};

use futures::{
    future::{ready, FutureExt},
    Future,
};
use tokio::{sync::oneshot, time::error::Elapsed};

lazy_static::lazy_static! {
    static ref NUM_CPUS : usize = num_cpus::get();
}

#[async_trait::async_trait]
trait AnyJob: Send {
    fn run(&mut self) -> Pin<Box<dyn Future<Output = ()>>>;
}

struct Job<RESULT, Fut, F>
where
    RESULT: Send + Debug + 'static,
    Fut: Future<Output = RESULT> + 'static,
    F: (FnOnce() -> Fut) + Send + 'static,
{
    data: Option<(Box<F>, oneshot::Sender<RESULT>)>,
}

#[async_trait::async_trait]
impl<RESULT, Fut, F> AnyJob for Job<RESULT, Fut, F>
where
    RESULT: Send + Debug + 'static,
    Fut: Future<Output = RESULT> + 'static,
    F: (FnOnce() -> Fut) + Send + 'static,
{
    fn run(&mut self) -> Pin<Box<dyn Future<Output = ()>>> {
        let (data, output_sender) = self.data.take().unwrap();

        (data)()
            .then(|result| {
                output_sender.send(result).ok();
                ready(())
            })
            .boxed_local()
    }
}

#[derive(Clone)]
pub struct RuntimePool(Arc<RuntimePoolInner>);

struct RuntimePoolInner {
    sender: async_channel::Sender<Box<dyn AnyJob>>,
    threads: Vec<std::thread::JoinHandle<()>>,
}

impl std::fmt::Debug for RuntimePoolInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimePoolInner").finish_non_exhaustive()
    }
}

// TODO This needs a lot of unwrap cleanup.
impl RuntimePool {
    pub fn new(num_threads: Option<usize>) -> Self {
        let num_threads = num_threads.unwrap_or(*NUM_CPUS);
        let (s, r) = async_channel::unbounded();

        let threads = itertools::repeat_n(r, num_threads)
            .map(|r| std::thread::spawn(|| worker(r)))
            .collect::<Vec<_>>();

        Self(Arc::new(RuntimePoolInner { sender: s, threads }))
    }

    /// Shut down the pool and wait for all the threads to finish processing the remaining jobs.
    pub async fn close(self, timeout: Option<tokio::time::Duration>) -> Result<(), Elapsed> {
        let RuntimePoolInner { sender, threads } = Arc::try_unwrap(self.0).unwrap();
        let stop = tokio::task::spawn_blocking(move || {
            drop(sender);
            for t in threads {
                t.join();
            }
        });

        match timeout {
            Some(d) => {
                tokio::time::timeout(d, stop).await?;
            }
            None => {
                stop.await;
            }
        };

        Ok(())
    }

    pub async fn run<F, Fut, RESULT>(self: &RuntimePool, run_fn: F) -> RESULT
    where
        F: (FnOnce() -> Fut) + Send + 'static,
        Fut: Future<Output = RESULT> + 'static,
        RESULT: Send + Debug + 'static,
    {
        let (s, r) = oneshot::channel();
        let job = Job {
            data: Some((Box::new(run_fn), s)),
        };

        self.0.sender.send(Box::new(job)).await;
        r.await.unwrap()
    }
}

fn worker(r: async_channel::Receiver<Box<dyn AnyJob>>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let local_set = tokio::task::LocalSet::new();
        local_set.spawn_local(async move {
            while let Ok(mut job) = r.recv().await {
                tokio::task::spawn_local(async move {
                    job.run().await;
                });
            }
        });

        local_set.await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Runtime, RuntimeOptions};

    #[tokio::test]
    async fn run_job() {
        let pool = RuntimePool::new(Some(2));

        let script = r##"
            async function doIt() {
                return 5;
            }

            doIt().then((value) => {
                globalThis.result = value;
            })
        "##;

        let ret_val = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            pool.run(move || async move {
                let mut runtime = Runtime::new(RuntimeOptions::default());
                runtime
                    .execute_script("script", script)
                    .expect("script ran");
                runtime.run_event_loop(false).await.expect("run_event_loop");
                runtime
                    .get_global_value::<usize>("result")
                    .unwrap()
                    .unwrap()
            }),
        )
        .await
        .expect("run timed out");

        pool.close(Some(tokio::time::Duration::from_secs(10)))
            .await
            .expect("close timed out");

        assert_eq!(ret_val, 5);
    }
}
