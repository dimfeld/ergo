use std::{borrow::Cow, time::Duration};

use structopt::StructOpt;

use crate::{
    error::Error,
    queues::{Job, Queue},
};

#[derive(Debug, StructOpt)]
pub struct Args {
    queue: String,
    #[structopt(subcommand)]
    cmd: QueueCmd,
}

#[derive(Debug, StructOpt)]
enum QueueCmd {
    #[structopt(about = "Add a job to the queue")]
    Add { id: String, data: String },
    #[structopt(about = "Show information about the queue")]
    Show,
    #[structopt(name = "show-job", about = "Show information about a job")]
    ShowJob { id: String },
    #[structopt(
        about = "Get and acknowledge the next job on the queue. (Don't use this in production)"
    )]
    Run {
        #[structopt(
            short,
            long,
            help = "Wait this long, in milliseconds, before processing the job"
        )]
        delay: Option<u64>,
        #[structopt(
            short,
            long,
            help = "Report this error instead of marking the job succeeded"
        )]
        error: Option<String>,
    },
    #[structopt(about = "Cancel a job")]
    Cancel { id: String },
    #[structopt(about = "Run a stress test on the queue system")]
    Stress(super::erq_stress::Args),
}

pub async fn main(args: Args) -> Result<(), Error> {
    let redis_pool = ergo_database::RedisPool::new(None, None).expect("Creating redis pool");

    let queue = Queue::new(redis_pool, args.queue.clone(), None, None, None);

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
            let status = queue.status().await?;
            println!("{:?}", status);
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
        QueueCmd::Stress(stress_args) => super::erq_stress::main(args.queue, stress_args).await?,
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
            job.process(|item| async move {
                println!("Got job {} with payload {}", item.id, item.data.clone());

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
                        println!("Finishing job {} with error {}", item.id, e);
                        Err(Error::StringError(e))
                    }
                }
            })
            .await
        }
    }
}
