# Notification Service

Multi-channel notification service supporting email, SMS, and push notifications.

## Architecture

**gRPC-only** internal service. External clients access via BFF.

- **gRPC**: Port 50053 (all business logic)
- **HTTP**: Port 8081 (health checks only)

## gRPC Service

### NotificationService

```protobuf
// Send notifications
rpc SendEmail(SendEmailRequest) returns (SendEmailResponse)
rpc SendSms(SendSmsRequest) returns (SendSmsResponse)
rpc SendPush(SendPushRequest) returns (SendPushResponse)
rpc SendBatch(SendBatchRequest) returns (SendBatchResponse)

// Query notifications
rpc GetNotification(GetNotificationRequest) returns (GetNotificationResponse)
rpc ListNotifications(ListNotificationsRequest) returns (ListNotificationsResponse)
```

## Channels

| Channel | Provider | Config |
|---------|----------|--------|
| Email | SMTP/Gmail | `SMTP_*` env vars |
| SMS | Msg91 | `MSG91_*` env vars |
| Push | FCM | `FCM_*` env vars |

## Usage (grpcurl)

```bash
# List services
grpcurl -plaintext localhost:50053 list

# Send email
grpcurl -plaintext -d '{
  "to": "user@example.com",
  "subject": "Welcome",
  "body_html": "<h1>Hello!</h1>",
  "body_text": "Hello!",
  "metadata": {"user_id": "123", "tenant_id": "456"}
}' localhost:50053 micros.notification.v1.NotificationService/SendEmail

# Send SMS
grpcurl -plaintext -d '{
  "to": "+14155551234",
  "body": "Your code is 123456",
  "metadata": {"purpose": "otp"}
}' localhost:50053 micros.notification.v1.NotificationService/SendSms

# Get notification status
grpcurl -plaintext -d '{
  "notification_id": "notif-abc"
}' localhost:50053 micros.notification.v1.NotificationService/GetNotification

# List notifications
grpcurl -plaintext -d '{
  "channel": "NOTIFICATION_CHANNEL_EMAIL",
  "status": "NOTIFICATION_STATUS_DELIVERED",
  "page_size": 20
}' localhost:50053 micros.notification.v1.NotificationService/ListNotifications
```

## Configuration

| Variable | Description |
|----------|-------------|
| `MONGODB_URI` | MongoDB connection |
| `SMTP_HOST` | SMTP server |
| `SMTP_PORT` | SMTP port |
| `SMTP_USER` | SMTP username |
| `SMTP_PASSWORD` | SMTP password |
| `MSG91_AUTH_KEY` | Msg91 API key |
| `MSG91_SENDER_ID` | Msg91 sender ID |
| `FCM_PROJECT_ID` | Firebase project ID |
| `FCM_SERVICE_ACCOUNT_KEY` | Firebase service account |
| `GRPC_PORT` | gRPC port (default: 50053) |
| `HTTP_PORT` | Health check port (default: 8081) |

## Health Checks

```bash
# HTTP
curl http://localhost:8081/health

# gRPC
grpcurl -plaintext localhost:50053 grpc.health.v1.Health/Check
```

## Proto Definitions

See `proto/micros/notification/v1/`:
- `notification.proto` - Main service
- `email.proto` - Email messages
- `sms.proto` - SMS messages
- `push.proto` - Push messages
