use std::time::Duration;

use dotenv::dotenv;
use futures::future::try_join_all;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use tokio::{sync::watch, task::JoinHandle};

use ergo::{
    error::Error,
    graceful_shutdown::{GracefulShutdown, GracefulShutdownConsumer},
    queues::{Job, JobId, Queue},
};

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(
        long,
        required_unless = "num-jobs",
        help = "The amount of time, in seconds, to spend producing jobs"
    )]
    time: Option<u64>,
    #[structopt(
        short,
        long,
        required_unless = "time",
        conflicts_with = "time",
        help = "The number of jobs to produce"
    )]
    num_jobs: Option<usize>,
    #[structopt(short, long, help = "The number of job-producing workers to spawn")]
    producers: usize,
    #[structopt(short, long, help = "The number of job-consuming workers to spawn")]
    consumers: usize,
    #[structopt(
        short,
        long,
        help = "Produce all the jobs first and then consume them, instead of doing them concurrently"
    )]
    staged: bool,
    #[structopt(
        long,
        help = "The queue to run against. Normally you should omit this and let the tool generate its own queue id"
    )]
    queue: Option<String>,
}

enum JobLimit {
    Num(usize),
    Time(Duration),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();
    let args = Args::from_args();
    let redis_database = std::env::var("REDIS_URL").expect("REDIS_URL is required");
    let redis_pool = deadpool_redis::Config {
        url: Some(redis_database),
        pool: Some(deadpool_redis::PoolConfig::new(
            args.consumers + args.producers + 1,
        )),
    }
    .create_pool()
    .expect("Creating redis pool");

    let queue_name = args
        .queue
        .unwrap_or_else(|| format!("stress-{}", uuid::Uuid::new_v4()));
    let queue = Queue::new(redis_pool.clone(), &queue_name, None, None, None);

    let job_limit = match (args.num_jobs, args.time) {
        (Some(n), _) => JobLimit::Num(n),
        (_, Some(d)) => JobLimit::Time(Duration::from_secs(d)),
        _ => panic!("Neither num_jobs nor time were set"),
    };

    let shutdown = GracefulShutdown::new();

    let status_task = {
        let queue = queue.clone();
        let consumer = shutdown.consumer();
        tokio::spawn(async move { queue_status(queue, consumer).await })
    };

    let (close_consumers_tx, close_consumers_rx) = watch::channel::<bool>(false);
    if args.staged {
        // Generate all the jobs, then consume them.
        let generators = generate_jobs(
            queue.clone(),
            args.producers,
            shutdown.consumer(),
            close_consumers_tx,
            job_limit,
        );

        generators.await??;

        let consumers = consume_jobs(
            queue.clone(),
            args.consumers,
            shutdown.consumer(),
            close_consumers_rx,
            true,
        );

        consumers.await??;
    } else {
        // Generate and consume jobs at the same time.
        let generators = generate_jobs(
            queue.clone(),
            args.producers,
            shutdown.consumer(),
            close_consumers_tx,
            job_limit,
        );

        let consumers = consume_jobs(
            queue.clone(),
            args.consumers,
            shutdown.consumer(),
            close_consumers_rx,
            false,
        );

        let (gen_result, consumer_result) = tokio::try_join!(generators, consumers)?;
        gen_result?;
        consumer_result?;
    }

    shutdown.shutdown().await?;
    status_task.await??;

    cleanup(redis_pool, &queue_name).await?;

    Ok(())
}

async fn cleanup(pool: deadpool_redis::Pool, queue_name: &str) -> Result<(), Error> {
    let mut conn = pool.get().await.expect("Cleanup: Acquiring connection");
    let key_pattern = format!("erq:{}:*", queue_name);
    let mut cmd = deadpool_redis::cmd("SCAN");
    let mut iter: redis::AsyncIter<String> = cmd
        .cursor_arg(0)
        .arg("MATCH")
        .arg(&key_pattern)
        .arg("COUNT")
        .arg(100)
        .clone()
        .iter_async(&mut **conn)
        .await?;

    let mut del_cmd = deadpool_redis::cmd("DEL");
    while let Some(key) = iter.next_item().await {
        del_cmd.arg(&key);
    }

    del_cmd.execute_async(&mut conn).await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct JobPayload {
    data: String,
}

fn job_generator(
    queue: Queue,
    index: usize,
    num_jobs: usize,
    mut shutdown: GracefulShutdownConsumer,
    close: watch::Receiver<bool>,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        let data = serde_json::to_vec(&JobPayload {
            data: format!("Payload from generator {}", index),
        })?;

        for i in 0..num_jobs {
            let job = Job::from_bytes(
                JobId::Value(format!("w-{}-{}", index, i).as_str()),
                data.as_slice(),
            );

            queue.enqueue(&job).await?;

            if shutdown.shutting_down() || *close.borrow() {
                break;
            }
        }

        Ok(())
    })
}

