# Audit Logging Guide

The Auth Service maintains a comprehensive audit log of security-critical events. These logs are essential for compliance, security monitoring, and forensic analysis.

## 1. Logged Events

The following events are automatically recorded:

| Event Type | Trigger Endpoint | Description |
|------------|------------------|-------------|
| `user_registration` | `POST /auth/register` | User signed up. |
| `user_login` | `POST /auth/login` | Successful user authentication. |
| `user_logout` | `POST /auth/logout` | User session terminated. |
| `password_reset_request` | `POST /auth/password-reset/request` | Reset email requested. |
| `password_reset_confirm` | `POST /auth/password-reset/confirm` | Password successfully changed. |
| `service_auth` | *Various* | Service-to-service calls (recorded implicitly via access logs/tracing). |

## 2. Audit Log Schema

Each audit entry contains:

```json
{
  "_id": "659d4...",
  "timestamp": "2024-01-05T10:00:00Z",
  "event_type": "user_login",
  "service_id": "user-123",        // ID of the user or service actor
  "service_name": null,            // Name of service (if applicable)
  "endpoint": "/auth/login",
  "method": "POST",
  "status_code": 200,
  "ip_address": "192.168.1.5",
  "details": null,
  "scopes": null
}
```

## 3. Accessing Logs

Audit logs are stored in the `audit_logs` MongoDB collection.

### Admin API

Administrators can fetch logs for specific service accounts via the Admin API.

`GET /auth/admin/services/{service_id}/audit-log`

**Headers:**
`X-Admin-Api-Key: <ADMIN_KEY>`

**Response:**
```json
[
  {
    "timestamp": "...",
    "event_type": "...",
    ...
  }
]
```

## 4. Integration Scenarios

### Scenario: Security Monitoring Dashboard (PLG)

To build a dashboard showing recent suspicious activities:

1.  **Ingest:** Promtail automatically ships the application's JSON logs to Loki (see [Observability](./observability.md)).
2.  **Filter:** In Grafana, use LogQL to filter for `event_type` fields.
    ```
    {container="auth-service"} | json | event_type="user_login"
    ```
3.  **Alert:** Set up Grafana Alerts based on Loki queries (e.g., high-frequency `user_login` failures from the same IP or `password_reset_confirm`).

### Scenario: Compliance Reporting

For GDPR/SOC2 compliance:
1.  Ensure retention policies on the MongoDB `audit_logs` collection (or backup archives) meet your legal requirements (e.g., 90 days).
2.  Use the Admin API to export logs for specific users upon Data Subject Access Requests (DSAR).
