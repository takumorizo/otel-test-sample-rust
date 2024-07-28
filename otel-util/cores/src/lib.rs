use deps::opentelemetry;
use deps::opentelemetry::{global, propagation::Extractor, propagation::Injector};
use deps::serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 参考記事に倣って、実装。
// https://peterpaul.hashnode.dev/opentelemetry-context-propagation-with-rust
// Serializable datastructure to hold the opentelemetry propagation context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropagationContext(pub HashMap<String, String>);

impl PropagationContext {
    fn empty() -> Self {
        Self(HashMap::new())
    }

    pub fn inject(context: &opentelemetry::Context) -> Self {
        global::get_text_map_propagator(|propagator| {
            let mut propagation_context = PropagationContext::empty();
            propagator.inject_context(context, &mut propagation_context);
            propagation_context
        })
    }

    pub fn extract(&self) -> opentelemetry::Context {
        global::get_text_map_propagator(|propagator| propagator.extract(self))
    }
}

impl Injector for PropagationContext {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_owned(), value);
    }
}

impl Extractor for PropagationContext {
    fn get(&self, key: &str) -> Option<&str> {
        let key = key.to_owned();
        self.0.get(&key).map(|v| v.as_ref())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_ref()).collect()
    }
}

use deps::opentelemetry::KeyValue;
use deps::opentelemetry_otlp;
use deps::opentelemetry_otlp::WithExportConfig;
use deps::opentelemetry_sdk;
use deps::opentelemetry_sdk::{
    runtime,
    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
    Resource,
};
use deps::opentelemetry_semantic_conventions::{
    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use deps::tokio::runtime::Handle;
use deps::tracing;
use deps::tracing_core::Level;
use deps::tracing_opentelemetry::OpenTelemetryLayer;
use deps::tracing_subscriber;
use deps::tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn resource(service_name: &str, version: &str) -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, service_name.to_string()),
            KeyValue::new(SERVICE_VERSION, version.to_string()),
            KeyValue::new(
                DEPLOYMENT_ENVIRONMENT,
                std::env::var("DEPLOYMENT_ENVIRONMENT").unwrap_or("unknown".to_string()),
            ),
        ],
        SCHEMA_URL,
    )
}

fn init_tracer(collector_endpoint: &str, service_name: &str, version: &str) -> Tracer {
    global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new());

    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                // Customize sampling strategy
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    1.0,
                ))))
                // If export trace to AWS X-Ray, you can use XrayIdGenerator
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource(service_name, version)),
        )
        .with_batch_config(BatchConfig::default())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(collector_endpoint),
        )
        .install_batch(runtime::Tokio)
        .unwrap()
}

pub fn init_telemetry(collector_endpoint: &str, service_name: &str, version: &str) -> OtelGuard {
    let tracer = init_tracer(collector_endpoint, service_name, version);

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer.clone()))
        .init();

    std::panic::set_hook(Box::new(|panic_info| {
        tracing::error!("panic occurred: {}", panic_info);
    }));

    OtelGuard { tracer }
}

pub struct OtelGuard {
    tracer: Tracer,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        Handle::current().spawn(async {
            opentelemetry::global::shutdown_tracer_provider();
        });
    }
}
