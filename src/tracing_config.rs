use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::time::ChronoUtc, layer::SubscriberExt, EnvFilter, Registry};

pub fn configure(name: impl Into<String>) {
    LogTracer::init().expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or(EnvFilter::new("info"));

    let subscriber = Registry::default().with(env_filter);

    if envoption::with_default("LOG_PRETTY", false).unwrap() {
        let formatter = tracing_subscriber::fmt::Layer::new().pretty();
        let subscriber = subscriber.with(formatter);

        set_global_default(subscriber).expect("Setting subscriber");
    } else {
        let formatter = tracing_subscriber::fmt::Layer::new()
            .json()
            .with_timer(ChronoUtc::rfc3339());
        let subscriber = subscriber.with(formatter);

        set_global_default(subscriber).expect("Setting subscriber");
    }
}
