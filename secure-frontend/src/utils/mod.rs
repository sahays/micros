use tracing::subscriber::set_global_default;
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};

pub mod crypto;
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = fmt::layer();
    let subscriber = Registry::default().with(env_filter).with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set subscriber");
}
