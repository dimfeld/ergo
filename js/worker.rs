use std::{fmt::Debug, future::ready, pin::Pin};

use futures::{future::FutureExt, Future};
use tokio::{sync::oneshot, task::JoinError};

use crate::{BufferConsole, Runtime, RuntimeOptions};

trait WorkerJob: Send {
    fn run<'run>(&mut self, runtime: &'run mut Runtime) -> JobFuture<'run, ()>;
}

type JobFuture<'a, RESULT> = Pin<Box<dyn Future<Output = RESULT> + 'a>>;

struct Job<RESULT, F>
where
    RESULT: Send + Debug + 'static,
    for<'a> F: (FnOnce(&'a mut Runtime) -> JobFuture<'a, RESULT>) + Send + 'static,
{
    data: Option<(Box<F>, oneshot::Sender<RESULT>)>,
}

impl<RESULT, F> WorkerJob for Job<RESULT, F>
where
    RESULT: Send + Debug + 'static,
    for<'a> F: (FnOnce(&'a mut Runtime) -> JobFuture<'a, RESULT>) + Send + 'static,
{
    fn run<'run>(&mut self, runtime: &'run mut Runtime) -> JobFuture<'run, ()> {
        let (func, output_sender) = self.data.take().unwrap();

        (func)(runtime)
            .then(|result| {
                output_sender.send(result).ok();
                ready(())
            })
            .boxed_local()
    }
}

/// A worker that lets you reuse the same runtime across calls.
pub struct JsWorker {
    worker_thread: std::thread::JoinHandle<()>,
    sender: async_channel::Sender<Box<dyn WorkerJob>>,
}

impl JsWorker {
    pub fn new() -> JsWorker {
        let (sender, receiver) = async_channel::bounded(1);
        let worker_thread = std::thread::spawn(|| worker(receiver));

        JsWorker {
            worker_thread,
            sender,
        }
    }

    pub async fn run<F, RESULT>(&self, run_fn: F) -> RESULT
    where
        RESULT: Send + Debug + 'static,
        for<'a> F: (FnOnce(&'a mut Runtime) -> JobFuture<'a, RESULT>) + Send + 'static,
    {
        let (s, r) = oneshot::channel();
        let job = Job {
            data: Some((Box::new(run_fn), s)),
        };

        self.sender
            .send(Box::new(job))
            .await
            .expect("Sent job after worker closed");
        r.await.unwrap()
    }

    /// Close the worker. This can be used if you want to wait until you are sure the
    /// worker has closed, but just dropping the worker is also OK.
    pub async fn close(self) -> Result<std::thread::Result<()>, JoinError> {
        // TODO Clean up the return type since it's a hassle to deal with.
        let Self {
            sender,
            worker_thread,
        } = self;
        drop(sender);
        tokio::task::spawn_blocking(move || worker_thread.join()).await
    }
}

impl Default for JsWorker {
    fn default() -> Self {
        Self::new()
    }
}

fn worker(r: async_channel::Receiver<Box<dyn WorkerJob>>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let local_set = tokio::task::LocalSet::new();
        local_set.spawn_local(async move {
            let mut runtime = Runtime::new(RuntimeOptions {
                console: Some(Box::new(BufferConsole::new(crate::ConsoleLevel::Info))),
                ..Default::default()
            });
            while let Ok(mut job) = r.recv().await {
                job.run(&mut runtime).await;
            }
        });

        local_set.await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_job() {
        let worker = JsWorker::new();

        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            worker.run(move |runtime| {
                async move {
                    let init_script = r##"
                    function doIt(val) {
                        return val + 1;
                    }
                "##;

                    runtime.execute_script("init_script", init_script).unwrap();
                    Ok::<(), ()>(())
                }
                .boxed_local()
            }),
        )
        .await
        .expect("run timed out")
        .expect("Script error");

        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            worker.run(|runtime| {
                async move {
                    runtime.execute_script("call_function", "globalThis.pval = doIt(1);")?;
                    Ok::<(), crate::Error>(())
                }
                .boxed_local()
            }),
        )
        .await
        .expect("run timed out")
        .expect("Script error");

        tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            worker.run(|runtime| {
                async move {
                    runtime.execute_script(
                        "call_function",
                        "globalThis.pval = doIt(globalThis.pval);",
                    )?;
                    Ok::<(), crate::Error>(())
                }
                .boxed_local()
            }),
        )
        .await
        .expect("run timed out")
        .expect("Script error");

        let output_value = tokio::time::timeout(
            tokio::time::Duration::from_secs(5),
            worker.run(|runtime| {
                async move { runtime.get_global_value::<i32>("pval") }.boxed_local()
            }),
        )
        .await
        .expect("run timed out")
        .expect("Script error");

        worker
            .close()
            .await
            .expect("close timed out")
            .expect("close failed");

        assert_eq!(output_value, Some(3));
    }
}
