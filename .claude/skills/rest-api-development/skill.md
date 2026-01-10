---
name: rest-api-development
description:
  Design and develop standards-based, semantic REST APIs following HTTP specifications and best practices. Use when
  building RESTful APIs that are discoverable, consistent, and follow REST architectural constraints.
---

- Resource-Oriented Design
  - APIs model resources, not actions
  - Resources are nouns, not verbs
  - Use plural nouns for collections
  - Resource hierarchy: reflect relationships in URI structure, limit nesting to 2-3 levels maximum
  - Granularity: design resources at appropriate level, balance between too fine-grained (chattiness) and too coarse (inflexibility)

- Semantic HTTP Methods
  - GET: retrieve resource, safe and idempotent, no request body, cacheable
  - POST: create new resource or non-idempotent operations, Location header in 201 response
  - PUT: replace entire resource, idempotent, client specifies resource URI
  - PATCH: partial update, send only changed fields, use JSON Patch or Merge Patch standards
  - DELETE: remove resource, idempotent, 204 No Content or 200 with response body
  - HEAD: identical to GET but no response body, check existence or metadata
  - OPTIONS: describe communication options, return allowed methods in Allow header

- Semantic HTTP Status Codes
  - 2xx Success: 200 OK (successful GET/PUT/PATCH/DELETE with body), 201 Created (POST creating resource), 204 No Content (successful request with no body)
  - 3xx Redirection: 301 Moved Permanently, 304 Not Modified (cached version is current)
  - 4xx Client Errors: 400 Bad Request (malformed syntax), 401 Unauthorized (auth required/failed), 403 Forbidden (authenticated but not authorized), 404 Not Found, 409 Conflict, 422 Unprocessable Entity (semantic errors), 429 Too Many Requests
  - 5xx Server Errors: 500 Internal Server Error, 503 Service Unavailable (temporary overload/maintenance)

- URI Design
  - Lowercase with hyphens: /user-accounts not /UserAccounts or /user_accounts
  - No trailing slashes: /users not /users/
  - Plural resource names: /users/123 not /user/123
  - Relationships: /users/123/orders for nested resources
  - Avoid verbs in URIs: use HTTP methods, not /getUser/123
  - Query parameters for filtering: /users?role=admin&status=active
  - Avoid deep nesting: if beyond 2-3 levels, consider top-level resource with filters

- Request and Response Bodies
  - JSON as default: use application/json content type, support content negotiation via Accept header
  - Consistent naming: choose camelCase or snake_case and use throughout (snake_case aligns with JSON:API)
  - ISO 8601 for dates: 2025-12-27T10:30:00Z with timezone
  - Envelope only when necessary: return resource directly, not wrapped, unless pagination or metadata required
  - Null vs omission: decide convention, generally omit null values for cleaner responses

- Error Responses
  - Include: HTTP status code, machine-readable error code, human-readable message, details array for validation errors, request ID for debugging, documentation link when helpful
  - Validation errors: specify which fields failed and why, return 422 not 400

- Pagination
  - Cursor-based for large datasets: more stable than offset when data changes
  - Offset-based for small datasets: simpler, allows jumping to pages
  - Include: total count (when feasible), links to next/previous pages, current page metadata
  - Use Link header: RFC 5988 standard for pagination links

- Versioning
  - URI versioning: /v1/users (most visible and explicit)
  - Header versioning: Accept: application/vnd.api.v1+json (cleaner URIs)
  - Choose one strategy: be consistent, don't mix
  - Version only when breaking changes: additive changes don't require new version

- Filtering, Sorting, Searching
  - Filtering: query parameters for field values (/users?status=active&role=admin)
  - Sorting: ?sort=created_at or ?sort=-created_at (minus for descending)
  - Searching: ?q=query for full-text search across fields
  - Field selection: ?fields=id,name,email to reduce payload

- Idempotency
  - Natural idempotency: GET, PUT, DELETE are inherently idempotent
  - POST idempotency: use Idempotency-Key header for safe retries, store key with 24-hour expiration
  - Critical for payments and mutations: prevents duplicate operations from retries

- HATEOAS (Hypermedia)
  - Include links to related resources and available actions
  - Makes API discoverable and self-documenting
  - Level 3 REST maturity: hypermedia controls guide client interactions
  - Consider: HAL, JSON:API, or custom link format for consistency

- Caching
  - Use ETags: for conditional requests and cache validation
  - Cache-Control headers: specify caching behavior explicitly
  - GET and HEAD are cacheable: PUT, POST, PATCH, DELETE are not
  - Vary header: indicate which request headers affect response

- Security
  - See rest-api-security skill for comprehensive coverage of authentication, authorization, rate limiting, bot prevention, and attack mitigation
  - Essential: HTTPS only, input validation, proper error handling without exposing internals

- API Documentation
  - OpenAPI/Swagger: standard machine-readable format
  - Document: all endpoints, parameters, response codes, request/response schemas, authentication
  - Examples: provide request/response examples for common scenarios
  - Changelog: track API changes and deprecations

- Consistency
  - Naming conventions: consistent across all resources
  - Response structure: same format for similar operations
  - Error handling: uniform error response format
  - Behavior: similar resources behave similarly

- Standards Compliance
  - Follow RFC specifications: HTTP/1.1 (RFC 7231), URI (RFC 3986), JSON (RFC 8259)
  - Consider standard formats: JSON:API, HAL, or Collection+JSON for consistency
  - HTTP header conventions: use standard headers (Accept, Content-Type, Authorization)
