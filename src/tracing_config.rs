use std::sync::atomic::{AtomicBool, Ordering};

use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn configure(name: impl Into<String>) {
    if INITIALIZED.swap(true, Ordering::Acquire) {
        return;
    }

    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or(EnvFilter::new("info"));

    let formatting_layer = BunyanFormattingLayer::new(name.into(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Setting subscriber");
}
