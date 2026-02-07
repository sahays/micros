use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use prometheus::{IntCounterVec, Opts, Registry};
use std::sync::OnceLock;

pub static METRICS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();
pub static PROMETHEUS_REGISTRY: OnceLock<Registry> = OnceLock::new();
pub static PAYMENT_TRANSACTIONS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_AMOUNT_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_LINKED_ACCOUNTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_TRANSFERS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_TRANSFER_AMOUNT_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_COMMISSION_AMOUNT_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_SETTLEMENTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_SUBSCRIPTIONS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_SUBSCRIPTION_CHARGES_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_REFUNDS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_REFUND_AMOUNT_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_LINKS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();
pub static PAYMENT_WEBHOOK_EVENTS_TOTAL: OnceLock<IntCounterVec> = OnceLock::new();

fn register_counter(registry: &Registry, name: &str, help: &str, labels: &[&str]) -> IntCounterVec {
    let counter = IntCounterVec::new(Opts::new(name, help), labels)
        .unwrap_or_else(|_| panic!("Failed to create {} metric", name));
    registry
        .register(Box::new(counter.clone()))
        .unwrap_or_else(|_| panic!("Failed to register {}", name));
    counter
}

pub fn init_metrics() {
    let builder = PrometheusBuilder::new();
    let handle = builder
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    if METRICS_HANDLE.set(handle).is_err() {
        panic!("failed to set metrics handle: already initialized");
    }

    let registry = Registry::new();

    let transactions_counter = register_counter(
        &registry,
        "payment_transactions_total",
        "Total payment transactions by tenant and status",
        &["tenant_id", "status"],
    );

    let amount_counter = register_counter(
        &registry,
        "payment_amount_total",
        "Total payment amounts by tenant and currency (in smallest unit)",
        &["tenant_id", "currency"],
    );

    let linked_accounts_counter = register_counter(
        &registry,
        "payment_linked_accounts_total",
        "Total linked accounts by tenant and status",
        &["tenant_id", "status"],
    );

    let transfers_counter = register_counter(
        &registry,
        "payment_transfers_total",
        "Total transfers by tenant and status",
        &["tenant_id", "status"],
    );

    let transfer_amount_counter = register_counter(
        &registry,
        "payment_transfer_amount_total",
        "Total transfer amounts by tenant and currency",
        &["tenant_id", "currency"],
    );

    let commission_amount_counter = register_counter(
        &registry,
        "payment_commission_amount_total",
        "Total commission amounts by tenant and currency",
        &["tenant_id", "currency"],
    );

    let settlements_counter = register_counter(
        &registry,
        "payment_settlements_total",
        "Total settlements by tenant, type and status",
        &["tenant_id", "type", "status"],
    );

    let subscriptions_counter = register_counter(
        &registry,
        "payment_subscriptions_total",
        "Total subscriptions by tenant and status",
        &["tenant_id", "status"],
    );

    let subscription_charges_counter = register_counter(
        &registry,
        "payment_subscription_charges_total",
        "Total subscription charges by tenant and status",
        &["tenant_id", "status"],
    );

    let refunds_counter = register_counter(
        &registry,
        "payment_refunds_total",
        "Total refunds by tenant, status and speed",
        &["tenant_id", "status", "speed"],
    );

    let refund_amount_counter = register_counter(
        &registry,
        "payment_refund_amount_total",
        "Total refund amounts by tenant and currency",
        &["tenant_id", "currency"],
    );

    let links_counter = register_counter(
        &registry,
        "payment_links_total",
        "Total payment links by tenant and status",
        &["tenant_id", "status"],
    );

    let webhook_events_counter = register_counter(
        &registry,
        "payment_webhook_events_total",
        "Total webhook events by event type and domain",
        &["event_type", "domain"],
    );

    PROMETHEUS_REGISTRY
        .set(registry)
        .expect("Failed to set prometheus registry");
    PAYMENT_TRANSACTIONS_TOTAL
        .set(transactions_counter)
        .expect("Failed to set payment_transactions_total");
    PAYMENT_AMOUNT_TOTAL
        .set(amount_counter)
        .expect("Failed to set payment_amount_total");
    PAYMENT_LINKED_ACCOUNTS_TOTAL
        .set(linked_accounts_counter)
        .expect("Failed to set payment_linked_accounts_total");
    PAYMENT_TRANSFERS_TOTAL
        .set(transfers_counter)
        .expect("Failed to set payment_transfers_total");
    PAYMENT_TRANSFER_AMOUNT_TOTAL
        .set(transfer_amount_counter)
        .expect("Failed to set payment_transfer_amount_total");
    PAYMENT_COMMISSION_AMOUNT_TOTAL
        .set(commission_amount_counter)
        .expect("Failed to set payment_commission_amount_total");
    PAYMENT_SETTLEMENTS_TOTAL
        .set(settlements_counter)
        .expect("Failed to set payment_settlements_total");
    PAYMENT_SUBSCRIPTIONS_TOTAL
        .set(subscriptions_counter)
        .expect("Failed to set payment_subscriptions_total");
    PAYMENT_SUBSCRIPTION_CHARGES_TOTAL
        .set(subscription_charges_counter)
        .expect("Failed to set payment_subscription_charges_total");
    PAYMENT_REFUNDS_TOTAL
        .set(refunds_counter)
        .expect("Failed to set payment_refunds_total");
    PAYMENT_REFUND_AMOUNT_TOTAL
        .set(refund_amount_counter)
        .expect("Failed to set payment_refund_amount_total");
    PAYMENT_LINKS_TOTAL
        .set(links_counter)
        .expect("Failed to set payment_links_total");
    PAYMENT_WEBHOOK_EVENTS_TOTAL
        .set(webhook_events_counter)
        .expect("Failed to set payment_webhook_events_total");
}

