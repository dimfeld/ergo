use std::{borrow::Cow, time::Duration};

use chrono::{DateTime, Utc};
use dotenv::dotenv;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use structopt::StructOpt;

use ergo::{
    error::Error,
    graceful_shutdown::GracefulShutdownConsumer,
    queues::{Job, Queue},
};

#[derive(Debug, StructOpt)]
struct Args {
    #[structopt(
        long,
        required_unless = "num_jobs",
        help = "The amount of time, in seconds, to spend producing jobs"
    )]
    time: Option<i64>,
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
    Time(DateTime<Utc>),
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
    let queue = Queue::new(redis_pool, &queue_name, None, None, None);

    let job_limit = match (args.num_jobs, args.time) {
        (Some(n), _) => JobLimit::Num(n),
        (_, Some(d)) => {
            let deadline = Utc::now() + chrono::Duration::seconds(d);
            JobLimit::Time(deadline)
        }
        _ => panic!("Neither num_jobs nor time were set"),
    };

    Ok(())
}

async fn queue_status(queue: Queue, mut shutdown: GracefulShutdownConsumer) -> Result<(), Error> {
    let bars = MultiProgress::new();
    bars.set_move_cursor(true);

    let pending_bar = ProgressBar::new(u64::MAX)
        .with_style(ProgressStyle::default_spinner().template("{spinner} {pos} jobs pending"));
    let running_bar = ProgressBar::new(u64::MAX)
        .with_style(ProgressStyle::default_spinner().template("{spinner} {pos} jobs running"));
    let enqueued_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs enqueued ({per_sec}/s)"),
    );
    let retrieved_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs retrieved ({per_sec}/s)"),
    );
    let done_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs finished ({per_sec}/s)"),
    );
    let error_bar = ProgressBar::new(u64::MAX).with_style(
        ProgressStyle::default_spinner().template("{spinner} {pos} jobs errored ({per_sec}/s)"),
    );

    bars.add(pending_bar.clone());
    bars.add(running_bar.clone());
    bars.add(enqueued_bar.clone());
    bars.add(retrieved_bar.clone());
    bars.add(done_bar.clone());
    bars.add(error_bar.clone());

    loop {
        let status = queue.status().await?;

        pending_bar.set_position(status.current_pending as u64);
        running_bar.set_position(status.current_running as u64);
        enqueued_bar.set_position(status.total_enqueued as u64);
        retrieved_bar.set_position(status.total_retrieved as u64);
        done_bar.set_position(status.total_succeeded as u64);
        error_bar.set_position(status.total_errored as u64);

        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {},
            _ = shutdown.wait_for_shutdown() => break,
        };
    }

    bars.join().expect("MultiProgressBar join");

    Ok(())
}

async fn run_job(queue: &Queue, delay: Option<u64>, error: Option<String>) -> Result<(), Error> {
    let job = queue.get_job::<Box<serde_json::value::Value>>().await?;

    match job {
        None => {
            println!("No jobs waiting to run");
            Ok(())
        }
        Some(job) => {
            job.process(|id, payload| async move {
                println!("Got job {} with payload {}", id, payload.clone());

                if let Some(d) = delay {
                    println!("Sleeping for {}ms", d);
                    tokio::time::sleep(Duration::from_millis(d)).await;
                }

                match error {
                    None => {
                        println!("Finishing job");
                        Ok(())
                    }
                    Some(e) => {
                        println!("Finishing job {} with error {}", id, e);
                        Err(Error::StringError(e))
                    }
                }
            })
            .await
        }
    }
}
