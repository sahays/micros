use crate::{
    dtos::auth::{
        IntrospectResponse, LoginRequest, PasswordResetConfirm, PasswordResetRequest,
        RefreshRequest, RegisterRequest, RegisterResponse, VerifyResponse,
    },
    models::{AuditLog, RefreshToken, User, VerificationToken},
    services::{
        EmailProvider, JwtService, MongoDb, PolicyService, ServiceError, TokenBlacklist,
        TokenResponse,
    },
    utils::{hash_password, verify_password, Password, PasswordHashString},
};
use mongodb::bson::doc;
use rand::Rng;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthService {
    db: MongoDb,
    email: Arc<dyn EmailProvider>,
    jwt: JwtService,
    redis: Arc<dyn TokenBlacklist>,
}

impl AuthService {
    pub fn new(
        db: MongoDb,
        email: Arc<dyn EmailProvider>,
        jwt: JwtService,
        redis: Arc<dyn TokenBlacklist>,
    ) -> Self {
        Self {
            db,
            email,
            jwt,
            redis,
        }
    }

    pub async fn register(
        &self,
        req: RegisterRequest,
        app_id: String,
        org_id: String,
        ip_address: String,
        base_url: String,
    ) -> Result<RegisterResponse, ServiceError> {
        // Load organization and validate it exists and is enabled
        let org = self
            .db
            .find_organization_in_app(&app_id, &org_id)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::OrganizationNotFound)?;

        if !org.enabled {
            return Err(ServiceError::OrganizationDisabled);
        }

        // Validate password against organization's auth policy
        PolicyService::validate_password(&req.password, &org.auth_policy)?;

        // Check if user already exists within this tenant
        if self
            .db
            .find_user_by_email_in_tenant(&app_id, &org_id, &req.email)
            .await
            .map_err(ServiceError::Database)?
            .is_some()
        {
            return Err(ServiceError::EmailAlreadyRegistered);
        }

        // Hash password
        let password_hash = hash_password(&Password::new(req.password.clone())).map_err(|e| {
            ServiceError::Internal(anyhow::anyhow!("Password hashing error: {}", e))
        })?;

        // Create user with tenant context
        let user = User::new(
            app_id,
            org_id,
            req.email.clone(),
            password_hash.into_string(),
            req.name,
        );

        self.db
            .users()
            .insert_one(&user, None)
            .await
            .map_err(ServiceError::Database)?;

        tracing::info!(user_id = %user.id, "User registered");

        // Audit log registration
        let audit_log = AuditLog::new(
            "user_registration".to_string(),
            Some(user.id.clone()),
            "/auth/register".to_string(),
            "POST".to_string(),
            201,
            ip_address,
        );
        let db = self.db.clone();
        tokio::spawn(async move {
            let _ = db.audit_logs().insert_one(audit_log, None).await;
        });

        // Generate verification token
        let token = generate_random_token();
        let verification_token =
            VerificationToken::new_email_verification(user.id.clone(), token.clone());

        self.db
            .verification_tokens()
            .insert_one(&verification_token, None)
            .await
            .map_err(ServiceError::Database)?;

        // Send verification email
        self.email
            .send_verification_email(&req.email, &token, &base_url)
            .await
            .map_err(|e| ServiceError::EmailError(e.to_string()))?;

