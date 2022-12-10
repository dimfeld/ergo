use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

pub fn configure<W>(name: impl Into<String>, sink: W)
where
    for<'writer> W: MakeWriter<'writer> + Send + Sync + 'static,
{
    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    // let formatting_layer = BunyanFormattingLayer::new(name.into(), sink);
    let formatting_layer = HierarchicalLayer::new(2)
        .with_bracketed_fields(true)
        .with_targets(true)
        .with_writer(sink);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Setting subscriber");
}
