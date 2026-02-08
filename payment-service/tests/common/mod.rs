//! Common test utilities for payment-service integration tests.
//!
//! Embedded TestApp with wiremock for Razorpay mocking.

pub mod embedded;

#[allow(dead_code)]
pub const TEST_APP_ID: &str = "test-app";
#[allow(dead_code)]
pub const TEST_ORG_ID: &str = "test-org";
#[allow(dead_code)]
pub const TEST_USER_ID: &str = "test-user";
