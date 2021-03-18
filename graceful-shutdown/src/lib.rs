use tokio::select;
use tokio::signal::ctrl_c;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

pub struct GracefulShutdown {
    pub start_shutdown: oneshot::Sender<()>,
    pub shutdown_finished: JoinHandle<()>,

    consumer: GracefulShutdownConsumer,
}

#[derive(Clone, Debug)]
pub struct GracefulShutdownConsumer(watch::Receiver<bool>);

impl GracefulShutdown {
    pub fn new() -> GracefulShutdown {
        // This channel changes to true and drops when shutdown is started
        let (shutdown_started_tx, shutdown_started_rx) = watch::channel(false);

        // Send a value or close this channel to start shutting down.
        let (start_shutdown_tx, start_shutdown_rx) = oneshot::channel();

        let shutdown_waiter = tokio::spawn(async move {
            select! {
                _ = ctrl_c() => {},
                _ = start_shutdown_rx => {},
            };

            // Explicitly drop this so that we'll have ownership of it.
            // In the future there will be more to do here as we start waiting for other things to
            // actually shut down.
            shutdown_started_tx.send(true).unwrap();
        });

        GracefulShutdown {
            start_shutdown: start_shutdown_tx,
            shutdown_finished: shutdown_waiter,
            consumer: GracefulShutdownConsumer(shutdown_started_rx),
        }
    }

    pub fn consumer(&self) -> GracefulShutdownConsumer {
        self.consumer.clone()
    }
}

impl GracefulShutdownConsumer {
    pub async fn shutting_down(&mut self) -> bool {
        match self.0.changed().await {
            // Sender is still open, but value is true so we're shutting down.
            Ok(_) => *self.0.borrow() == true,
            // Sender closed, which means we're shutting down.
            Err(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn must_be_sync() {
        fn takes_a_sync<T: Sync>(_value: T) {}

        let gs = GracefulShutdown::new();
        takes_a_sync(gs.consumer());
    }
}
