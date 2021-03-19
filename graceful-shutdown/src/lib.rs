use tokio::select;
use tokio::signal::ctrl_c;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct GracefulShutdown {
    pub shutdown_finished: JoinHandle<()>,

    start_shutdown: Option<oneshot::Sender<()>>,
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
            start_shutdown: Some(start_shutdown_tx),
            shutdown_finished: shutdown_waiter,
            consumer: GracefulShutdownConsumer(shutdown_started_rx),
        }
    }

    pub fn consumer(&self) -> GracefulShutdownConsumer {
        self.consumer.clone()
    }

    pub fn shutdown(&mut self) {
        if let Some(sender) = self.start_shutdown.take() {
            sender.send(()).unwrap();
        }
    }
}

impl GracefulShutdownConsumer {
    pub fn shutting_down(&mut self) -> bool {
        *self.0.borrow()
    }

    pub async fn wait_for_shutdown(&mut self) -> () {
        loop {
            match self.0.changed().await {
                Ok(_) => {
                    // Sender is still open, but value is true so we're shutting down.
                    if *self.0.borrow() == true {
                        return;
                    }
                }
                // Sender closed, which means we're shutting down.
                Err(_) => return,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use libc::{getpid, kill, SIGINT};
    use tokio::{sync::oneshot::error::TryRecvError, time::timeout};

    /// Send a SIGINT to the current process
    #[doc(hidden)]
    pub fn send_sigint() {
        unsafe {
            kill(getpid(), SIGINT);
        }
    }

    #[tokio::test]
    async fn consumer_must_be_send_and_sync() {
        fn takes_a_sync<T: Send + Sync>(_value: T) {}

        let gs = GracefulShutdown::new();
        takes_a_sync(gs.consumer());
    }

    #[tokio::test]
    async fn handle_sigint() {
        let s = GracefulShutdown::new();

        let mut done_consumer = s.consumer();
        assert_eq!(done_consumer.shutting_down(), false);
        let (done_tx, mut done_rx) = oneshot::channel::<()>();
        let done_task = tokio::spawn(async move {
            done_consumer.wait_for_shutdown().await;
            done_tx.send(()).unwrap();
        });

        // It shouldn't have triggered yet.
        assert_eq!(done_rx.try_recv(), Err(TryRecvError::Empty));

        let mut before_consumer = s.consumer();
        assert_eq!(before_consumer.shutting_down(), false);

        // Yield to make sure that the GracefulShutdown task gets a chance to start before we send
        // the SIGINT.
        tokio::task::yield_now().await;

        send_sigint();

        match timeout(Duration::from_secs(2), done_task).await {
            Ok(Ok(())) => {}
            x => panic!("Done waiter failed to stop: {:?}", x),
        };

        assert_eq!(before_consumer.shutting_down(), true);

        // Consumers created after the SIGINT should work too.
        let mut after_consumer = s.consumer();
        assert_eq!(after_consumer.shutting_down(), true);

        match timeout(Duration::from_secs(2), after_consumer.wait_for_shutdown()).await {
            Ok(()) => {}
            x => panic!(
                "Waiter started after SIGINT does not indicate SIGINT already happened: {:?}",
                x
            ),
        };

        match timeout(Duration::from_secs(2), s.shutdown_finished).await {
            Ok(Ok(())) => {}
            x => panic!(
                "GracefulShutdown tasks didn't quit after SIGINT: result {:?}",
                x
            ),
        };
    }

    #[tokio::test]
    async fn handle_manual_shutdown() {
        let mut s = GracefulShutdown::new();

        let mut done_consumer = s.consumer();
        assert_eq!(done_consumer.shutting_down(), false);
        let (done_tx, mut done_rx) = oneshot::channel::<()>();
        let done_task = tokio::spawn(async move {
            done_consumer.wait_for_shutdown().await;
            done_tx.send(()).unwrap();
        });

        // It shouldn't have triggered yet.
        assert_eq!(done_rx.try_recv(), Err(TryRecvError::Empty));

        let mut before_consumer = s.consumer();
        assert_eq!(before_consumer.shutting_down(), false);

        s.shutdown();

        match timeout(Duration::from_secs(2), done_task).await {
            Ok(Ok(())) => {}
            x => panic!("Done waiter failed to stop: {:?}", x),
        };

        assert_eq!(before_consumer.shutting_down(), true);

        // Consumers created after the shutdown has started should work too.
        let mut after_consumer = s.consumer();
        assert_eq!(after_consumer.shutting_down(), true);

        match timeout(Duration::from_secs(2), after_consumer.wait_for_shutdown()).await {
            Ok(()) => {}
            x => panic!(
                "Waiter started after SIGINT does not indicate SIGINT already happened: {:?}",
                x
            ),
        };

        match timeout(Duration::from_secs(2), s.shutdown_finished).await {
            Ok(Ok(())) => {}
            x => panic!(
                "GracefulShutdown tasks didn't quit after SIGINT: result {:?}",
                x
            ),
        };
    }
}
