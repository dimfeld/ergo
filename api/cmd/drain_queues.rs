use crate::{
    error::Result,
    service_config::database_configuration_from_env,
    tasks::{actions::queue::ActionQueue, inputs::queue::InputQueue},
};
use ergo_graceful_shutdown::GracefulShutdown;

pub async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    dotenv::from_filename("vault_dev_roles.env").ok();

    crate::tracing_config::configure("drain-queues", std::io::stdout);

    let shutdown = GracefulShutdown::new();

    let vault_client = ergo_database::vault::from_env("AIO_SERVER", shutdown.consumer()).await;
    tracing::info!("{:?}", vault_client);

    let database_config = database_configuration_from_env()?;
    let backend_pg_pool =
        crate::service_config::backend_pg_pool(shutdown.consumer(), &vault_client, database_config)
            .await?;
    let redis_pool = ergo_database::RedisPool::new(None, None)?;

    let input_queue = InputQueue::new(redis_pool.clone());
    let action_queue = ActionQueue::new(redis_pool.clone());
    let _queue_drain = crate::tasks::queue_drain_runner::AllQueuesDrain::new(
        input_queue,
        action_queue,
        backend_pg_pool.clone(),
        redis_pool.clone(),
        shutdown.consumer(),
    )?;

    shutdown.consumer().wait_for_shutdown().await;

    Ok(())
}
