use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
}

/// Decode JWT claims without validation
///
/// Since secure-frontend receives tokens directly from auth-service via signed requests,
/// we trust the token and just need to extract user_id for session storage and
/// propagation to other services.
///
/// Note: This does NOT validate the signature. The token is trusted because it comes
/// from auth-service via HMAC-signed request.
pub fn decode_jwt_claims(token: &str) -> Result<JwtClaims> {
    let parts: Vec<&str> = token.split('.').collect();

    if parts.len() != 3 {
        return Err(anyhow::anyhow!("Invalid JWT format"));
    }

    // Decode the payload (second part)
    let payload = general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|e| anyhow::anyhow!("Failed to decode JWT payload: {}", e))?;

    let claims: JwtClaims = serde_json::from_slice(&payload)
        .map_err(|e| anyhow::anyhow!("Failed to parse JWT claims: {}", e))?;

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_jwt_claims() {
        // Example JWT: header.payload.signature
        // Payload: {"sub":"user_123","email":"test@example.com","exp":9999999999,"iat":1736500000,"jti":"abc123"}
        let token = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ1c2VyXzEyMyIsImVtYWlsIjoidGVzdEBleGFtcGxlLmNvbSIsImV4cCI6OTk5OTk5OTk5OSwiaWF0IjoxNzM2NTAwMDAwLCJqdGkiOiJhYmMxMjMifQ.signature";

        let claims = decode_jwt_claims(token).unwrap();
        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.jti, "abc123");
    }
}
