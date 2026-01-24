//! Common test utilities for workflow integration tests.

use std::time::Duration;
use workflow_tests::{wait_for_services, WorkflowTestContext};

/// Default timeout for waiting on services.
pub const SERVICE_TIMEOUT: Duration = Duration::from_secs(60);

/// Create a new workflow test context, ensuring services are healthy.
///
/// This is the main entry point for workflow tests.
pub async fn setup() -> WorkflowTestContext {
    // Wait for all services to be healthy
    wait_for_services(SERVICE_TIMEOUT)
        .await
        .expect("Services not healthy - run ./scripts/dev-up.sh first");

    // Create context connected to all services
    WorkflowTestContext::new()
        .await
        .expect("Failed to create workflow test context")
}

/// Helper to check if workflow tests should be skipped.
///
/// Returns true if SKIP_WORKFLOW_TESTS env var is set.
pub fn should_skip() -> bool {
    std::env::var("SKIP_WORKFLOW_TESTS").is_ok()
}

/// Macro to skip workflow tests if services aren't running.
#[macro_export]
macro_rules! skip_if_no_services {
    () => {
        if common::should_skip() {
            eprintln!("Skipping workflow test (SKIP_WORKFLOW_TESTS is set)");
            return;
        }
    };
}
