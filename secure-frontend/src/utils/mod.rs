use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use tracing::subscriber::set_global_default;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};

pub mod crypto;
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let otlp_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://tempo:4317");

    let tracer =
        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(otlp_exporter)
            .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
                KeyValue::new("service.name", "secure-frontend"),
            ])))
            .install_batch(runtime::Tokio)
            .expect("Failed to initialize OTLP tracer");

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let formatting_layer = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .json()
        .flatten_event(true);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(telemetry)
        .with(formatting_layer);

    set_global_default(subscriber).expect("Failed to set subscriber");
}
