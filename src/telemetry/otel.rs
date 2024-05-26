use std::time::Duration;

use opentelemetry::trace::TraceError;

use opentelemetry_sdk::resource::{
    EnvResourceDetector, SdkProvidedResourceDetector, TelemetryResourceDetector,
};
use opentelemetry_sdk::trace::{Span, SpanLimits};
use opentelemetry_sdk::Resource;

pub(crate) fn init_opentelemetry_tracer() -> Result<opentelemetry_sdk::trace::Tracer, TraceError> {
    let resource = Resource::from_detectors(
        Duration::from_secs(0),
        vec![
            Box::new(SdkProvidedResourceDetector),
            Box::new(EnvResourceDetector::new()),
            Box::new(TelemetryResourceDetector),
        ],
    );

    opentelemetry_otlp::new_pipeline().tracing().with_exporter(
        opentelemetry_otlp::new_exporter().tonic())
        .with_batch_config(
            opentelemetry_sdk::trace::BatchConfigBuilder::default()
                .with_max_queue_size(30000)
                .with_max_export_batch_size(10000)
                .with_scheduled_delay(Duration::from_millis(10000))
                .build(),
        )
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(resource))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
}
