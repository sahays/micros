use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use uuid::Uuid;

use crate::config::JwtConfig;

/// JWT service for token generation and validation
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    public_key_pem: String,
    key_id: String,
    access_token_expiry_minutes: i64,
    refresh_token_expiry_days: i64,
    app_token_expiry_minutes: i64,
}

/// JWK representation
#[derive(Debug, Serialize, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub r#use: String,
    pub alg: String,
    pub kid: String,
    pub n: String,
    pub e: String,
}

/// JWKS representation
#[derive(Debug, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// Claims for access tokens (short-lived)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Email
    pub email: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// JWT ID (for blacklisting)
    pub jti: String,
}

/// Claims for refresh tokens (long-lived)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Token ID (matches database record)
    pub jti: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
}

/// Claims for app tokens (short-lived, for services)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTokenClaims {
    /// Subject (client ID)
    pub sub: String,
    /// Client ID (explicitly included as per specs)
    pub client_id: String,
    /// App Name
    pub name: String,
    /// Token Type (should be "app")
    pub typ: String,
    /// Scopes
    pub scopes: Vec<String>,
    /// Rate limit per minute
    pub rate_limit_per_min: u32,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// JWT ID (for blacklisting/tracking)
    pub jti: String,
}

use utoipa::ToSchema;

/// Token response returned to client
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    #[schema(example = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub access_token: String,
    #[schema(example = "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub refresh_token: String,
    #[schema(example = "Bearer")]
    pub token_type: String,
    #[schema(example = 900)]
    pub expires_in: i64,
}

