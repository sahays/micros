mod common;

use common::TestApp;
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn health_check_works() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/health", app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "notification-service");
}

#[tokio::test]
async fn send_email_returns_accepted() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/notifications/email", app.address))
        .json(&json!({
            "to": "test@example.com",
            "subject": "Test Email",
            "body_text": "Hello, this is a test email."
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 202);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(!body["notification_id"].as_str().unwrap().is_empty());
    assert_eq!(body["channel"], "email");
}

#[tokio::test]
async fn send_email_validates_email_address() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/notifications/email", app.address))
        .json(&json!({
            "to": "invalid-email",
            "subject": "Test Email",
            "body_text": "Hello"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 422);
}

#[tokio::test]
async fn send_email_requires_body() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/notifications/email", app.address))
        .json(&json!({
            "to": "test@example.com",
            "subject": "Test Email"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn send_sms_returns_accepted() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/notifications/sms", app.address))
        .json(&json!({
            "to": "+14155551234",
            "body": "Your verification code is 123456"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 202);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(!body["notification_id"].as_str().unwrap().is_empty());
    assert_eq!(body["channel"], "sms");
}

#[tokio::test]
async fn send_push_returns_accepted() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/notifications/push", app.address))
        .json(&json!({
            "device_token": "test-device-token-12345",
            "platform": "fcm",
            "title": "Test Notification",
            "body": "This is a test push notification"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 202);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(!body["notification_id"].as_str().unwrap().is_empty());
    assert_eq!(body["channel"], "push");
}

#[tokio::test]
async fn send_batch_returns_accepted() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .post(&format!("{}/notifications/batch", app.address))
        .json(&json!({
            "notifications": [
                {
                    "channel": "email",
                    "to": "test1@example.com",
                    "subject": "Test 1",
                    "body_text": "Hello 1"
                },
                {
                    "channel": "sms",
                    "to": "+14155551234",
                    "body": "Test SMS"
                }
            ]
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 202);

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert!(!body["batch_id"].as_str().unwrap().is_empty());
    assert_eq!(body["notifications"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn get_notification_status() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    // First send an email
    let send_response = client
        .post(&format!("{}/notifications/email", app.address))
        .json(&json!({
            "to": "test@example.com",
            "subject": "Test Email",
            "body_text": "Hello"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(send_response.status(), 202);

    let send_body: serde_json::Value = send_response
        .json()
        .await
        .expect("Failed to parse response");
    let notification_id = send_body["notification_id"].as_str().unwrap();

    // Now get the status
    let status_response = client
        .get(&format!(
            "{}/notifications/{}",
            app.address, notification_id
        ))
        .send()
        .await
        .expect("Failed to execute request");

    let status_code = status_response.status();
    let status_body: serde_json::Value = status_response
        .json()
        .await
        .unwrap_or_else(|_| json!({"error": "failed to parse"}));

    assert!(
        status_code.is_success(),
        "Expected success, got {} with body: {}",
        status_code,
        status_body
    );
    assert_eq!(status_body["notification_id"], notification_id);
    assert_eq!(status_body["channel"], "email");
}

#[tokio::test]
async fn list_notifications() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    // Send a couple of notifications
    client
        .post(&format!("{}/notifications/email", app.address))
        .json(&json!({
            "to": "test1@example.com",
            "subject": "Test 1",
            "body_text": "Hello 1"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    client
        .post(&format!("{}/notifications/sms", app.address))
        .json(&json!({
            "to": "+14155551234",
            "body": "Test SMS"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    // List all notifications
    let list_response = client
        .get(&format!("{}/notifications", app.address))
        .send()
        .await
        .expect("Failed to execute request");

    let status_code = list_response.status();
    let list_body: serde_json::Value = list_response
        .json()
        .await
        .unwrap_or_else(|_| json!({"error": "failed to parse"}));

    assert!(
        status_code.is_success(),
        "Expected success, got {} with body: {}",
        status_code,
        list_body
    );
    assert!(list_body["count"].as_i64().unwrap() >= 2);
}

#[tokio::test]
async fn list_notifications_with_channel_filter() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    // Send email and SMS
    client
        .post(&format!("{}/notifications/email", app.address))
        .json(&json!({
            "to": "test@example.com",
            "subject": "Test",
            "body_text": "Hello"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    client
        .post(&format!("{}/notifications/sms", app.address))
        .json(&json!({
            "to": "+14155551234",
            "body": "Test SMS"
        }))
        .send()
        .await
        .expect("Failed to execute request");

    // List only email notifications
    let list_response = client
        .get(&format!("{}/notifications?channel=email", app.address))
        .send()
        .await
        .expect("Failed to execute request");

    let status_code = list_response.status();
    let list_body: serde_json::Value = list_response
        .json()
        .await
        .unwrap_or_else(|_| json!({"error": "failed to parse"}));

    assert!(
        status_code.is_success(),
        "Expected success, got {} with body: {}",
        status_code,
        list_body
    );

    // All returned notifications should be email
    for notification in list_body["notifications"].as_array().unwrap() {
        assert_eq!(notification["channel"], "email");
    }
}

#[tokio::test]
async fn get_nonexistent_notification_returns_404() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/notifications/nonexistent-id", app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), 404);
}
