use std::time::Duration;

use opentelemetry::trace::{TraceError, TraceState};

use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::resource::{
    EnvResourceDetector, SdkProvidedResourceDetector, TelemetryResourceDetector,
};
use opentelemetry_sdk::trace::{Sampler, ShouldSample};
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

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_timeout(Duration::from_secs(5)),
        )
        .with_batch_config(
            opentelemetry_sdk::trace::BatchConfigBuilder::default()
                .with_max_queue_size(30 * 1024)
                .with_max_export_batch_size(10 * 1024)
                .with_max_concurrent_exports(2)
                .with_scheduled_delay(Duration::from_millis(2000))
                .build(),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::config()
                .with_resource(resource)
                .with_sampler(SpanKindSampler {
                    parent: Box::new(Sampler::ParentBased(Box::new(Sampler::AlwaysOn))),
                    accepted_span_kinds: vec![
                        opentelemetry::trace::SpanKind::Client,
                        opentelemetry::trace::SpanKind::Server,
                    ],
                }),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)
}

#[derive(Debug, Clone)]
struct SpanKindSampler<S: std::fmt::Debug + Clone> {
    parent: Box<S>,
    accepted_span_kinds: Vec<opentelemetry::trace::SpanKind>,
}

impl<S> ShouldSample for SpanKindSampler<S>
where
    S: ShouldSample + std::fmt::Debug + Clone + 'static,
{
    fn should_sample(
        &self,
        parent_context: Option<&opentelemetry::Context>,
        trace_id: opentelemetry::trace::TraceId,
        name: &str,
        span_kind: &opentelemetry::trace::SpanKind,
        attributes: &[opentelemetry::KeyValue],
        links: &[opentelemetry::trace::Link],
    ) -> opentelemetry::trace::SamplingResult {
        if self.accepted_span_kinds.contains(span_kind) {
            self.parent
                .should_sample(parent_context, trace_id, name, span_kind, attributes, links)
        } else {
            opentelemetry::trace::SamplingResult {
                decision: opentelemetry::trace::SamplingDecision::Drop,
                attributes: Vec::default(),
                trace_state: TraceState::NONE,
            }
        }
    }
}
