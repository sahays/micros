mod common;

use common::TestApp;
use reqwest::Client;
use service_core::grpc::{NotificationChannelProto, PushPlatformProto};
use std::collections::HashMap;

// =============================================================================
// Health Check (HTTP)
// =============================================================================

#[tokio::test]
async fn health_check_works() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/health", app.http_address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Failed to parse response");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["service"], "notification-service");
}

#[tokio::test]
async fn readiness_check_works() {
    let app = TestApp::spawn().await;
    let client = Client::new();

    let response = client
        .get(&format!("{}/ready", app.http_address))
        .send()
        .await
        .expect("Failed to execute request");

    assert!(response.status().is_success());
}

// =============================================================================
// Email (gRPC)
// =============================================================================

#[tokio::test]
async fn send_email_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let response = client
        .send_email_simple(
            "test@example.com".to_string(),
            "Test Email".to_string(),
            "Hello, this is a test email.".to_string(),
        )
        .await
        .expect("Failed to send email");

    assert!(!response.notification_id.is_empty());
    assert_eq!(response.channel, "email");
}

#[tokio::test]
async fn send_email_with_metadata() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let mut metadata = HashMap::new();
    metadata.insert("user_id".to_string(), "test-user-123".to_string());
    metadata.insert("tenant_id".to_string(), "test-tenant-456".to_string());

    let response = client
        .send_email(
            "test@example.com".to_string(),
            "Test with metadata".to_string(),
            Some("Plain text body".to_string()),
            Some("<h1>HTML body</h1>".to_string()),
            Some("Custom Sender".to_string()),
            Some("reply@example.com".to_string()),
            metadata,
        )
        .await
        .expect("Failed to send email");

    assert!(!response.notification_id.is_empty());
}

// =============================================================================
// SMS (gRPC)
// =============================================================================

#[tokio::test]
async fn send_sms_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let response = client
        .send_sms_simple(
            "+14155551234".to_string(),
            "Your verification code is 123456".to_string(),
        )
        .await
        .expect("Failed to send SMS");

    assert!(!response.notification_id.is_empty());
    assert_eq!(response.channel, "sms");
}

#[tokio::test]
async fn send_sms_with_metadata() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let mut metadata = HashMap::new();
    metadata.insert("purpose".to_string(), "otp".to_string());

    let response = client
        .send_sms("+14155551234".to_string(), "Test SMS".to_string(), metadata)
        .await
        .expect("Failed to send SMS");

    assert!(!response.notification_id.is_empty());
}

// =============================================================================
// Push (gRPC)
// =============================================================================

#[tokio::test]
async fn send_push_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let response = client
        .send_push_simple(
            "test-device-token-12345".to_string(),
            PushPlatformProto::Fcm,
            "Test Notification".to_string(),
            "This is a test push notification".to_string(),
        )
        .await
        .expect("Failed to send push");

    assert!(!response.notification_id.is_empty());
    assert_eq!(response.channel, "push");
}

#[tokio::test]
async fn send_push_with_data() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let mut data = HashMap::new();
    data.insert("action".to_string(), "open_chat".to_string());
    data.insert("chat_id".to_string(), "chat-123".to_string());

    let mut metadata = HashMap::new();
    metadata.insert("user_id".to_string(), "user-456".to_string());

    let response = client
        .send_push(
            "test-device-token".to_string(),
            PushPlatformProto::Fcm,
            "New Message".to_string(),
            "You have a new message".to_string(),
            data,
            metadata,
        )
        .await
        .expect("Failed to send push");

    assert!(!response.notification_id.is_empty());
}

// =============================================================================
// Batch (gRPC)
// =============================================================================

#[tokio::test]
async fn send_batch_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    use service_core::grpc::proto::notification::{
        BatchNotification, NotificationChannel, SendBatchRequest, SendEmailRequest, SendSmsRequest,
    };

    let batch_request = SendBatchRequest {
        notifications: vec![
            BatchNotification {
                channel: NotificationChannel::Email as i32,
                email: Some(SendEmailRequest {
                    to: "test1@example.com".to_string(),
                    subject: "Test 1".to_string(),
                    body_text: Some("Hello 1".to_string()),
                    body_html: None,
                    from_name: None,
                    reply_to: None,
                    metadata: HashMap::new(),
                }),
                sms: None,
                push: None,
            },
            BatchNotification {
                channel: NotificationChannel::Sms as i32,
                email: None,
                sms: Some(SendSmsRequest {
                    to: "+14155551234".to_string(),
                    body: "Test SMS".to_string(),
                    metadata: HashMap::new(),
                }),
                push: None,
            },
        ],
    };

    let response = client
        .send_batch(batch_request)
        .await
        .expect("Failed to send batch");

    assert!(!response.batch_id.is_empty());
    assert_eq!(response.results.len(), 2);
}

// =============================================================================
// Status & Query (gRPC)
// =============================================================================

#[tokio::test]
async fn get_notification_status_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // First send an email
    let send_response = client
        .send_email_simple(
            "test@example.com".to_string(),
            "Test Email".to_string(),
            "Hello".to_string(),
        )
        .await
        .expect("Failed to send email");

    let notification_id = send_response.notification_id;

    // Now get the status
    let status_response = client
        .get_notification(notification_id.clone())
        .await
        .expect("Failed to get notification status");

    let notification = status_response.notification.expect("Missing notification");
    assert_eq!(notification.notification_id, notification_id);
    assert_eq!(notification.channel, NotificationChannelProto::Email as i32);
}

#[tokio::test]
async fn list_notifications_via_grpc() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Send a couple of notifications
    client
        .send_email_simple(
            "test1@example.com".to_string(),
            "Test 1".to_string(),
            "Hello 1".to_string(),
        )
        .await
        .expect("Failed to send email");

    client
        .send_sms_simple("+14155551234".to_string(), "Test SMS".to_string())
        .await
        .expect("Failed to send SMS");

    // List all notifications
    let list_response = client
        .list_notifications(None, None, None, None, None, 10, None)
        .await
        .expect("Failed to list notifications");

    assert!(list_response.notifications.len() >= 2);
}

#[tokio::test]
async fn list_notifications_with_channel_filter() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    // Send email and SMS
    client
        .send_email_simple(
            "filter-test@example.com".to_string(),
            "Test".to_string(),
            "Hello".to_string(),
        )
        .await
        .expect("Failed to send email");

    client
        .send_sms_simple("+14155559999".to_string(), "Test SMS".to_string())
        .await
        .expect("Failed to send SMS");

    // List only email notifications
    let list_response = client
        .list_notifications(
            Some(NotificationChannelProto::Email),
            None,
            None,
            None,
            None,
            10,
            None,
        )
        .await
        .expect("Failed to list notifications");

    // All returned notifications should be email
    for notification in &list_response.notifications {
        assert_eq!(notification.channel, NotificationChannelProto::Email as i32);
    }
}

#[tokio::test]
async fn get_nonexistent_notification_returns_not_found() {
    let app = TestApp::spawn().await;
    let mut client = app.grpc_client().await;

    let result = client.get_notification("nonexistent-id".to_string()).await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status.code(), tonic::Code::NotFound);
}
