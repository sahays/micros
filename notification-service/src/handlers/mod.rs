//! HTTP handlers for notification-service.
//!
//! Note: Business logic has been moved to gRPC. This module only contains
//! health check handlers for infrastructure probes.

pub mod health;

pub use health::health_check;
