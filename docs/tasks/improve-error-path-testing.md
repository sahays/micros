# Task: Improve Error Path Testing

**Status**: Open
**Priority**: Medium
**Created**: 2026-01-10
**Related Skill**: rust-development

## Overview

Enhance test coverage for error paths and edge cases across all microservices to ensure proper error handling behavior and improve system reliability.

## Background

Following the recent error handling improvements (all `.await?` calls now include error context via `.await.map_err()`), we need comprehensive tests to verify that:
1. Error contexts are properly logged
2. Error propagation works correctly through the middleware stack
3. Edge cases and failure scenarios are handled gracefully

## Acceptance Criteria

### 1. Error Path Coverage
- [ ] Add tests for all database connection failures in `auth-service` and `document-service`
- [ ] Add tests for Redis connection failures in `auth-service`
- [ ] Add tests for MongoDB query failures (invalid queries, network timeouts)
- [ ] Add tests for storage backend failures in `document-service` (disk full, permission denied, S3 errors)

### 2. Middleware Error Handling
- [ ] Test signature validation failures with various invalid signatures
- [ ] Test rate limiting behavior at limit boundaries (exactly at limit, one over limit)
- [ ] Test bot detection with various user agents
- [ ] Test authentication middleware with expired/invalid tokens

### 3. Handler Error Scenarios
- [ ] Test all handlers with malformed request payloads
- [ ] Test handlers with missing required headers
- [ ] Test concurrent modification scenarios (optimistic locking)
- [ ] Test large file uploads at boundary limits in `document-service`

### 4. Observability Validation
- [ ] Verify error logs include proper context (trace_id, user_id, etc.)
- [ ] Verify metrics are incremented on error paths
- [ ] Test that errors are properly traced through OpenTelemetry

### 5. Edge Cases
- [ ] Test behavior during service startup failures
- [ ] Test graceful shutdown with in-flight requests
- [ ] Test circuit breaker patterns for external dependencies
- [ ] Test retry logic with exponential backoff

## Implementation Guidelines

1. **Use the `Application` pattern**: All integration tests should use the existing `Application` struct to spawn test servers on random ports

2. **Property-based testing**: Consider using `proptest` or `quickcheck` for testing invariants:
   ```rust
   use proptest::prelude::*;

   proptest! {
       #[test]
       fn signature_validation_rejects_invalid_signatures(
           method in "GET|POST|PUT|DELETE",
           path in "/.+",
           timestamp in 0u64..u64::MAX,
       ) {
           // Test that invalid signatures are always rejected
       }
   }
   ```

3. **Concurrent testing**: Use `loom` for testing thread safety in concurrent code:
   ```rust
   #[cfg(loom)]
   #[test]
   fn rate_limiter_is_thread_safe() {
       loom::model(|| {
           // Test concurrent access to rate limiter
       });
   }
   ```

4. **Test isolation**: Each test should be independent and not rely on shared state

5. **Mock external dependencies**: Use mock implementations for MongoDB, Redis, and S3 clients

## Success Metrics

- [ ] Test coverage increases to >80% for error paths
- [ ] All critical error scenarios have dedicated tests
- [ ] CI pipeline includes error path tests
- [ ] Documentation includes examples of error handling testing

## Dependencies

- `proptest` or `quickcheck` for property-based testing
- `loom` for concurrent code testing
- Mock implementations for external services

## Notes

- This task builds on the error handling improvements made in commit [to be added]
- Consider adding mutation testing to verify test effectiveness
- Error path tests should run in CI but may be slower than happy path tests
