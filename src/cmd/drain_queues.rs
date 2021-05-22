use crate::{
    error::Result,
    graceful_shutdown::GracefulShutdown,
    tasks::{actions::queue::ActionQueue, inputs::queue::InputQueue},
};

pub async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    dotenv::from_filename("vault_dev_roles.env").ok();

    crate::tracing_config::configure("drain-queues");

    let shutdown = GracefulShutdown::new();

    let vault_client = crate::vault::from_env("AIO_SERVER", &shutdown).await;
    tracing::info!("{:?}", vault_client);

    let backend_pg_pool =
        crate::service_config::backend_pg_pool(shutdown.consumer(), &vault_client).await?;
    let redis_pool = crate::service_config::redis_pool()?;

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
