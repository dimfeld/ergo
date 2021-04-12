use ergo::{error::Error, queues::Queue};
use structopt::StructOpt;

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
    Finish {
        id: String,
    },
    Error {
        id: String,
        err: String,
    },
    Del {
        id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::from_args();
    let redis_database = std::env::var("REDIS_URL").expect("REDIS_URL is required");
    let redis_pool = deadpool_redis::Config {
        url: Some(redis_database),
        pool: None,
    }
    .create_pool()
    .expect("Creating redis pool");

    let queue = Queue::new(redis_pool, &args.queue, None, None, None);
    Ok(())
}
