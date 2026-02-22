//! gRPC utilities for micros microservices.
//!
//! This module provides shared gRPC infrastructure including:
//! - Error conversion between `AppError` and `tonic::Status`
//! - Interceptors for trace context propagation and tenant context extraction
//! - Health check service implementation
//! - Server builder utilities
//! - Retry utilities for service-to-service calls
//! - Notification service client for service-to-service communication
//! - Document service client for service-to-service communication
//! - Payment service client for service-to-service communication
//! - Ledger service client for service-to-service communication

pub mod app_registry;
pub mod document_client;
pub mod error;
pub mod genai_client;
pub mod health;
pub mod interceptors;
pub mod ledger_client;
pub mod notification_client;
pub mod payment_client;
pub mod retry;
pub mod server;

// Include the generated proto code for clients
pub mod proto {
    pub mod auth {
        tonic::include_proto!("micros.auth.v1");
    }
    pub mod billing {
        tonic::include_proto!("micros.billing.v1");
    }
    pub mod document {
        tonic::include_proto!("micros.document.v1");
    }
    pub mod genai {
        tonic::include_proto!("micros.genai.v1");
    }
    pub mod invoicing {
        tonic::include_proto!("micros.invoicing.v1");
    }
    pub mod ledger {
        tonic::include_proto!("micros.ledger.v1");
    }
    pub mod notification {
        tonic::include_proto!("micros.notification.v1");
    }
    pub mod payment {
        tonic::include_proto!("micros.payment.v1");
    }
    pub mod reconciliation {
        tonic::include_proto!("micros.reconciliation.v1");
    }
    pub mod common {
        tonic::include_proto!("micros.common");
        pub const APP_REGISTRY_FILE_DESCRIPTOR_SET: &[u8] =
            tonic::include_file_descriptor_set!("app_registry_descriptor");
    }
}

pub use app_registry::{AppRegistry, AppRegistryServiceImpl};
pub use document_client::{
    DocumentClient, DocumentClientConfig, DocumentProto, DocumentStatusProto,
    ProcessingMetadataProto,
};
pub use error::{GrpcResult, IntoStatus};
pub use genai_client::{
    BANK_STATEMENT_PROMPT, BANK_STATEMENT_SCHEMA_V1, GenaiClient, GenaiClientConfig,
};
pub use health::{HealthComponents, HealthReporter, HealthStatus, create_health_service};
pub use interceptors::{
    APP_ID_KEY, TENANT_ID_KEY, TenantContext, USER_ID_KEY, extract_app_id, extract_app_id_uuid,
    extract_request_id, extract_tenant_context, extract_tenant_id, extract_user_id, inject_app_id,
    inject_tenant_id, inject_user_id,
};
pub use ledger_client::{LedgerClient, LedgerClientConfig, TransactionEntry};
pub use notification_client::{
    BatchNotification, BatchNotificationResult, NotificationChannelProto, NotificationClient,
    NotificationClientConfig, NotificationProto, NotificationStatusProto, PushPlatformProto,
};
pub use payment_client::{PaymentClient, PaymentClientConfig};
pub use retry::{RetryConfig, RetryingClient, is_permanent_failure, is_retryable, retry_grpc_call};
pub use server::{GrpcServerBuilder, create_reflection_service, start_http_health_server};

// Re-export commonly used tonic types
pub use tonic::{Code, Request, Response, Status};
