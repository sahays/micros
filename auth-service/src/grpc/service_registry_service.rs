//! gRPC implementation of ServiceRegistryService (KYS - Know Your Service).

use crate::grpc::proto::auth::{
    service_registry_service_server::ServiceRegistryService, GetServicePermissionsRequest,
    GetServicePermissionsResponse, GetServiceRequest, GetServiceResponse, GetServiceTokenRequest,
    GetServiceTokenResponse, GrantPermissionRequest, GrantPermissionResponse,
    RegisterServiceRequest, RegisterServiceResponse, RotateSecretRequest, RotateSecretResponse,
    Service as ProtoService,
};
use crate::models::{Service, ServiceSecret};
use crate::AppState;
use prost_types::Timestamp;
use service_core::grpc::IntoStatus;
use sha2::{Digest, Sha256};
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// gRPC ServiceRegistryService implementation.
pub struct ServiceRegistryServiceImpl {
    state: AppState,
}

impl ServiceRegistryServiceImpl {
    /// Create a new ServiceRegistryServiceImpl.
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

/// Generate a secure random secret.
fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

/// Hash a secret for storage.
fn hash_secret(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

#[tonic::async_trait]
impl ServiceRegistryService for ServiceRegistryServiceImpl {
    async fn register_service(
        &self,
        request: Request<RegisterServiceRequest>,
    ) -> Result<Response<RegisterServiceResponse>, Status> {
        let req = request.into_inner();

        // Parse optional tenant_id
        let tenant_id = req
            .tenant_id
            .filter(|s| !s.is_empty())
            .map(|s| Uuid::parse_str(&s))
            .transpose()
            .map_err(|_| Status::invalid_argument("Invalid tenant_id"))?;

        // Check if service key already exists
        if self
            .state
            .db
            .find_service_by_key(&req.svc_key)
            .await
            .map_err(|e| e.into_status())?
            .is_some()
        {
            return Err(Status::already_exists("Service key already registered"));
        }

        // Create service
        let service = Service::new(tenant_id, req.svc_key.clone(), req.svc_name.clone());

        self.state
            .db
            .insert_service(&service)
            .await
            .map_err(|e| e.into_status())?;

        // Generate and store secret
        let client_secret = generate_secret();
        let secret_hash = hash_secret(&client_secret);

        let secret = ServiceSecret::new(service.svc_id, secret_hash);

        self.state
            .db
            .insert_service_secret(&secret)
            .await
            .map_err(|e| e.into_status())?;

        let active = service.is_active();
        Ok(Response::new(RegisterServiceResponse {
            service: Some(ProtoService {
                svc_id: service.svc_id.to_string(),
                svc_key: service.svc_key,
                svc_name: service.svc_label,
                description: req.description,
                active,
                created_utc: Some(datetime_to_timestamp(service.created_utc)),
            }),
            client_secret,
        }))
    }

    async fn get_service_token(
        &self,
        request: Request<GetServiceTokenRequest>,
    ) -> Result<Response<GetServiceTokenResponse>, Status> {
        let req = request.into_inner();

        // Find service
        let service = self
            .state
            .db
            .find_service_by_key(&req.svc_key)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Service not found"))?;

        if !service.is_active() {
            return Err(Status::permission_denied("Service is disabled"));
        }

        // Verify secret
        let secret_hash = hash_secret(&req.client_secret);
        let valid_secret = self
            .state
            .db
            .find_valid_service_secret(service.svc_id)
            .await
            .map_err(|e| e.into_status())?;

        // Check if secret matches
        let secret =
            valid_secret.ok_or_else(|| Status::unauthenticated("No valid secret found"))?;
        if secret.secret_hash_text != secret_hash {
            return Err(Status::unauthenticated("Invalid client secret"));
        }

        // Get service permissions
        let permissions = self
            .state
            .db
            .get_service_permissions(service.svc_id)
            .await
            .map_err(|e| e.into_status())?;

        // Generate service token
        let access_token = self
            .state
            .jwt
            .generate_app_token(
                &service.svc_id.to_string(),
                &service.svc_label,
                permissions,
                0, // No rate limit for services
            )
            .map_err(|e| Status::internal(format!("Token generation failed: {}", e)))?;

        let expires_in = self.state.config.jwt.app_token_expiry_minutes * 60;

        Ok(Response::new(GetServiceTokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in,
        }))
    }

    async fn get_service(
        &self,
        request: Request<GetServiceRequest>,
    ) -> Result<Response<GetServiceResponse>, Status> {
        let req = request.into_inner();

        let service = self
            .state
            .db
            .find_service_by_key(&req.svc_key)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Service not found"))?;

        let active = service.is_active();
        Ok(Response::new(GetServiceResponse {
            service: Some(ProtoService {
                svc_id: service.svc_id.to_string(),
                svc_key: service.svc_key,
                svc_name: service.svc_label,
                description: None, // No description field in model
                active,
                created_utc: Some(datetime_to_timestamp(service.created_utc)),
            }),
        }))
    }

    async fn rotate_secret(
        &self,
        request: Request<RotateSecretRequest>,
    ) -> Result<Response<RotateSecretResponse>, Status> {
        let req = request.into_inner();

        // Find service
        let service = self
            .state
            .db
            .find_service_by_key(&req.svc_key)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Service not found"))?;

        // Verify current secret
        let current_hash = hash_secret(&req.current_secret);
        let valid_secret = self
            .state
            .db
            .find_valid_service_secret(service.svc_id)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::unauthenticated("No valid secret found"))?;

        if valid_secret.secret_hash_text != current_hash {
            return Err(Status::unauthenticated("Invalid current secret"));
        }

        // Revoke old secret
        self.state
            .db
            .revoke_service_secret(valid_secret.secret_id)
            .await
            .map_err(|e| e.into_status())?;

        // Generate and store new secret
        let new_secret = generate_secret();
        let new_hash = hash_secret(&new_secret);

        let secret = ServiceSecret::new(service.svc_id, new_hash);

        self.state
            .db
            .insert_service_secret(&secret)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(RotateSecretResponse {
            new_secret,
            message: "Secret rotated successfully".to_string(),
        }))
    }

    async fn get_service_permissions(
        &self,
        request: Request<GetServicePermissionsRequest>,
    ) -> Result<Response<GetServicePermissionsResponse>, Status> {
        let req = request.into_inner();

        // Find service
        let service = self
            .state
            .db
            .find_service_by_key(&req.svc_key)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Service not found"))?;

        let permissions = self
            .state
            .db
            .get_service_permissions(service.svc_id)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(GetServicePermissionsResponse { permissions }))
    }

    async fn grant_permission(
        &self,
        request: Request<GrantPermissionRequest>,
    ) -> Result<Response<GrantPermissionResponse>, Status> {
        let req = request.into_inner();

        // Find service
        let service = self
            .state
            .db
            .find_service_by_key(&req.svc_key)
            .await
            .map_err(|e| e.into_status())?
            .ok_or_else(|| Status::not_found("Service not found"))?;

        // Insert permission
        self.state
            .db
            .insert_service_permission(service.svc_id, &req.permission)
            .await
            .map_err(|e| e.into_status())?;

        Ok(Response::new(GrantPermissionResponse {
            message: "Permission granted successfully".to_string(),
        }))
    }
}
