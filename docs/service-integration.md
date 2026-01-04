# Service Integration Guide

This guide explains how to integrate your microservice with the `auth-service` using API keys for service-to-service authentication.

## Authentication Overview

Services authenticate using an API key provided in the `Authorization` header as a Bearer token.

**Header Format:**
`Authorization: Bearer <your_api_key>`

API keys are prefixed with `svc_live_` for production or `svc_test_` for development.

## Scopes

Endpoints are protected by scopes. Your service account must have the required scopes to access specific resources.

Common Scopes:
- `user:read`: Read user profiles
- `user:write`: Update user profiles
- `token:introspect`: Validate user JWT tokens
- `admin:*`: Full access to admin operations

## Integration Examples

### Rust (using `reqwest`)

```rust
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

async fn call_protected_api() -> Result<(), reqwest::Error> {
    let api_key = "svc_test_your_api_key_here";
    let client = reqwest::Client::new();
    
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
    );

    let response = client
        .get("https://auth-service/users/me")
        .headers(headers)
        .send()
        .await?;

    println!("Status: {}", response.status());
    Ok(())
}
```

### Node.js (using `axios`)

```javascript
const axios = require('axios');

async function callProtectedApi() {
  const apiKey = 'svc_test_your_api_key_here';
  
  try {
    const response = await axios.get('https://auth-service/users/me', {
      headers: {
        'Authorization': `Bearer ${apiKey}`
      }
    });
    console.log(response.data);
  } catch (error) {
    console.error('Error:', error.response.status);
  }
}
```

### Python (using `requests`)

```python
import requests

def call_protected_api():
    api_key = "svc_test_your_api_key_here"
    headers = {
        "Authorization": f"Bearer {api_key}"
    }
    
    response = requests.get("https://auth-service/users/me", headers=headers)
    
    if response.status_code == 200:
        print(response.json())
    else:
        print(f"Error: {response.status_code}")
```

### Go

```go
package main

import (
	"fmt"
	"net/http"
)

func callProtectedApi() {
	apiKey := "svc_test_your_api_key_here"
	client := &http.Client{}
	
	req, _ := http.NewRequest("GET", "https://auth-service/users/me", nil)
	req.Header.Set("Authorization", fmt.Sprintf("Bearer %s", apiKey))
	
	resp, err := client.Do(req)
	if err != nil {
		fmt.Println("Error:", err)
		return
	}
	defer resp.Body.Close()
	
	fmt.Println("Status:", resp.Status)
}
```

## Error Handling

| Status Code | Description |
|-------------|-------------|
| 200/201 | Success |
| 401 | Invalid or missing API key |
| 403 | Insufficient scopes or disabled account |
| 429 | Rate limit exceeded |
| 500 | Internal server error |

## Best Practices

1. **Keep Keys Secret**: Treat API keys like passwords. Never commit them to version control. Use environment variables or secret managers.
2. **Use Test Keys for Dev**: Always use `svc_test_` keys during development and testing.
3. **Handle Rotation**: Be prepared to update your API key. Rotation has a 7-day grace period where both old and new keys work.
4. **Least Privilege**: Only request the scopes your service absolutely needs.
