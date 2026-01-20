//! gRPC implementation of AuthService.

use service_core::grpc::IntoStatus;
use tonic::{Request, Response, Status};

use crate::grpc::proto::auth::{
    auth_service_server::AuthService, LoginRequest, LoginResponse, LogoutRequest, RefreshRequest,
    RefreshResponse, RegisterRequest, RegisterResponse, SendOtpRequest, SendOtpResponse,
    ValidateTokenRequest, ValidateTokenResponse, VerifyOtpRequest, VerifyOtpResponse,
};
use crate::handlers::auth as auth_handler;
use crate::handlers::otp as otp_handler;
use crate::AppState;

/// gRPC AuthService implementation.
pub struct AuthServiceImpl {
    state: AppState,
}

impl AuthServiceImpl {
    /// Create a new AuthServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();

        // Convert to handler request type
        let handler_req = auth_handler::RegisterRequest {
            tenant_slug: req.tenant_slug,
            email: req.email,
            password: req.password,
            display_name: req.display_name,
        };

        // Call existing handler logic
        let result = auth_handler::register_impl(&self.state, handler_req)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(RegisterResponse {
            access_token: result.access_token,
            refresh_token: result.refresh_token,
            token_type: result.token_type,
            expires_in: result.expires_in,
            user: Some(super::proto::auth::UserInfo {
                user_id: result.user.user_id.to_string(),
                email: result.user.email,
                display_name: result.user.display_name,
                tenant_id: result.user.tenant_id.to_string(),
            }),
        }))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        let req = request.into_inner();

        let handler_req = auth_handler::LoginRequest {
            tenant_slug: req.tenant_slug,
            email: req.email,
            password: req.password,
        };

        let result = auth_handler::login_impl(&self.state, handler_req)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(LoginResponse {
            access_token: result.access_token,
            refresh_token: result.refresh_token,
            token_type: result.token_type,
            expires_in: result.expires_in,
            user: Some(super::proto::auth::UserInfo {
                user_id: result.user.user_id.to_string(),
                email: result.user.email,
                display_name: result.user.display_name,
                tenant_id: result.user.tenant_id.to_string(),
            }),
        }))
    }

    async fn refresh(
        &self,
        request: Request<RefreshRequest>,
    ) -> Result<Response<RefreshResponse>, Status> {
        let req = request.into_inner();

        let handler_req = auth_handler::RefreshRequest {
            refresh_token: req.refresh_token,
        };

        let result = auth_handler::refresh_impl(&self.state, handler_req)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(RefreshResponse {
            access_token: result.access_token,
            refresh_token: result.refresh_token,
            token_type: result.token_type,
            expires_in: result.expires_in,
            user: Some(super::proto::auth::UserInfo {
                user_id: result.user.user_id.to_string(),
                email: result.user.email,
                display_name: result.user.display_name,
                tenant_id: result.user.tenant_id.to_string(),
            }),
        }))
    }

    async fn logout(&self, request: Request<LogoutRequest>) -> Result<Response<()>, Status> {
        let req = request.into_inner();

        let handler_req = auth_handler::LogoutRequest {
            refresh_token: req.refresh_token,
        };

        auth_handler::logout_impl(&self.state, handler_req)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(()))
    }

    async fn validate_token(
        &self,
        request: Request<ValidateTokenRequest>,
    ) -> Result<Response<ValidateTokenResponse>, Status> {
        let req = request.into_inner();

        // Validate the token using JWT service
        match self.state.jwt.validate_access_token(&req.access_token) {
            Ok(claims) => {
                // Convert claims to proto format
                let proto_claims = super::proto::auth::TokenClaims {
                    sub: claims.sub,
                    app_id: claims.app_id,
                    org_id: claims.org_id,
                    email: claims.email,
                    jti: claims.jti,
                    token_type: "access".to_string(),
                    iat: None, // Could add if needed
                    exp: None, // Could add if needed
                };

                Ok(Response::new(ValidateTokenResponse {
                    valid: true,
                    claims: Some(proto_claims),
                }))
            }
            Err(_) => Ok(Response::new(ValidateTokenResponse {
                valid: false,
                claims: None,
            })),
        }
    }

    async fn send_otp(
        &self,
        request: Request<SendOtpRequest>,
    ) -> Result<Response<SendOtpResponse>, Status> {
        let req = request.into_inner();

        let tenant_id = uuid::Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id format"))?;

        let channel = match req.channel {
            1 => crate::models::OtpChannel::Email,
            2 => crate::models::OtpChannel::Sms,
            3 => crate::models::OtpChannel::Whatsapp,
            _ => return Err(Status::invalid_argument("Invalid OTP channel")),
        };

        let purpose = match req.purpose {
            1 => crate::models::OtpPurpose::Login,
            2 => crate::models::OtpPurpose::VerifyEmail,
            3 => crate::models::OtpPurpose::VerifyPhone,
            4 => crate::models::OtpPurpose::ResetPassword,
            _ => return Err(Status::invalid_argument("Invalid OTP purpose")),
        };

        let handler_req = otp_handler::SendOtpRequest {
            tenant_id,
            destination: req.destination,
            channel,
            purpose,
        };

        let result = otp_handler::send_otp_impl(&self.state, handler_req)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(SendOtpResponse {
            otp_id: result.otp_id.to_string(),
            expires_in: result.expires_in,
        }))
    }

    async fn verify_otp(
        &self,
        request: Request<VerifyOtpRequest>,
    ) -> Result<Response<VerifyOtpResponse>, Status> {
        let req = request.into_inner();

        let otp_id = uuid::Uuid::parse_str(&req.otp_id)
            .map_err(|_| Status::invalid_argument("Invalid otp_id format"))?;

        let handler_req = otp_handler::VerifyOtpRequest {
            otp_id,
            code: req.code,
        };

        let result = otp_handler::verify_otp_impl(&self.state, handler_req)
            .await
            .map_err(|e| e.into_status())?;

        let (verified, purpose, auth) = match result {
            otp_handler::VerifyOtpResponse::Login(login_resp) => (
                true,
                super::proto::auth::OtpPurpose::Login as i32,
                Some(LoginResponse {
                    access_token: login_resp.access_token,
                    refresh_token: login_resp.refresh_token,
                    token_type: login_resp.token_type,
                    expires_in: login_resp.expires_in,
                    user: Some(super::proto::auth::UserInfo {
                        user_id: login_resp.user_id.to_string(),
                        email: String::new(), // Not available in this response
                        display_name: None,
                        tenant_id: String::new(),
                    }),
                }),
            ),
            otp_handler::VerifyOtpResponse::Verify(verify_resp) => {
                let purpose = match verify_resp.purpose.as_str() {
                    "verify_email" => super::proto::auth::OtpPurpose::VerifyEmail as i32,
                    "verify_phone" => super::proto::auth::OtpPurpose::VerifyPhone as i32,
                    "reset_password" => super::proto::auth::OtpPurpose::ResetPassword as i32,
                    _ => super::proto::auth::OtpPurpose::Unspecified as i32,
                };
                (verify_resp.verified, purpose, None)
            }
        };

        Ok(Response::new(VerifyOtpResponse {
            verified,
            purpose,
            auth,
        }))
    }
}
