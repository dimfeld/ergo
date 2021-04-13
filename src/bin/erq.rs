use std::{borrow::Cow, time::Duration};

use dotenv::dotenv;
use structopt::StructOpt;

use ergo::{
    error::Error,
    queues::{Job, Queue},
};

#[derive(Debug, StructOpt)]
struct Args {
    queue: String,
    #[structopt(subcommand)]
    cmd: QueueCmd,
}

#[derive(Debug, StructOpt)]
enum QueueCmd {
    Add {
        id: String,
        data: String,
    },
    Show,
    #[structopt(name = "show-job")]
    ShowJob {
        id: String,
    },
    Run {
        #[structopt(short, long, help = "Processing delay in milliseconds")]
        delay: Option<u64>,
        #[structopt(
            short,
            long,
            help = "Report this error instead of marking the job succeeded"
        )]
        error: Option<String>,
    },
    Cancel {
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();
    let args = Args::from_args();
    let redis_database = std::env::var("REDIS_URL").expect("REDIS_URL is required");
    let redis_pool = deadpool_redis::Config {
        url: Some(redis_database),
        pool: None,
    }
    .create_pool()
    .expect("Creating redis pool");

    let queue = Queue::new(redis_pool, &args.queue, None, None, None);

    match args.cmd {
        QueueCmd::Add { id, data } => {
            let bytes = data.into_bytes();
            let job = Job {
                id,
                payload: Cow::Owned(bytes),
                ..Default::default()
            };
            queue.enqueue(&job).await?;
        }
        QueueCmd::Show => {
            unimplemented!();
        }
        QueueCmd::Cancel { id } => {
            queue.cancel_job(&id).await?;
        }
        QueueCmd::Run { delay, error } => {
            run_job(&queue, delay, error).await?;
        }
        QueueCmd::ShowJob { id } => match queue.job_info(&id).await? {
            Some(job) => println!("{:?}", job),
            None => println!("Job not found"),
        },
    }
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
