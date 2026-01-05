# Service Integration Guide

This guide explains how to authenticate server-to-server communication with the `auth-service` using Service Accounts.

## 1. Provisioning Access

Service authentication requires an API Key. This must be provisioned by an administrator.

**Create Service Account (Admin Only):**
```bash
POST /auth/admin/services
X-Admin-Api-Key: <ADMIN_API_KEY_FROM_ENV>
Content-Type: application/json

{
  "service_name": "Billing Service",
  "scopes": ["user:read", "token:introspect"]
}
```

**Response:**
```json
{
  "service_id": "...",
  "api_key": "svc_live_...", 
  "scopes": ["user:read", "token:introspect"]
}
```

> **Important:** The `api_key` is shown **only once**. Store it securely (e.g., Vault, Env Vars).

## 2. Authentication

Include the API key in the `Authorization` header with the `Bearer` scheme.

**Header Format:**
`Authorization: Bearer <your_api_key>`

### Env Prefixes
- **Production**: Keys start with `svc_live_`
- **Development**: Keys start with `svc_test_`

## 3. Implementation (Pseudocode)

```python
function call_auth_service(endpoint):
    key = load_env("AUTH_SERVICE_API_KEY")
    
    headers = {
        "Authorization": "Bearer " + key,
        "Accept": "application/json"
    }
    
    response = http_get("https://auth-service" + endpoint, headers)
    
    if response.status == 401:
        log_error("Invalid API Key")
    elif response.status == 403:
        log_error("Insufficient Permissions/Scope")
        
    return response
```

## 4. Scopes & Permissions

Ensure your service account has the necessary scopes for the endpoints you call.

| Scope | Description |
|-------|-------------|
| `user:read` | Read public user profile data. |
| `user:write` | Modify user data. |
| `token:introspect` | Verify JWT validity. |
| `admin:*` | **High Privilege**: Full administrative access. |

## 5. Key Rotation

To rotate a compromised or old key without downtime:

1.  **Call Admin API**: `POST /auth/admin/services/{id}/rotate`
2.  **Update Config**: Deploy the new key to your service.
3.  **Grace Period**: The old key remains valid for **7 days** to allow propagation.