use std::{any::Any, fmt::Debug, pin::Pin};

use futures::{
    future::{ready, FutureExt},
    stream::StreamExt,
    Future,
};
use tokio::sync::oneshot;

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
    F: (Fn() -> Fut) + Send + 'static,
{
    data: Box<F>,
    output_sender: Option<oneshot::Sender<RESULT>>,
}

#[async_trait::async_trait]
impl<RESULT, Fut, F> AnyJob for Job<RESULT, Fut, F>
where
    RESULT: Send + Debug + 'static,
    Fut: Future<Output = RESULT> + 'static,
    F: (Fn() -> Fut) + Send + 'static,
{
    fn run(&mut self) -> Pin<Box<dyn Future<Output = ()>>> {
        let output_sender = self.output_sender.take().unwrap();

        (self.data)()
            .then(|result| {
                output_sender.send(result).ok();
                ready(())
            })
            .boxed_local()
    }
}

pub struct RuntimePool {
    sender: async_channel::Sender<Box<dyn AnyJob>>,
}

impl RuntimePool {
    pub fn new(num_threads: Option<usize>) -> Self {
        let num_threads = num_threads.unwrap_or_else(|| num_cpus::get());
        let (s, r) = async_channel::unbounded();

        let threads = itertools::repeat_n(r, num_threads)
            .map(|r| std::thread::spawn(|| worker(r)))
            .collect::<Vec<_>>();

        Self { sender: s }
    }

    pub async fn run<F, Fut, RESULT>(self: &RuntimePool, run_fn: F) -> RESULT
    where
        F: (Fn() -> Fut) + Send + 'static,
        Fut: Future<Output = RESULT> + 'static,
        RESULT: Send + Debug + 'static,
    {
        let (s, r) = oneshot::channel();
        let job = Job {
            data: Box::new(run_fn),
            output_sender: Some(s),
        };

        self.sender.send(Box::new(job)).await;
        r.await.unwrap()
    }
}

fn worker(r: async_channel::Receiver<Box<dyn AnyJob>>) {
    let runtime = tokio::runtime::Handle::current();
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
