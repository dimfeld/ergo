use ergo_database::database_configuration_from_env;
use ergo_graceful_shutdown::GracefulShutdown;
use structopt::StructOpt;
use tracing::{event, Level};

#[derive(Debug, StructOpt)]
pub struct Args {
    #[structopt(long, help = "Do not run the PostgreSQL queue stage drain tasks")]
    no_drain_queues: bool,
}

pub async fn main(args: Args) -> Result<(), crate::error::Error> {
    let shutdown = GracefulShutdown::new();
    let config = crate::server::Config {
        bind_address: Some(envoption::with_default("BIND_ADDRESS", "127.0.0.1")?),
        bind_port: envoption::with_default("BIND_PORT", 6543 as u16)?,
        database: database_configuration_from_env()?,
        redis_url: None,
        redis_queue_prefix: None,
        no_drain_queues: args.no_drain_queues,
        shutdown: shutdown.consumer(),
    };

    crate::tracing_config::configure("ergo", std::io::stdout);

    let server = crate::server::start(config).await?;
    event!(
        Level::INFO,
        "Listening on {}:{}",
        server.bind_address,
        server.bind_port
    );

    server.server.await?;

    shutdown.shutdown().await?;
    Ok(())
}
