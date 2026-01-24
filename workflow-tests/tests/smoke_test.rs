//! Smoke test to verify workflow-tests infrastructure.

mod common;

/// Verify that WorkflowTestContext can be created (requires services running).
#[tokio::test]
async fn workflow_context_connects_to_services() {
    // This test verifies the basic infrastructure works.
    // It will fail if services aren't running, which is expected.
    let ctx = common::setup().await;

    // Basic assertions
    assert!(!ctx.tenant_id.is_nil());
    assert!(!ctx.user_id.is_nil());
}
