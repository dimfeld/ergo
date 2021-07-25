use ergo_api::{cmd, error};
use structopt::StructOpt;

#[derive(StructOpt)]
enum Args {
    #[structopt(about = "Run the Ergo server")]
    Server(cmd::server::Args),
    #[structopt(about = "Run a task that only drains the Postgres queues")]
    DrainQueues,
    #[structopt(about = "Development commands")]
    Dev(DevCmds),
}

#[derive(StructOpt)]
enum DevCmds {
    #[structopt(about = "Create a password hash")]
    HashPassword(cmd::hash_passwd::Args),
    #[structopt(about = "Create an API key")]
    MakeApiKey(cmd::make_api_key::Args),
    #[structopt(about = "Regenerate the JSON schema files")]
    MakeJsonSchema,
    #[structopt(about = "Examine the task queues")]
    Queue(cmd::erq::Args),
}

#[actix_web::main]
async fn main() -> Result<(), error::Error> {
    dotenv::dotenv().ok();
    dotenv::from_filename("vault_dev_roles.env").ok();

    let args = Args::from_args();

    match args {
        Args::Server(s) => cmd::server::main(s).await,
        Args::DrainQueues => cmd::drain_queues::main().await,
        Args::Dev(cmd) => match cmd {
            DevCmds::HashPassword(args) => cmd::hash_passwd::main(args),
            DevCmds::MakeApiKey(args) => cmd::make_api_key::main(args).await,
            DevCmds::MakeJsonSchema => cmd::make_json_schema::main(),
            DevCmds::Queue(args) => cmd::erq::main(args).await,
        },
    }?;

    Ok(())
}
