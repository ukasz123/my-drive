use tracing_subscriber::{EnvFilter, Layer};

pub(crate) fn log_layer<
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
>() -> impl tracing_subscriber::layer::Layer<S> + Send + Sync {
    tracing_subscriber::fmt::layer()
        .pretty()
        .with_filter(EnvFilter::from_default_env())
}
