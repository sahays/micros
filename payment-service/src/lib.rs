//! Payment service library.
//!
//! This service provides payment processing capabilities via gRPC.
//! HTTP is only used for health/readiness probes.

pub mod config;
pub mod dtos;
pub mod grpc;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;
pub mod startup;
pub mod utils;
