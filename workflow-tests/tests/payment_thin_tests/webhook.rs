use hmac::{Hmac, Mac};
use sha2::Sha256;
use workflow_tests::helpers::payment::TestApp;

fn webhook_secret() -> String {
    std::env::var("RAZORPAY_WEBHOOK_SECRET").unwrap_or_else(|_| "test_webhook_secret".to_string())
}

/// Compute HMAC-SHA256 signature for webhook body.
fn compute_webhook_signature(body: &str, secret: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

fn make_payment_webhook_body(event: &str) -> String {
    serde_json::json!({
        "entity": "event",
        "account_id": "acc_test",
        "event": event,
        "contains": ["payment"],
        "payload": {
            "payment": {
                "entity": {
                    "id": "pay_test_123",
                    "entity": "payment",
                    "amount": 10000,
                    "currency": "INR",
                    "status": "captured",
                    "order_id": null,
                    "method": "upi",
                    "created_at": 1700000000,
                    "captured": true
                }
            }
        },
        "created_at": 1700000000
    })
    .to_string()
}

fn make_transfer_webhook_body(event: &str) -> String {
    serde_json::json!({
        "entity": "event",
        "account_id": "acc_test",
        "event": event,
        "contains": ["transfer"],
        "payload": {
            "transfer": {
                "entity": {
                    "id": "trf_test_456",
                    "entity": "transfer",
                    "amount": 5000,
                    "currency": "INR",
                    "status": "processed",
                    "source": "pay_test_123",
                    "recipient": "acc_test_456",
                    "amount_reversed": 0
                }
            }
        },
        "created_at": 1700000000
    })
    .to_string()
}

fn make_subscription_webhook_body(event: &str) -> String {
    serde_json::json!({
        "entity": "event",
        "account_id": "acc_test",
        "event": event,
        "contains": ["subscription"],
        "payload": {
            "subscription": {
                "entity": {
                    "id": "sub_test_789",
                    "entity": "subscription",
                    "plan_id": "plan_test_123",
                    "status": "active",
                    "total_count": 12,
                    "paid_count": 1,
                    "remaining_count": 11
                }
            }
        },
        "created_at": 1700000000
    })
    .to_string()
}

fn make_account_webhook_body(event: &str) -> String {
    serde_json::json!({
        "entity": "event",
        "account_id": "acc_test",
        "event": event,
        "contains": ["account"],
        "payload": {
            "account": {
                "entity": {
                    "id": "acc_test_activated",
                    "status": "activated",
                    "email": "vendor@example.com"
                }
            }
        },
        "created_at": 1700000000
    })
    .to_string()
}

#[tokio::test]
async fn webhook_payment_captured_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let body = make_payment_webhook_body("payment.captured");
    let secret = webhook_secret();
    let signature = compute_webhook_signature(&body, &secret);

    let response = client
        .handle_razorpay_webhook(&body, &signature)
        .await
        .expect("Webhook should succeed");

    assert!(response.success);
    assert_eq!(response.event_type, "payment.captured");
}

#[tokio::test]
async fn webhook_payment_failed_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let body = make_payment_webhook_body("payment.failed");
    let secret = webhook_secret();
    let signature = compute_webhook_signature(&body, &secret);

    let response = client
        .handle_razorpay_webhook(&body, &signature)
        .await
        .expect("Webhook should succeed");

    assert!(response.success);
    assert_eq!(response.event_type, "payment.failed");
}

#[tokio::test]
async fn webhook_transfer_processed_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let body = make_transfer_webhook_body("transfer.processed");
    let secret = webhook_secret();
    let signature = compute_webhook_signature(&body, &secret);

    let response = client
        .handle_razorpay_webhook(&body, &signature)
        .await
        .expect("Webhook should succeed");

    assert!(response.success);
    assert_eq!(response.event_type, "transfer.processed");
}

#[tokio::test]
async fn webhook_subscription_active_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let body = make_subscription_webhook_body("subscription.active");
    let secret = webhook_secret();
    let signature = compute_webhook_signature(&body, &secret);

    let response = client
        .handle_razorpay_webhook(&body, &signature)
        .await
        .expect("Webhook should succeed");

    assert!(response.success);
    assert_eq!(response.event_type, "subscription.active");
}

#[tokio::test]
async fn webhook_account_activated_succeeds() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let body = make_account_webhook_body("account.activated");
    let secret = webhook_secret();
    let signature = compute_webhook_signature(&body, &secret);

    let response = client
        .handle_razorpay_webhook(&body, &signature)
        .await
        .expect("Webhook should succeed");

    assert!(response.success);
    assert_eq!(response.event_type, "account.activated");
}

#[tokio::test]
async fn webhook_unknown_event_acknowledged() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let body = serde_json::json!({
        "entity": "event",
        "account_id": "acc_test",
        "event": "some.unknown.event",
        "contains": [],
        "payload": {},
        "created_at": 1700000000
    })
    .to_string();

    let secret = webhook_secret();
    let signature = compute_webhook_signature(&body, &secret);

    let response = client
        .handle_razorpay_webhook(&body, &signature)
        .await
        .expect("Unknown webhook event should still succeed");

    assert!(response.success);
    assert_eq!(response.event_type, "some.unknown.event");
}

#[tokio::test]
async fn webhook_invalid_signature_fails() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let body = make_payment_webhook_body("payment.captured");

    let result = client
        .handle_razorpay_webhook(&body, "invalid_signature")
        .await;

    assert!(result.is_err());
}