impl JwtService {
    /// Create a new JWT service by loading RSA keys from files
    pub fn new(config: &JwtConfig) -> Result<Self, anyhow::Error> {
        // Load private key for signing
        let private_key_pem = fs::read_to_string(&config.private_key_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read private key from {}: {}",
                config.private_key_path,
                e
            )
        })?;

        let encoding_key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse private key: {}", e))?;

        // Load public key for verification
        let public_key_pem = fs::read_to_string(&config.public_key_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to read public key from {}: {}",
                config.public_key_path,
                e
            )
        })?;

        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to parse public key: {}", e))?;

        // Generate a key ID (SHA-256 hash of the public key PEM)
        let key_id = format!("{:x}", Sha256::digest(public_key_pem.as_bytes()))[..16].to_string();

        tracing::info!(key_id = %key_id, "JWT service initialized with RS256 keys");

        Ok(Self {
            encoding_key,
            decoding_key,
            public_key_pem,
            key_id,
            access_token_expiry_minutes: config.access_token_expiry_minutes,
            refresh_token_expiry_days: config.refresh_token_expiry_days,
            app_token_expiry_minutes: config.app_token_expiry_minutes,
        })
    }

    /// Get the public key set in JWKS format
    pub fn get_jwks(&self) -> Result<Jwks, anyhow::Error> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        use rsa::pkcs8::DecodePublicKey;
        use rsa::traits::PublicKeyParts;
        use rsa::RsaPublicKey;

        // Parse PEM to get RSA components
        // We might need to strip the PEM headers if DecodePublicKey doesn't handle them
        let public_key = RsaPublicKey::from_public_key_pem(&self.public_key_pem)
            .map_err(|e| anyhow::anyhow!("Failed to parse RSA public key for JWKS: {}", e))?;

        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());

        Ok(Jwks {
            keys: vec![Jwk {
                kty: "RSA".to_string(),
                r#use: "sig".to_string(),
                alg: "RS256".to_string(),
                kid: self.key_id.clone(),
                n,
                e,
            }],
        })
    }

    /// Generate an access token for a user
    pub fn generate_access_token(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<String, anyhow::Error> {
        let now = Utc::now();
        let exp = now + Duration::minutes(self.access_token_expiry_minutes);

        let claims = AccessTokenClaims {
            sub: user_id.to_string(),
            email: email.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.key_id.clone());
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode access token: {}", e))?;

        Ok(token)
    }

    /// Generate a refresh token for a user
    pub fn generate_refresh_token(
        &self,
        user_id: &str,
        token_id: &str,
    ) -> Result<String, anyhow::Error> {
        let now = Utc::now();
        let exp = now + Duration::days(self.refresh_token_expiry_days);

        let claims = RefreshTokenClaims {
            sub: user_id.to_string(),
            jti: token_id.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.key_id.clone());
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode refresh token: {}", e))?;

        Ok(token)
    }

    /// Generate both access and refresh tokens
    pub fn generate_token_pair(
        &self,
        user_id: &str,
        email: &str,
    ) -> Result<(String, String, String), anyhow::Error> {
        let access_token = self.generate_access_token(user_id, email)?;
        let refresh_token_id = Uuid::new_v4().to_string();
        let refresh_token = self.generate_refresh_token(user_id, &refresh_token_id)?;

        Ok((access_token, refresh_token, refresh_token_id))
    }

    /// Validate and decode an access token
    pub fn validate_access_token(&self, token: &str) -> Result<AccessTokenClaims, anyhow::Error> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<AccessTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow::anyhow!("Invalid access token: {}", e))?;

        Ok(token_data.claims)
    }

    /// Validate and decode a refresh token
    pub fn validate_refresh_token(&self, token: &str) -> Result<RefreshTokenClaims, anyhow::Error> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<RefreshTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow::anyhow!("Invalid refresh token: {}", e))?;

        Ok(token_data.claims)
    }

    /// Get access token expiry in seconds (for client info)
    pub fn access_token_expiry_seconds(&self) -> i64 {
        self.access_token_expiry_minutes * 60
    }

    /// Generate an app token for a client
    pub fn generate_app_token(
        &self,
        client_id: &str,
        app_name: &str,
        scopes: Vec<String>,
        rate_limit_per_min: u32,
    ) -> Result<String, anyhow::Error> {
        let now = Utc::now();
        let exp = now + Duration::minutes(self.app_token_expiry_minutes);

        let claims = AppTokenClaims {
            sub: client_id.to_string(),
            client_id: client_id.to_string(),
            name: app_name.to_string(),
            typ: "app".to_string(),
            scopes,
            rate_limit_per_min,
            exp: exp.timestamp(),
            iat: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(self.key_id.clone());
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow::anyhow!("Failed to encode app token: {}", e))?;

        Ok(token)
    }

    /// Validate and decode an app token
    pub fn validate_app_token(&self, token: &str) -> Result<AppTokenClaims, anyhow::Error> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<AppTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow::anyhow!("Invalid app token: {}", e))?;

        if token_data.claims.typ != "app" {
            return Err(anyhow::anyhow!("Invalid token type"));
        }

        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_keys() -> Result<(NamedTempFile, NamedTempFile), anyhow::Error> {
        // Generate test RSA key pair
        let private_key = r#"-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCazAniq0OLiSsC
OhQ+HVyptrwMEaWD5YJzz2I+yjCFcLRWcQ30j9xnyZO9Rxt2lYveqlH0A73+w3St
+lzZmhs3HnrpdWUIPgFxB2EiP9Hf6ty2/e29CdxACUPx7aGh5M2ViASOdzkeFUPY
NOFkYuxZTGNGMTH2JzTwPpAavvcXmZ994OO/BJx25IBhDSK+sgPgh1NceigiakfL
6LwTwIeenkPVaus9Gi1Gi2UrmL3hr/o5MMv4NAcN+nAzIvZHVlykOn1ci6Pm939L
DSYWiVZUoj7W0dFe6klL9XsnWaUROsb5W9IQKlwJDMfCs7FHDjERPoNCVwRd9/VE
j4IPu1kdAgMBAAECggEAL3KLNSc5tPN+c1hKDCAD3yFb0nc2PI+ExOq0OnrPFJfP
Lw/IL0ZJUKbA2iuJh3efP8kFBb5/5i8S/KDZBPnvjZ2SHy0Uosoetv6ED3NwaSoc
LRr4XBFBqX8tjGJCQNVZDpR6kRCKOWZbPVI4JAUOXPDFHSbHIaQy3dDPauNN6bV6
zX0DiQ3zNtVJ/Cygd0ndiVjgILKhxC9VnN4HRA3usLkXpo7jGiCV1J7XHTQsmB3X
Kkbn3uqtjkyy7ngcLuSq6sdx/EFQhsl7rvcweeNMHNRE/paKupoeulXxbWM9EpN2
qmFDRtA8ih3EfeUK1PZGdTfLkQWt5f/4dD9w61z4IQKBgQDNUSqO58NfMqVampfb
NySa34WuXoVTNMwtHDqzFAykfg+nXo8ABGv6SvNcIHL8CicwPSYSrd5JvbSCTwVs
tJsaC836xOjrZ0kK+oy8l4sycp6tERHNi7rTv64YfbmPE0Z77M60c1/KueOYBcKn
srNZZLPrHpxyjmFlToYvj/MpHwKBgQDBAk2DJsINL79+dE2PqUTCX9dq9ixDDQEt
mH2OOQj7Too49tOjvZP/iG5kPQ/Qkfjx2JZeru2xKzxunYa3qvwuHDeJYDvkilxa
G3NEeVZahvdp+ZknmGZKxgaZKgZP04kgW97PAcfFrqjzB8EcajwcjHLue2Qg5162
ceihyBeqQwKBgEpu5X3fWb3Wb4nUR79KU3PuGtmnHLCYkHi+Ji2r1BWCOgyUREVe
VQLtTyKUBPuIdsKPOJFHBTI4mwsuuKm7JAuiQe9qmYJV9G4NfR4V1nnYgdv+NzUM
NhP0BpqMYcwT0da1eA6FUTH+iBsh43rGVyzOTEet1kvVgEuo1w7BIgdDAoGAQkcx
KO1hS7fu0VTM4Z1l0D2rMr7QWkIX+nlX/EPXsry4uHECIkNSlDhceC2DxcKqsxoG
IQN++gz31qBfh6i+qnLkG1ehmYxtxD+S6JumLLYWNh0RG8i4r8qqr2QAAN+KQkNq
ErnwyRB+Ud6C0OgmNkOAoCZdLvNk0c/x68RTZBMCgYEAxXsNZwPZQBeQIjLZQeiR
3N1PS33NB4HcQP8K+wYLbW0PvjxeXUpMit2RmkKi4fFLX0rO7Huwa0rwJLPksJdy
szbJbBstFz1BZ8nwpJp1m/Ntqja3n74mp4MwSr6au1Db1SVJAOisMRZ3oIXuYI6m
C+AKS63xSUuh0BRfCg6QHGA=
-----END PRIVATE KEY-----"#;

        let public_key = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAmswJ4qtDi4krAjoUPh1c
qba8DBGlg+WCc89iPsowhXC0VnEN9I/cZ8mTvUcbdpWL3qpR9AO9/sN0rfpc2Zob
Nx566XVlCD4BcQdhIj/R3+rctv3tvQncQAlD8e2hoeTNlYgEjnc5HhVD2DThZGLs
WUxjRjEx9ic08D6QGr73F5mffeDjvwScduSAYQ0ivrID4IdTXHooImpHy+i8E8CH
np5D1WrrPRotRotlK5i94a/6OTDL+DQHDfpwMyL2R1ZcpDp9XIuj5vd/Sw0mFolW
VKI+1tHRXupJS/V7J1mlETrG+VvSECpcCQzHwrOxRw4xET6DQlcEXff1RI+CD7tZ
HQIDAQAB
-----END PUBLIC KEY-----"#;

        let mut private_file = NamedTempFile::new()?;
        private_file.write_all(private_key.as_bytes())?;

        let mut public_file = NamedTempFile::new()?;
        public_file.write_all(public_key.as_bytes())?;

        Ok((private_file, public_file))
    }

    #[test]
    fn test_jwt_service_creation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            app_token_expiry_minutes: 60,
        };

        let service = JwtService::new(&config)?;
        assert_eq!(service.access_token_expiry_minutes, 15);
        assert_eq!(service.refresh_token_expiry_days, 7);
        assert_eq!(service.app_token_expiry_minutes, 60);

        Ok(())
    }

    #[test]
    fn test_access_token_generation_and_validation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            app_token_expiry_minutes: 60,
        };

        let service = JwtService::new(&config)?;

        let token = service.generate_access_token("user_123", "test@example.com")?;
        assert!(!token.is_empty());

        let claims = service.validate_access_token(&token)?;
        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.email, "test@example.com");

        Ok(())
    }

    #[test]
    fn test_refresh_token_generation_and_validation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            app_token_expiry_minutes: 60,
        };

        let service = JwtService::new(&config)?;

        let token_id = "token_abc";
        let token = service.generate_refresh_token("user_123", token_id)?;
        assert!(!token.is_empty());

        let claims = service.validate_refresh_token(&token)?;
        assert_eq!(claims.sub, "user_123");
        assert_eq!(claims.jti, token_id);

        Ok(())
    }

    #[test]
    fn test_token_pair_generation() -> Result<(), anyhow::Error> {
        let (private_file, public_file) = create_test_keys()?;

        let config = JwtConfig {
            private_key_path: private_file.path().to_str().unwrap().to_string(),
            public_key_path: public_file.path().to_str().unwrap().to_string(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            app_token_expiry_minutes: 60,
        };

        let service = JwtService::new(&config)?;

        let (access_token, refresh_token, refresh_token_id) =
            service.generate_token_pair("user_123", "test@example.com")?;

        assert!(!access_token.is_empty());
        assert!(!refresh_token.is_empty());
        assert!(!refresh_token_id.is_empty());

        let access_claims = service.validate_access_token(&access_token)?;
        assert_eq!(access_claims.sub, "user_123");

        let refresh_claims = service.validate_refresh_token(&refresh_token)?;
        assert_eq!(refresh_claims.jti, refresh_token_id);

        Ok(())
    }
}