pub fn get_metrics() -> String {
    let mut output = METRICS_HANDLE
        .get()
        .map(|handle| handle.render())
        .unwrap_or_else(|| "# Metrics recorder not initialized\n".to_string());

    // Append custom prometheus metrics
    if let Some(registry) = PROMETHEUS_REGISTRY.get() {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).ok();
        if let Ok(custom_metrics) = String::from_utf8(buffer) {
            output.push_str(&custom_metrics);
        }
    }

    output
}

/// Record a payment transaction for billing/metering.
pub fn record_transaction(tenant_id: &str, status: &str) {
    if let Some(counter) = PAYMENT_TRANSACTIONS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status]).inc();
    }
}

/// Record payment amount for billing/metering.
pub fn record_amount(tenant_id: &str, currency: &str, amount_cents: u64) {
    if let Some(counter) = PAYMENT_AMOUNT_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, currency])
            .inc_by(amount_cents);
    }
}

pub fn record_linked_account(tenant_id: &str, status: &str) {
    if let Some(counter) = PAYMENT_LINKED_ACCOUNTS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status]).inc();
    }
}

pub fn record_transfer(tenant_id: &str, status: &str) {
    if let Some(counter) = PAYMENT_TRANSFERS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status]).inc();
    }
}

pub fn record_transfer_amount(tenant_id: &str, currency: &str, amount: u64) {
    if let Some(counter) = PAYMENT_TRANSFER_AMOUNT_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, currency])
            .inc_by(amount);
    }
}

pub fn record_commission_amount(tenant_id: &str, currency: &str, amount: u64) {
    if let Some(counter) = PAYMENT_COMMISSION_AMOUNT_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, currency])
            .inc_by(amount);
    }
}

pub fn record_settlement(tenant_id: &str, settlement_type: &str, status: &str) {
    if let Some(counter) = PAYMENT_SETTLEMENTS_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, settlement_type, status])
            .inc();
    }
}

pub fn record_subscription(tenant_id: &str, status: &str) {
    if let Some(counter) = PAYMENT_SUBSCRIPTIONS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status]).inc();
    }
}

pub fn record_subscription_charge(tenant_id: &str, status: &str) {
    if let Some(counter) = PAYMENT_SUBSCRIPTION_CHARGES_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status]).inc();
    }
}

pub fn record_refund(tenant_id: &str, status: &str, speed: &str) {
    if let Some(counter) = PAYMENT_REFUNDS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status, speed]).inc();
    }
}

pub fn record_refund_amount(tenant_id: &str, currency: &str, amount: u64) {
    if let Some(counter) = PAYMENT_REFUND_AMOUNT_TOTAL.get() {
        counter
            .with_label_values(&[tenant_id, currency])
            .inc_by(amount);
    }
}

pub fn record_payment_link(tenant_id: &str, status: &str) {
    if let Some(counter) = PAYMENT_LINKS_TOTAL.get() {
        counter.with_label_values(&[tenant_id, status]).inc();
    }
}

pub fn record_webhook_event(event_type: &str, domain: &str) {
    if let Some(counter) = PAYMENT_WEBHOOK_EVENTS_TOTAL.get() {
        counter.with_label_values(&[event_type, domain]).inc();
    }
}
