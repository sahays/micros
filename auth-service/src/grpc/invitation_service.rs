//! gRPC implementation of InvitationService.

use crate::grpc::capability_check::require_capability;
use crate::grpc::proto::auth::{
    invitation_service_server::InvitationService, AcceptInvitationRequest,
    AcceptInvitationResponse, CreateInvitationRequest, CreateInvitationResponse,
    GetInvitationRequest, GetInvitationResponse, Invitation, InvitationStatus, LoginResponse,
    UserInfo,
};
use crate::models::{
    Invitation as ModelInvitation, OrgAssignment, RefreshSession, User, UserIdentity,
};
use crate::AppState;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use chrono::{Duration, Utc};
use prost_types::Timestamp;
use service_core::grpc::IntoStatus;
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// gRPC InvitationService implementation.
pub struct InvitationServiceImpl {
    state: AppState,
}

impl InvitationServiceImpl {
    /// Create a new InvitationServiceImpl.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Convert chrono DateTime to protobuf Timestamp.
fn datetime_to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

/// Hash a token for storage.
fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Hash a password using argon2.
fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
}

#[tonic::async_trait]
impl InvitationService for InvitationServiceImpl {
    async fn create_invitation(
        &self,
        request: Request<CreateInvitationRequest>,
    ) -> Result<Response<CreateInvitationResponse>, Status> {
        // Require user:invite capability
        let _auth = require_capability(&self.state, &request, "user:invite").await?;

        let req = request.into_inner();

        let tenant_id = Uuid::parse_str(&req.tenant_id)
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;
        let org_node_id = Uuid::parse_str(&req.org_node_id)
            .map_err(|_| Status::invalid_argument("Invalid org_node_id"))?;
        let role_id = Uuid::parse_str(&req.role_id)
            .map_err(|_| Status::invalid_argument("Invalid role_id"))?;
        let inviter_user_id = Uuid::parse_str(&req.inviter_user_id)
            .map_err(|_| Status::invalid_argument("Invalid inviter_user_id"))?;

        // Verify tenant exists
        self.state
            .db
            .find_tenant_by_id(tenant_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Tenant not found"))?;

        // Verify org node exists
        self.state
            .db
            .find_org_node_by_id(org_node_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Org node not found"))?;

        // Verify role exists
        self.state
            .db
            .find_role_by_id(role_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Role not found"))?;

        // Generate token
        let token = Uuid::new_v4().to_string();
        let token_hash = hash_token(&token);

        let expires_in_hours = req.expires_in_hours.unwrap_or(72) as i64;
        let expiry_utc = Utc::now() + Duration::hours(expires_in_hours);

        let invitation = ModelInvitation::new(
            tenant_id,
            req.email.clone(),
            org_node_id,
            role_id,
            token_hash,
            expiry_utc,
            inviter_user_id,
        );

        self.state
            .db
            .insert_invitation(&invitation)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(CreateInvitationResponse {
            invitation: Some(Invitation {
                invitation_id: invitation.invitation_id.to_string(),
                tenant_id: invitation.tenant_id.to_string(),
                email: invitation.email,
                org_node_id: invitation.org_node_id.to_string(),
                role_id: invitation.role_id.to_string(),
                status: InvitationStatus::Pending as i32,
                expires_utc: Some(datetime_to_timestamp(invitation.expiry_utc)),
                created_utc: Some(datetime_to_timestamp(invitation.created_utc)),
                inviter_user_id: invitation.created_by_user_id.to_string(),
            }),
        }))
    }

    async fn get_invitation(
        &self,
        request: Request<GetInvitationRequest>,
    ) -> Result<Response<GetInvitationResponse>, Status> {
        let req = request.into_inner();

        let token_hash = hash_token(&req.token);

        let invitation = self
            .state
            .db
            .find_invitation_by_token_hash(&token_hash)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Invitation not found"))?;

        let status = if invitation.accepted_utc.is_some() {
            InvitationStatus::Accepted
        } else if invitation.expiry_utc < Utc::now() {
            InvitationStatus::Expired
        } else {
            InvitationStatus::Pending
        };

        Ok(Response::new(GetInvitationResponse {
            invitation: Some(Invitation {
                invitation_id: invitation.invitation_id.to_string(),
                tenant_id: invitation.tenant_id.to_string(),
                email: invitation.email,
                org_node_id: invitation.org_node_id.to_string(),
                role_id: invitation.role_id.to_string(),
                status: status as i32,
                expires_utc: Some(datetime_to_timestamp(invitation.expiry_utc)),
                created_utc: Some(datetime_to_timestamp(invitation.created_utc)),
                inviter_user_id: invitation.created_by_user_id.to_string(),
            }),
        }))
    }

    async fn accept_invitation(
        &self,
        request: Request<AcceptInvitationRequest>,
    ) -> Result<Response<AcceptInvitationResponse>, Status> {
        let req = request.into_inner();

        let token_hash = hash_token(&req.token);

        let invitation = self
            .state
            .db
            .find_invitation_by_token_hash(&token_hash)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Invitation not found"))?;

        // Check if already accepted
        if invitation.accepted_utc.is_some() {
            return Err(Status::failed_precondition("Invitation already accepted"));
        }

        // Check if expired
        if invitation.expiry_utc < Utc::now() {
            return Err(Status::failed_precondition("Invitation has expired"));
        }

        // Validate password length
        if req.password.len() < 8 {
            return Err(Status::invalid_argument(
                "Password must be at least 8 characters",
            ));
        }

        // Create user
        let user = User::new(
            invitation.tenant_id,
            invitation.email.clone(),
            req.display_name,
        );
        let user_id = user.user_id;

        self.state
            .db
            .insert_user(&user)
            .await
            .map_err(|e| e.into_status())?;

        // Create user identity with password
        let password_hash = hash_password(&req.password)
            .map_err(|e| Status::internal(format!("Password hashing failed: {}", e)))?;
        let identity = UserIdentity::new_password(user_id, password_hash);

        self.state
            .db
            .insert_user_identity(&identity)
            .await
            .map_err(|e| e.into_status())?;

        // Create assignment
        let assignment = OrgAssignment::new(
            invitation.tenant_id,
            user_id,
            invitation.org_node_id,
            invitation.role_id,
        );

        self.state
            .db
            .insert_org_assignment(&assignment)
            .await
            .map_err(|e| e.into_status())?;

        // Mark invitation as accepted
        self.state
            .db
            .accept_invitation(invitation.invitation_id)
            .await
            .map_err(|e| e.into_status())?;

        // Generate tokens
        let (access_token, refresh_token, refresh_token_id) = self
            .state
            .jwt
            .generate_token_pair(
                &user_id.to_string(),
                &invitation.tenant_id.to_string(),
                "",
                &invitation.email,
            )
            .map_err(|e| Status::internal(format!("Token generation failed: {}", e)))?;

        // Store refresh session
        let refresh_hash = hash_token(&refresh_token_id);
        let session = RefreshSession::new(
            user_id,
            refresh_hash,
            self.state.jwt.refresh_token_expiry_days(),
        );
        self.state
            .db
            .insert_refresh_session(&session)
            .await
            .map_err(|e| e.into_status())?;

        let expires_in = self.state.config.jwt.access_token_expiry_minutes * 60;

        Ok(Response::new(AcceptInvitationResponse {
            auth: Some(LoginResponse {
                access_token,
                refresh_token,
                token_type: "Bearer".to_string(),
                expires_in,
                user: Some(UserInfo {
                    user_id: user_id.to_string(),
                    email: invitation.email,
                    display_name: user.display_name,
                    tenant_id: invitation.tenant_id.to_string(),
                }),
            }),
            message: "Invitation accepted successfully".to_string(),
        }))
    }
}