        Ok(RegisterResponse {
            user_id: user.id,
            message: "Registration successful. Please check your email to verify your account."
                .to_string(),
        })
    }

    pub async fn verify_email(&self, token: String) -> Result<VerifyResponse, ServiceError> {
        // Find verification token
        let verification_token = self
            .db
            .find_token_by_token(&token)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::InvalidToken)?;

        // Check if token is expired
        if verification_token.is_expired() {
            return Err(ServiceError::TokenExpired);
        }

        // Update user as verified
        let result = self
            .db
            .users()
            .update_one(
                doc! { "_id": &verification_token.user_id },
                doc! { "$set": { "verified": true } },
                None,
            )
            .await
            .map_err(ServiceError::Database)?;

        if result.matched_count == 0 {
            return Err(ServiceError::UserNotFound);
        }

        // Delete used token
        let _ = self
            .db
            .verification_tokens()
            .delete_one(doc! { "_id": &verification_token.id }, None)
            .await;

        tracing::info!(user_id = %verification_token.user_id, "Email verified for user");

        Ok(VerifyResponse {
            message: "Email verified successfully".to_string(),
        })
    }

    pub async fn login(
        &self,
        req: LoginRequest,
        refresh_token_expiry_days: i64,
    ) -> Result<TokenResponse, ServiceError> {
        // Find user by email
        let user = self
            .db
            .find_user_by_email(&req.email)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::InvalidCredentials)?;

        // Verify the user's organization is still enabled
        let org = self
            .db
            .find_organization_in_app(&user.app_id, &user.org_id)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::OrganizationNotFound)?;

        if !org.enabled {
            return Err(ServiceError::OrganizationDisabled);
        }

        // TODO(Story #279): Implement account lockout based on org.auth_policy.max_failed_attempts
        // This would require tracking failed login attempts per user

        // Verify password
        verify_password(
            &Password::new(req.password),
            &PasswordHashString::new(user.password_hash.clone()),
        )
        .map_err(|_| ServiceError::InvalidCredentials)?;

        // Check if email is verified
        if !user.verified {
            return Err(ServiceError::EmailError(
                "Email not verified. Please check your email for verification link.".to_string(),
            ));
        }

        // Generate refresh token ID
        let refresh_token_id = uuid::Uuid::new_v4().to_string();

        // Generate JWT tokens with tenant context
        let access_token = self
            .jwt
            .generate_access_token(&user.id, &user.app_id, &user.org_id, &user.email)
            .map_err(ServiceError::Internal)?;

        let refresh_token_str = self
            .jwt
            .generate_refresh_token(&user.id, &refresh_token_id)
            .map_err(ServiceError::Internal)?;

        // Store refresh token in database
        let refresh_token = RefreshToken::new_with_id(
            refresh_token_id,
            user.id.clone(),
            &refresh_token_str,
            refresh_token_expiry_days,
        );

        self.db
            .refresh_tokens()
            .insert_one(&refresh_token, None)
            .await
            .map_err(ServiceError::Database)?;

        Ok(TokenResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt.access_token_expiry_seconds(),
        })
    }

    pub async fn logout(
        &self,
        refresh_token: String,
        access_token_jti: String,
        access_token_exp: i64,
        ip_address: String,
    ) -> Result<(), ServiceError> {
        // 1. Blacklist the current access token
        let remaining_time = access_token_exp - chrono::Utc::now().timestamp();
        if remaining_time > 0 {
            self.redis
                .blacklist_token(&access_token_jti, remaining_time)
                .await
                .map_err(ServiceError::Internal)?;
        }

        // 2. Validate and decode refresh token
        let claims = self
            .jwt
            .validate_refresh_token(&refresh_token)
            .map_err(|_| ServiceError::InvalidToken)?;

        // 3. Revoke refresh token in database
        let result = self
            .db
            .refresh_tokens()
            .update_one(
                doc! {
                    "_id": &claims.jti,
                    "user_id": &claims.sub,
                },
                doc! {
                    "$set": { "revoked": true }
                },
                None,
            )
            .await
            .map_err(ServiceError::Database)?;

        if result.matched_count == 0 {
            return Err(ServiceError::InvalidToken);
        }

        tracing::info!(user_id = %claims.sub, "User logged out");

        // Audit log logout
        let audit_log = AuditLog::new(
            "user_logout".to_string(),
            Some(claims.sub.clone()),
            "/auth/logout".to_string(),
            "POST".to_string(),
            200,
            ip_address,
        );
        let db = self.db.clone();
        tokio::spawn(async move {
            let _ = db.audit_logs().insert_one(audit_log, None).await;
        });

        Ok(())
    }

    pub async fn refresh(&self, req: RefreshRequest) -> Result<TokenResponse, ServiceError> {
        // Validate JWT signature and claims
        let claims = self
            .jwt
            .validate_refresh_token(&req.refresh_token)
            .map_err(|_| ServiceError::InvalidToken)?;

        // Find the refresh token in the database
        let stored_token = self
            .db
            .find_refresh_token_by_id(&claims.jti)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::InvalidToken)?;

        // Verify hashing and status
        if !stored_token.is_valid() {
            return Err(ServiceError::InvalidToken);
        }

        if stored_token.token_hash != RefreshToken::hash_token(&req.refresh_token) {
            tracing::warn!(user_id = %claims.sub, "Refresh token hash mismatch");
            return Err(ServiceError::InvalidToken);
        }

        // 1. Revoke the old token
        self.db
            .refresh_tokens()
            .update_one(
                doc! { "_id": &stored_token.id },
                doc! { "$set": { "revoked": true } },
                None,
            )
            .await
            .map_err(ServiceError::Database)?;

        // 2. Find user
        let user = self
            .db
            .find_user_by_id(&claims.sub)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::UserNotFound)?;

        if !user.verified {
            return Err(ServiceError::EmailError(
                "User account is not verified".to_string(),
            ));
        }

        // 3. Generate new tokens with tenant context
        let new_refresh_token_id = uuid::Uuid::new_v4().to_string();
        let access_token = self
            .jwt
            .generate_access_token(&user.id, &user.app_id, &user.org_id, &user.email)
            .map_err(ServiceError::Internal)?;
        let refresh_token_str = self
            .jwt
            .generate_refresh_token(&user.id, &new_refresh_token_id)
            .map_err(ServiceError::Internal)?;

        // 4. Store new refresh token
        let new_refresh_token = RefreshToken::new_with_id(
            new_refresh_token_id,
            user.id.clone(),
            &refresh_token_str,
            self.jwt.refresh_token_expiry_days(),
        );

        self.db
            .refresh_tokens()
            .insert_one(&new_refresh_token, None)
            .await
            .map_err(ServiceError::Database)?;

        tracing::info!(user_id = %user.id, "Token refreshed for user");

        Ok(TokenResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt.access_token_expiry_seconds(),
        })
    }

    pub async fn introspect(&self, token: String) -> IntrospectResponse {
        // 1. Validate token signature and expiration
        let claims = match self.jwt.validate_access_token(&token) {
            Ok(claims) => claims,
            Err(_) => {
                return IntrospectResponse {
                    active: false,
                    sub: None,
                    email: None,
                    exp: None,
                    iat: None,
                    jti: None,
                };
            }
        };

        // 2. Check blacklist
        let is_blacklisted = match self.redis.is_blacklisted(&claims.jti).await {
            Ok(blacklisted) => blacklisted,
            Err(e) => {
                tracing::error!(error = %e, "Redis error checking blacklist during introspection");
                true
            }
        };

        if is_blacklisted {
            return IntrospectResponse {
                active: false,
                sub: None,
                email: None,
                exp: None,
                iat: None,
                jti: None,
            };
        }

        // 3. Return active with metadata
        IntrospectResponse {
            active: true,
            sub: Some(claims.sub),
            email: Some(claims.email),
            exp: Some(claims.exp),
            iat: Some(claims.iat),
            jti: Some(claims.jti),
        }
    }

    pub async fn request_password_reset(
        &self,
        req: PasswordResetRequest,
        ip_address: String,
        base_url: String,
    ) -> Result<(), ServiceError> {
        let user = self
            .db
            .find_user_by_email(&req.email)
            .await
            .map_err(ServiceError::Database)?;

        if let Some(user) = user {
            let token = generate_random_token();
            let verification_token =
                VerificationToken::new_password_reset(user.id.clone(), token.clone());

            self.db
                .verification_tokens()
                .insert_one(&verification_token, None)
                .await
                .map_err(ServiceError::Database)?;

            self.email
                .send_password_reset_email(&req.email, &token, &base_url)
                .await
                .map_err(|e| ServiceError::EmailError(e.to_string()))?;

            tracing::info!(user_id = %user.id, "Password reset requested");

            let audit_log = AuditLog::new(
                "password_reset_request".to_string(),
                Some(user.id.clone()),
                "/auth/password-reset/request".to_string(),
                "POST".to_string(),
                200,
                ip_address,
            );
            let db = self.db.clone();
            tokio::spawn(async move {
                let _ = db.audit_logs().insert_one(audit_log, None).await;
            });
        }

        Ok(())
    }

    pub async fn confirm_password_reset(
        &self,
        req: PasswordResetConfirm,
        ip_address: String,
    ) -> Result<(), ServiceError> {
        let verification_token = self
            .db
            .verification_tokens()
            .find_one(
                doc! {
                    "token": &req.token,
                    "token_type": "password_reset"
                },
                None,
            )
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::InvalidToken)?;

        if verification_token.is_expired() {
            return Err(ServiceError::TokenExpired);
        }

        // Load user to get their org context
        let user = self
            .db
            .find_user_by_id(&verification_token.user_id)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::UserNotFound)?;

        // Load organization and validate password against org's auth policy
        let org = self
            .db
            .find_organization_in_app(&user.app_id, &user.org_id)
            .await
            .map_err(ServiceError::Database)?
            .ok_or(ServiceError::OrganizationNotFound)?;

        PolicyService::validate_password(&req.new_password, &org.auth_policy)?;

        let password_hash =
            hash_password(&Password::new(req.new_password.clone())).map_err(|e| {
                ServiceError::Internal(anyhow::anyhow!("Password hashing error: {}", e))
            })?;

        let session = self
            .db
            .client()
            .start_session(None)
            .await
            .map_err(ServiceError::Database)?;
        let mut session = session;
        session
            .start_transaction(None)
            .await
            .map_err(ServiceError::Database)?;

        self.db
            .users()
            .update_one_with_session(
                doc! { "_id": &verification_token.user_id },
                doc! {
                    "$set": {
                        "password_hash": password_hash.into_string(),
                        "updated_at": chrono::Utc::now()
                    }
                },
                None,
                &mut session,
            )
            .await
            .map_err(ServiceError::Database)?;

        self.db
            .refresh_tokens()
            .update_many_with_session(
                doc! { "user_id": &verification_token.user_id },
                doc! { "$set": { "revoked": true } },
                None,
                &mut session,
            )
            .await
            .map_err(ServiceError::Database)?;

        self.db
            .verification_tokens()
            .delete_one_with_session(doc! { "_id": &verification_token.id }, None, &mut session)
            .await
            .map_err(ServiceError::Database)?;

        session
            .commit_transaction()
            .await
            .map_err(ServiceError::Database)?;

        tracing::info!(user_id = %verification_token.user_id, "Password reset successful");

        let audit_log = AuditLog::new(
            "password_reset_confirm".to_string(),
            Some(verification_token.user_id.clone()),
            "/auth/password-reset/confirm".to_string(),
            "POST".to_string(),
            200,
            ip_address,
        );
        let db = self.db.clone();
        tokio::spawn(async move {
            let _ = db.audit_logs().insert_one(audit_log, None).await;
        });

        Ok(())
    }
}

fn generate_random_token() -> String {
    let mut rng = rand::thread_rng();
    let token_bytes: [u8; 32] = rng.gen();
    hex::encode(token_bytes)
}
