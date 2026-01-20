//! gRPC utilities for micros microservices.
//!
//! This module provides shared gRPC infrastructure including:
//! - Error conversion between `AppError` and `tonic::Status`
//! - Interceptors for trace context propagation
//! - Health check service implementation
//! - Server builder utilities
//! - Auth service client for service-to-service communication
//! - Notification service client for service-to-service communication
//! - Document service client for service-to-service communication
//! - Payment service client for service-to-service communication

pub mod auth_client;
pub mod document_client;
pub mod error;
pub mod health;
pub mod interceptors;
pub mod notification_client;
pub mod payment_client;
pub mod server;

// Include the generated proto code for clients
pub mod proto {
    pub mod auth {
        tonic::include_proto!("micros.auth.v1");
    }
    pub mod document {
        tonic::include_proto!("micros.document.v1");
    }
    pub mod notification {
        tonic::include_proto!("micros.notification.v1");
    }
    pub mod payment {
        tonic::include_proto!("micros.payment.v1");
    }
}

pub use auth_client::{AuthClient, AuthClientConfig};
pub use document_client::{DocumentClient, DocumentClientConfig};
pub use error::{GrpcResult, IntoStatus};
pub use health::{HealthComponents, HealthReporter, HealthStatus, create_health_service};
pub use interceptors::{
    extract_request_id, extract_traceparent, inject_trace_context,
    inject_trace_context_with_request_id, trace_context_interceptor,
};
pub use notification_client::{NotificationClient, NotificationClientConfig};
pub use payment_client::{PaymentClient, PaymentClientConfig};
pub use server::{GrpcServerBuilder, create_reflection_service, start_http_health_server};

// Re-export commonly used tonic types
pub use tonic::{Code, Request, Response, Status};
