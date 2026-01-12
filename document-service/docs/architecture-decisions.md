# Architecture Decisions

## Separation of Concerns: Authorization vs Processing

### Decision: Document-service does NOT enforce user ownership/authorization

**Date:** 2026-01-12
**Status:** Implemented

### Context

In a microservices architecture with a Backend-for-Frontend (BFF) pattern, there are two common approaches to authorization:

1. **Service-level authorization**: Each microservice validates user permissions
2. **BFF-level authorization**: The BFF layer handles all authorization, services trust the caller

### Decision

document-service is a **processing engine**, not an application. It should:

- ✅ Accept document IDs and process them
- ✅ Return processing results
- ✅ Validate business rules (e.g., can't process already-processing documents)
- ❌ NOT validate user ownership or permissions

### Rationale

**1. Single Responsibility**
- document-service's job is to process documents, not manage user access
- Authorization is an application concern, not a processing engine concern

**2. Reusability**
- document-service can be called by multiple clients with different authorization models:
  - secure-frontend (BFF): User session-based authorization
  - Admin tools: Role-based authorization
  - Batch processors: Service account authorization
  - Other microservices: Trust-based authorization

**3. BFF Pattern**
- secure-frontend is the **Backend for Frontend** - it's responsible for:
  - User session management
  - Authentication (verifying who the user is)
  - **Authorization (checking what the user can do)**
  - Request signing (HMAC signatures to prove authenticity)

**4. Trust Boundary**
- document-service validates HMAC signatures (via service-core middleware)
- This proves the request came from a trusted caller (secure-frontend)
- If secure-frontend sent the request, it has already checked authorization
- document-service trusts secure-frontend did its job correctly

### Implementation

**Handler signatures preserve user_id for logging/auditing:**
```rust
pub async fn process_document(
    State(state): State<AppState>,
    _user_id: UserId, // Available for logging/auditing, but authorization is BFF's responsibility
    Path(document_id): Path<String>,
    Json(options): Json<ProcessingOptions>,
) -> Result<impl IntoResponse, AppError>
```

**No ownership checks:**
- Removed: `if document.owner_id != user_id.0 { return Err(Forbidden) }`
- Added: Comments explaining BFF is responsible for authorization

### Consequences

**Positive:**
- ✅ Clean separation of concerns
- ✅ document-service is reusable across different authorization models
- ✅ Simpler document-service code
- ✅ Authorization logic centralized in BFF layer

**Negative:**
- ⚠️ Requires secure-frontend to implement ownership checks
- ⚠️ Direct access to document-service (bypassing BFF) has no authorization
  - Mitigated by: HMAC signature requirement (only trusted callers have secrets)
  - Network security: document-service should not be publicly accessible

### secure-frontend Responsibilities

The BFF (secure-frontend) MUST implement:

1. **Session validation**: Verify user is authenticated
2. **Ownership checks**: Verify user owns the document before calling document-service
   ```rust
   // In secure-frontend
   let document = fetch_document_metadata(doc_id).await?;
   if document.owner_id != session.user_id {
       return Err(Forbidden);
   }

   // Now safe to call document-service
   document_service_client.process_document(doc_id, options).await
   ```
3. **Request signing**: Add HMAC signature headers when calling document-service

### Testing

The test `any_authenticated_caller_can_process_document` verifies that:
- Different user_ids can process the same document
- document-service trusts the caller (doesn't enforce ownership)
- This demonstrates the service-to-service trust model

### References

- BFF Pattern: https://samnewman.io/patterns/architectural/bff/
- Trust Boundaries in Microservices
- document-service/src/handlers/documents.rs (implementation)
- document-service/tests/processing_test.rs (tests)
