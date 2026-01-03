use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use mongodb::bson::doc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    middleware::AuthUser,
    models::{RefreshToken, User, VerificationToken},
    services::TokenResponse,
    utils::{hash_password, verify_password, Password, PasswordHashString},
    AppState,
};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    req.validate().map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!("Validation error: {}", e),
            }),
        )
    })?;

    // Check if user already exists
    let existing_user = state
        .db
        .users()
        .find_one(doc! { "email": &req.email }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error checking existing user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    if existing_user.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Email already registered".to_string(),
            }),
        ));
    }

    // Hash password
    let password_hash = hash_password(&Password::new(req.password)).map_err(|e| {
        tracing::error!("Password hashing error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    // Create user
    let user = User::new(req.email.clone(), password_hash.into_string(), req.name);

    state
        .db
        .users()
        .insert_one(&user, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!("User registered: {}", user.id);

    // Generate verification token
    let token = generate_random_token();
    let verification_token =
        VerificationToken::new_email_verification(user.id.clone(), token.clone());

    state
        .db
        .verification_tokens()
        .insert_one(&verification_token, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error creating verification token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // Send verification email
    let base_url = format!("http://localhost:{}", state.config.port);
    state
        .email
        .send_verification_email(&req.email, &token, &base_url)
        .await
        .map_err(|e| {
            tracing::error!("Failed to send verification email: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to send verification email".to_string(),
                }),
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: user.id,
            message: "Registration successful. Please check your email to verify your account."
                .to_string(),
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub message: String,
}

pub async fn verify_email(
    State(state): State<AppState>,
    axum::extract::Query(req): axum::extract::Query<VerifyRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Find verification token
    let verification_token = state
        .db
        .verification_tokens()
        .find_one(doc! { "token": &req.token }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding verification token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let verification_token = verification_token.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Invalid or expired verification token".to_string(),
            }),
        )
    })?;

    // Check if token is expired
    if verification_token.is_expired() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Verification token has expired".to_string(),
            }),
        ));
    }

    // Update user as verified
    let result = state
        .db
        .users()
        .update_one(
            doc! { "_id": &verification_token.user_id },
            doc! { "$set": { "verified": true } },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!("Database error updating user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    if result.matched_count == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "User not found".to_string(),
            }),
        ));
    }

    // Delete used token
    state
        .db
        .verification_tokens()
        .delete_one(doc! { "_id": &verification_token.id }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error deleting verification token: {}", e);
            // Don't fail the request if token deletion fails
        })
        .ok();

    tracing::info!("Email verified for user: {}", verification_token.user_id);

    Ok((
        StatusCode::OK,
        Json(VerifyResponse {
            message: "Email verified successfully".to_string(),
        }),
    ))
}

fn generate_random_token() -> String {
    let mut rng = rand::thread_rng();
    let token_bytes: [u8; 32] = rng.gen();
    hex::encode(token_bytes)
}

// Login/Logout endpoints

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    req.validate().map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!("Validation error: {}", e),
            }),
        )
    })?;

    // Find user by email
    let user = state
        .db
        .users()
        .find_one(doc! { "email": &req.email }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let user = user.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid email or password".to_string(),
            }),
        )
    })?;

    // Verify password (constant-time comparison)
    verify_password(
        &Password::new(req.password),
        &PasswordHashString::new(user.password_hash.clone()),
    )
    .map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid email or password".to_string(),
            }),
        )
    })?;

    // Check if email is verified
    if !user.verified {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Email not verified. Please check your email for verification link."
                    .to_string(),
            }),
        ));
    }

    // Generate refresh token ID (this will be both the jti in JWT and _id in database)
    let refresh_token_id = uuid::Uuid::new_v4().to_string();

    // Generate JWT tokens
    let access_token = state
        .jwt
        .generate_access_token(&user.id, &user.email)
        .map_err(|e| {
            tracing::error!("Failed to generate access token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let refresh_token_str = state
        .jwt
        .generate_refresh_token(&user.id, &refresh_token_id)
        .map_err(|e| {
            tracing::error!("Failed to generate refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // Store refresh token in database with matching ID
    let refresh_token = RefreshToken::new_with_id(
        refresh_token_id,
        user.id.clone(),
        &refresh_token_str,
        state.config.jwt.refresh_token_expiry_days,
    );

    state
        .db
        .refresh_tokens()
        .insert_one(&refresh_token, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error storing refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!("User logged in: {}", user.id);

    Ok((
        StatusCode::OK,
        Json(TokenResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt.access_token_expiry_seconds(),
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

pub async fn logout(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<LogoutRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // 1. Blacklist the current access token
    let access_token_claims = user.0;
    let remaining_time = access_token_claims.exp - chrono::Utc::now().timestamp();
    
    if remaining_time > 0 {
        state
            .redis
            .blacklist_token(&access_token_claims.jti, remaining_time)
            .await
            .map_err(|e| {
                tracing::error!("Failed to blacklist access token: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
            })?;
    }

    // 2. Validate and decode refresh token
    let claims = state
        .jwt
        .validate_refresh_token(&req.refresh_token)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid refresh token".to_string(),
                }),
            )
        })?;

    // 3. Revoke refresh token in database
    let result = state
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
        .map_err(|e| {
            tracing::error!("Database error revoking refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    if result.matched_count == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Refresh token not found".to_string(),
            }),
        ));
    }

    tracing::info!("User logged out: {}", claims.sub);

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Logged out successfully"
        })),
    ))
}

