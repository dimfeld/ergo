use std::{future::Future, time::Duration};

use once_cell::sync::Lazy;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

fn configure_tracing(name: impl Into<String>, sink: impl MakeWriter + Send + Sync + 'static) {
    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let formatting_layer = BunyanFormattingLayer::new(name.into(), sink);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Setting subscriber");
}

pub static TRACING: Lazy<()> = Lazy::new(|| {
    if std::env::var("TEST_LOG").is_ok() {
        configure_tracing("test", std::io::stdout);
    } else {
        configure_tracing("test", std::io::sink);
    }
});

/// Wait for a function to return a non-None value. If it tries more than 30 times
/// it will return an Err. The Err currently always indicates a time out.
pub async fn wait_for<Fut, DATA>(f: impl Fn() -> Fut) -> Result<DATA, ()>
where
    Fut: Future<Output = Option<DATA>>,
    DATA: Send + Sync,
{
    let mut tries = 0;
    while tries < 30 {
        match f().await {
            Some(d) => return Ok(d),
            None => {
                tokio::time::sleep(Duration::from_millis(250)).await;
                tries += 1;
            }
        }
    }

    Err(())
}
