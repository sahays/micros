use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, runtime, trace as sdktrace};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing(service_name: &str, log_level: &str, otlp_endpoint: &str) {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level));

    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(otlp_endpoint);

    let tracer =
        match opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(otlp_exporter)
            .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", service_name.to_string()),
            ])))
            .install_batch(runtime::Tokio)
        {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Failed to initialize OTLP tracer for service '{}' at endpoint '{}': {}",
                    service_name, otlp_endpoint, e
                );
                panic!("Failed to initialize OTLP tracer: {}", e);
            }
        };

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(telemetry)
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .json()
                .flatten_event(true),
        )
        .init();
}
