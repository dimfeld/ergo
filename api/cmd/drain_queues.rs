use crate::{error::Result, service_config::database_configuration_from_env};
use ergo_graceful_shutdown::GracefulShutdown;

pub async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    crate::tracing_config::configure("drain-queues", std::io::stdout);

    let shutdown = GracefulShutdown::new();

    let database_config = database_configuration_from_env()?;
    let backend_pg_pool = crate::service_config::backend_pg_pool(&database_config).await?;
    let redis_pool = ergo_database::RedisPool::new(None, None)?;

    let _queue_drain = ergo_tasks::queue_drain_runner::AllQueuesDrain::new(
        backend_pg_pool.clone(),
        redis_pool.clone(),
        shutdown.consumer(),
    )?;

    shutdown.consumer().wait_for_shutdown().await;

    Ok(())
}
