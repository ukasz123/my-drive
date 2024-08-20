use tracing::level_filters::LevelFilter;
use tracing::{Level, Subscriber};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

mod log;
mod otel;

pub(crate) fn create_subscriber() -> Box<dyn Subscriber + Send + Sync> {
    let log_layer = log::log_layer();

    let opentelemetry_layer = otel::init_opentelemetry_tracer()
        .map(|tracer| tracing_opentelemetry::layer().with_tracer(tracer))
        .map(|otel_layer| otel_layer.with_filter(LevelFilter::from_level(Level::DEBUG)));

    let subscriber = tracing_subscriber::Registry::default();

    let subscriber = subscriber.with(log_layer);

    if let Ok(opentelemetry_layer) = opentelemetry_layer {
        Box::new(subscriber.with(opentelemetry_layer))
    } else {
        Box::new(subscriber)
    }
}

pub(crate) fn init_telemetry(subscriber: impl Subscriber + Send + Sync) {
    opentelemetry::global::set_error_handler(|error| {
        ::tracing::error!(
            target: "opentelemetry",
            "OpenTelemetry error occurred: {:#}",
            anyhow::anyhow!(error),
        );
    })
    .expect("to be able to set error handler");
    tracing::subscriber::set_global_default(subscriber)
        .expect("Subscriber failed to be set as global");
}