use axum_extra::extract::cookie::{Cookie, CookieJar};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    email: String,
    name: Option<String>,
    #[allow(dead_code)]
    picture: Option<String>,
}

pub async fn google_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    axum::extract::Query(query): axum::extract::Query<GoogleCallbackQuery>,
) -> Result<(CookieJar, impl IntoResponse), (StatusCode, Json<ErrorResponse>)> {
    // 1. Validate state
    let stored_state = jar.get("oauth_state").map(|c| c.value());
    if stored_state != Some(&query.state) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid OAuth state".to_string(),
            }),
        ));
    }

    // 2. Get code verifier
    let code_verifier = jar.get("code_verifier").map(|c| c.value()).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Missing code verifier".to_string(),
            }),
        )
    })?;

    // 3. Exchange code for token
    let client = reqwest::Client::new();
    let token_response = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", &state.config.google.client_id),
            ("client_secret", &state.config.google.client_secret),
            ("code", &query.code),
            ("code_verifier", &code_verifier.to_string()),
            ("grant_type", &"authorization_code".to_string()),
            ("redirect_uri", &state.config.google.redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to exchange Google code: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to authenticate with Google".to_string(),
                }),
            )
        })?
        .json::<GoogleTokenResponse>()
        .await
        .map_err(|e| {
            tracing::error!("Failed to parse Google token response: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 4. Fetch user info
    let user_info = client
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(token_response.access_token)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch Google user info: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get user info from Google".to_string(),
                }),
            )
        })?
        .json::<GoogleUserInfo>()
        .await
        .map_err(|e| {
            tracing::error!("Failed to parse Google user info: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 5. Find or create user
    let user = state
        .db
        .users()
        .find_one(doc! { "email": &user_info.email }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let user = match user {
        Some(user) => user,
        None => {
            // Create new user for social login
            let new_user = User::new(
                user_info.email.clone(),
                "SOCIAL_LOGIN_NO_PASSWORD".to_string(),
                user_info.name,
            );
            // Social users are pre-verified
            let mut user = new_user;
            user.verified = true;

            state
                .db
                .users()
                .insert_one(&user, None)
                .await
                .map_err(|e| {
                    tracing::error!("Database error creating social user: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: "Internal server error".to_string(),
                        }),
                    )
                })?;
            user
        }
    };

    // 6. Issue tokens
    let (access_token, refresh_token_str, refresh_token_id) = state
        .jwt
        .generate_token_pair(&user.id, &user.email)
        .map_err(|e| {
            tracing::error!("Failed to generate token pair: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 7. Store refresh token
    let refresh_token = RefreshToken::new_with_id(
        refresh_token_id,
        user.id.clone(),
        &refresh_token_str,
        state.config.jwt.refresh_token_expiry_days,
    );

    state
        .db
        .refresh_tokens()
        .insert_one(&refresh_token, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error storing refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 8. Clean up cookies and return tokens
    let final_jar = jar
        .remove(Cookie::from("oauth_state"))
        .remove(Cookie::from("code_verifier"));

    Ok((
        final_jar,
        Json(TokenResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt.access_token_expiry_seconds(),
        }),
    ))
}

pub async fn google_login(
    State(state): State<AppState>,
    jar: CookieJar,
) -> (CookieJar, impl IntoResponse) {
    let oauth_state = generate_random_token();
    let code_verifier = generate_random_token();
    
    // Create code challenge
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

    let google_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth?\
        response_type=code&\
        client_id={}&\
        redirect_uri={}&\
        scope=openid%20email%20profile&\
        state={}&\
        code_challenge={}&\
        code_challenge_method=S256",
        state.config.google.client_id,
        state.config.google.redirect_uri,
        oauth_state,
        code_challenge
    );

    // Store state and verifier in cookies (secure, http_only)
    let updated_jar = jar
        .add(
            Cookie::build(("oauth_state", oauth_state))
                .path("/")
                .http_only(true)
                .max_age(time::Duration::minutes(15))
                .build(),
        )
        .add(
            Cookie::build(("code_verifier", code_verifier))
                .path("/")
                .http_only(true)
                .max_age(time::Duration::minutes(15))
                .build(),
        );

    (updated_jar, axum::response::Redirect::to(&google_url))
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate JWT signature and claims
    let claims = state
        .jwt
        .validate_refresh_token(&req.refresh_token)
        .map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid refresh token".to_string(),
                }),
            )
        })?;

    // Find the refresh token in the database
    let stored_token = state
        .db
        .refresh_tokens()
        .find_one(
            doc! {
                "_id": &claims.jti,
                "user_id": &claims.sub,
            },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!("Database error finding refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let stored_token = stored_token.ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Refresh token not found".to_string(),
            }),
        )
    })?;

    // Verify hashing and status
    // 1. Check if token is valid (not expired, not revoked)
    if !stored_token.is_valid() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Refresh token is invalid or expired".to_string(),
            }),
        ));
    }

    // 2. Verify the hash matches (Security check: prevents reuse of old tokens with same ID if we were reusing IDs, 
    // though here we use UUIDs, this checks if the token content matches what we expect)
    if stored_token.token_hash != RefreshToken::hash_token(&req.refresh_token) {
        // This is suspicious - ID matches but content doesn't
        tracing::warn!("Refresh token hash mismatch for user {}", claims.sub);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid refresh token".to_string(),
            }),
        ));
    }

    // Generate new tokens (Rotate)
    // We perform operations sequentially. Transactions would be better but require a replica set.
    
    // 1. Revoke the old token
    state
        .db
        .refresh_tokens()
        .update_one(
            doc! { "_id": &stored_token.id },
            doc! { "$set": { "revoked": true } },
            None,
        )
        .await
        .map_err(|e| {
            tracing::error!("Database error revoking old refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 2. Generate new tokens
    let new_refresh_token_id = uuid::Uuid::new_v4().to_string();

    // Note: We need user email for access token. Currently not in RefreshTokenClaims.
    // We should probably look up the user to get the email and ensure they still exist/are valid.
    let user = state
        .db
        .users()
        .find_one(doc! { "_id": &claims.sub }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "User not found".to_string(),
                }),
            )
        })?;
    
    // Check if user is still verified/active
    if !user.verified {
        return Err((
             StatusCode::FORBIDDEN,
             Json(ErrorResponse {
                 error: "User account is not verified".to_string(),
             }),
        ));
    }

    let access_token = state
        .jwt
        .generate_access_token(&user.id, &user.email)
        .map_err(|e| {
            tracing::error!("Failed to generate access token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let refresh_token_str = state
        .jwt
        .generate_refresh_token(&user.id, &new_refresh_token_id)
        .map_err(|e| {
            tracing::error!("Failed to generate refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // 3. Store new refresh token
    let new_refresh_token = RefreshToken::new_with_id(
        new_refresh_token_id,
        user.id.clone(),
        &refresh_token_str,
        state.config.jwt.refresh_token_expiry_days,
    );

    state
        .db
        .refresh_tokens()
        .insert_one(&new_refresh_token, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error storing new refresh token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    tracing::info!("Token refreshed for user: {}", user.id);

    Ok((
        StatusCode::OK,
        Json(TokenResponse {
            access_token,
            refresh_token: refresh_token_str,
            token_type: "Bearer".to_string(),
            expires_in: state.jwt.access_token_expiry_seconds(),
        }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct IntrospectRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct IntrospectResponse {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
}

pub async fn introspect(
    State(state): State<AppState>,
    Json(req): Json<IntrospectRequest>,
) -> impl IntoResponse {
    // 1. Validate token signature and expiration
    let claims = match state.jwt.validate_access_token(&req.token) {
        Ok(claims) => claims,
        Err(_) => {
            return Json(IntrospectResponse {
                active: false,
                sub: None,
                email: None,
                exp: None,
                iat: None,
                jti: None,
            });
        }
    };

    // 2. Check blacklist
    let is_blacklisted = match state.redis.is_blacklisted(&claims.jti).await {
        Ok(blacklisted) => blacklisted,
        Err(e) => {
            tracing::error!("Redis error checking blacklist during introspection: {}", e);
            // In case of Redis error, we fail closed (secure)
            true
        }
    };

    if is_blacklisted {
        return Json(IntrospectResponse {
            active: false,
            sub: None,
            email: None,
            exp: None,
            iat: None,
            jti: None,
        });
    }

    // 3. Return active with metadata
    Json(IntrospectResponse {
        active: true,
        sub: Some(claims.sub),
        email: Some(claims.email),
        exp: Some(claims.exp),
        iat: Some(claims.iat),
        jti: Some(claims.jti),
    })
}

#[derive(Debug, Deserialize, Validate)]
pub struct PasswordResetRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    req.validate().map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!("Validation error: {}", e),
            }),
        )
    })?;

    // Find user by email
    let user = state
        .db
        .users()
        .find_one(doc! { "email": &req.email }, None)
        .await
        .map_err(|e| {
            tracing::error!("Database error finding user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // If user exists, generate token and send email
    if let Some(user) = user {
        // Generate reset token
        let token = generate_random_token();
        let verification_token = VerificationToken::new_password_reset(user.id.clone(), token.clone());

        state
            .db
            .verification_tokens()
            .insert_one(&verification_token, None)
            .await
            .map_err(|e| {
                tracing::error!("Database error creating reset token: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
            })?;

        // Send reset email
        let base_url = format!("http://localhost:{}", state.config.port);
        state
            .email
            .send_password_reset_email(&req.email, &token, &base_url)
            .await
            .map_err(|e| {
                tracing::error!("Failed to send reset email: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Internal server error".to_string(),
                    }),
                )
            })?;

        tracing::info!("Password reset requested for user: {}", user.id);
    } else {
        // If user doesn't exist, we still return 200 OK to prevent email enumeration
        tracing::info!("Password reset requested for non-existent email: {}", req.email);
    }

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "If your email is registered, you will receive a password reset link shortly."
        })),
    ))
}

#[derive(Debug, Deserialize, Validate)]
pub struct PasswordResetConfirm {
    pub token: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
}

pub async fn confirm_password_reset(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetConfirm>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Validate request
    req.validate().map_err(|e| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!("Validation error: {}", e),
            }),
        )
    })?;

    // Find reset token
    let verification_token = state
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
        .map_err(|e| {
            tracing::error!("Database error finding reset token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    let verification_token = verification_token.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid or expired reset token".to_string(),
            }),
        )
    })?;

    // Check if token is expired
    if verification_token.is_expired() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Reset token has expired".to_string(),
            }),
        ));
    }

    // Hash new password
    let password_hash = hash_password(&Password::new(req.new_password)).map_err(|e| {
        tracing::error!("Password hashing error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    // Update user password and invalidate all refresh tokens
    let session = state.db.client().start_session(None).await.map_err(|e| {
        tracing::error!("Failed to start database session: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    // We use a transaction to ensure atomicity
    let mut session = session;
    session.start_transaction(None).await.map_err(|e| {
        tracing::error!("Failed to start transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    // Update password
    state
        .db
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
        .map_err(|e| {
            tracing::error!("Database error updating user password: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // Invalidate all refresh tokens for the user
    state
        .db
        .refresh_tokens()
        .update_many_with_session(
            doc! { "user_id": &verification_token.user_id },
            doc! { "$set": { "revoked": true } },
            None,
            &mut session,
        )
        .await
        .map_err(|e| {
            tracing::error!("Database error revoking refresh tokens: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    // Delete the used reset token
    state
        .db
        .verification_tokens()
        .delete_one_with_session(doc! { "_id": &verification_token.id }, None, &mut session)
        .await
        .map_err(|e| {
            tracing::error!("Database error deleting reset token: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                }),
            )
        })?;

    session.commit_transaction().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Internal server error".to_string(),
            }),
        )
    })?;

    tracing::info!("Password reset successful for user: {}", verification_token.user_id);

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Password reset successful. You can now login with your new password."
        })),
    ))
}