fn generate_jobs(
    queue: Queue,
    num_workers: usize,
    mut shutdown: GracefulShutdownConsumer,
    done_generating: watch::Sender<bool>,
    limit: JobLimit,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        let total_num_jobs = match limit {
            JobLimit::Num(n) => n,
            _ => usize::MAX,
        };

        let jobs_per_worker = total_num_jobs / num_workers;
        let round_up = total_num_jobs % num_workers;

        let (close_workers_tx, close_workers_rx) = watch::channel(false);

        let workers = (0..num_workers)
            .into_iter()
            .map(|i| {
                let mut num_jobs = jobs_per_worker;
                if i < round_up {
                    num_jobs += 1;
                }

                job_generator(
                    queue.clone(),
                    i,
                    num_jobs,
                    shutdown.clone(),
                    close_workers_rx.clone(),
                )
            })
            .collect::<Vec<_>>();

        match limit {
            JobLimit::Time(t) => {
                tokio::select! {
                    _ = tokio::time::sleep(t) => {},
                    _ = shutdown.wait_for_shutdown() => {}
                };

                close_workers_tx
                    .send(true)
                    .expect("Setting close_workers_tx");
                try_join_all(workers).await?;
            }
            JobLimit::Num(_) => {
                // The workers have their limit built in, so just wait for them to finish.
                tokio::select! {
                    result = try_join_all(workers) => {
                        result?.into_iter().collect::<Result<Vec<()>, Error>>()?;
                    },
                    _ = shutdown.wait_for_shutdown() => {}
                };
            }
        };

        done_generating
            .send(true)
            .map_err(|_| Error::StringError("Failed to close consumer channel".to_string()))?;

        Ok(())
    })
}

fn consume_jobs(
    queue: Queue,
    num_workers: usize,
    shutdown: GracefulShutdownConsumer,
    close_consumers: watch::Receiver<bool>,
    close_on_idle: bool,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        let workers = (0..num_workers)
            .into_iter()
            .map(|_| {
                job_consumer(
                    queue.clone(),
                    shutdown.clone(),
                    close_consumers.clone(),
                    close_on_idle,
                )
            })
            .collect::<Vec<_>>();

        try_join_all(workers)
            .await?
            .into_iter()
            .collect::<Result<Vec<()>, Error>>()?;

        Ok(())
    })
}

fn job_consumer(
    queue: Queue,
    mut shutdown: GracefulShutdownConsumer,
    close_consumers: watch::Receiver<bool>,
    mut close_on_idle: bool,
) -> JoinHandle<Result<(), Error>> {
    tokio::spawn(async move {
        loop {
            match queue.get_job::<JobPayload>().await? {
                Some(job) => {
                    job.process(|_, _| async move { Ok::<(), Error>(()) })
                        .await?;
                }
                None => {
                    if close_on_idle {
                        break;
                    } else {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            };

            if shutdown.shutting_down() {
                break;
            } else if *close_consumers.borrow() == true {
                close_on_idle = true;
            }
        }

        Ok(())
    })
}

async fn queue_status(queue: Queue, mut shutdown: GracefulShutdownConsumer) -> Result<(), Error> {
    let bars = MultiProgress::new();
    bars.set_move_cursor(true);

    let pending_bar = ProgressBar::new(u64::MAX)
        .with_style(ProgressStyle::default_spinner().template("{spinner} {pos} jobs pending"));
    let running_bar = ProgressBar::new(u64::MAX)
        .with_style(ProgressStyle::default_spinner().template("{spinner} {pos} jobs running"));
    let enqueued_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs enqueued ({per_sec})"),
    );
    let retrieved_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs retrieved ({per_sec})"),
    );
    let done_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs finished ({per_sec})"),
    );
    let error_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs errored ({per_sec})"),
    );

    bars.add(pending_bar.clone());
    bars.add(running_bar.clone());
    bars.add(enqueued_bar.clone());
    bars.add(retrieved_bar.clone());
    bars.add(done_bar.clone());
    bars.add(error_bar.clone());

    let update_task = tokio::task::spawn(async move {
        let mut exit = false;
        let mut interval = tokio::time::interval(Duration::from_millis(500));

        while !exit {
            tokio::select! {
                _ = interval.tick() => {},
                _ = shutdown.wait_for_shutdown() => {
                    exit = true;
                },
            };

            match queue.status().await {
                Ok(status) => {
                    pending_bar.set_position(status.current_pending as u64);
                    running_bar.set_position(status.current_running as u64);
                    enqueued_bar.set_position(status.total_enqueued as u64);
                    retrieved_bar.set_position(status.total_retrieved as u64);
                    done_bar.set_position(status.total_succeeded as u64);
                    error_bar.set_position(status.total_errored as u64);
                }
                Err(_) => {
                    break;
                }
            };
        }

        pending_bar.finish_at_current_pos();
        running_bar.finish_at_current_pos();
        enqueued_bar.finish_at_current_pos();
        retrieved_bar.finish_at_current_pos();
        done_bar.finish_at_current_pos();
        error_bar.finish_at_current_pos();

        Ok::<(), Error>(())
    });

    tokio::task::spawn_blocking(move || {
        bars.join().expect("MultiProgressBar join");
    })
    .await?;

    update_task.await??;

    Ok(())
}
