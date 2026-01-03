---
name: rest-api-development
description:
  Design and develop standards-based, semantic REST APIs following HTTP specifications and best practices. Use when
  building RESTful APIs that are discoverable, consistent, and follow REST architectural constraints.
---

# REST API Development

## Resource-Oriented Design

APIs model resources, not actions. Resources are nouns, not verbs. Use plural nouns for collections.

**Resource hierarchy**: Reflect relationships in URI structure. Limit nesting to 2-3 levels maximum.

**Granularity**: Design resources at appropriate level. Too fine-grained increases chattiness, too coarse reduces
flexibility.

## Semantic HTTP Methods

Use HTTP methods according to their defined semantics:

**GET**: Retrieve resource. Safe and idempotent. No request body. Cacheable.

**POST**: Create new resource or non-idempotent operations. Location header in 201 response.

**PUT**: Replace entire resource. Idempotent. Client specifies resource URI.

**PATCH**: Partial update. Send only changed fields. Use JSON Patch or Merge Patch standards.

**DELETE**: Remove resource. Idempotent. 204 No Content or 200 with response body.

**HEAD**: Identical to GET but no response body. Check existence or metadata.

**OPTIONS**: Describe communication options. Return allowed methods in Allow header.

## Semantic HTTP Status Codes

Use status codes that accurately convey response meaning:

**2xx Success**:

- 200 OK: Successful GET, PUT, PATCH, or DELETE with body
- 201 Created: Successful POST creating resource
- 204 No Content: Successful request with no response body

**3xx Redirection**:

- 301 Moved Permanently: Resource permanently relocated
- 304 Not Modified: Cached version is current

**4xx Client Errors**:

- 400 Bad Request: Malformed request syntax
- 401 Unauthorized: Authentication required or failed
- 403 Forbidden: Authenticated but not authorized
- 404 Not Found: Resource doesn't exist
- 409 Conflict: Request conflicts with current state
- 422 Unprocessable Entity: Valid syntax but semantic errors
- 429 Too Many Requests: Rate limit exceeded

**5xx Server Errors**:

- 500 Internal Server Error: Unexpected server condition
- 503 Service Unavailable: Temporary overload or maintenance

## URI Design

**Lowercase with hyphens**: `/user-accounts`, not `/UserAccounts` or `/user_accounts`

**No trailing slashes**: `/users`, not `/users/`

**Plural resource names**: `/users/123`, not `/user/123`

**Relationships**: `/users/123/orders` for nested resources

**Avoid verbs in URIs**: Use HTTP methods instead. `/users/123`, not `/getUser/123`

**Query parameters for filtering**: `/users?role=admin&status=active`

**Avoid deep nesting**: If beyond 2-3 levels, consider top-level resource with filters

## Request and Response Bodies

**JSON as default**: Use `application/json` content type. Support content negotiation via Accept header.

**Consistent naming**: Choose camelCase or snake_case and use throughout. snake_case aligns with JSON:API standard.

**ISO 8601 for dates**: `2025-12-27T10:30:00Z` with timezone

**Envelope only when necessary**: Return resource directly, not wrapped in envelope, unless pagination or metadata
required.

**Null vs omission**: Decide convention. Generally omit null values for cleaner responses.

## Error Responses

Use consistent error format across all endpoints:

**Include**:

- HTTP status code
- Machine-readable error code
- Human-readable message
- Details array for validation errors
- Request ID for debugging
- Documentation link when helpful

**Validation errors**: Specify which fields failed and why. Return 422, not 400.

## Pagination

**Cursor-based for large datasets**: More stable than offset when data changes

**Offset-based for small datasets**: Simpler, allows jumping to pages

**Include in response**:

- Total count (when feasible)
- Links to next/previous pages
- Current page metadata

**Use Link header**: RFC 5988 standard for pagination links

## Versioning

**URI versioning**: `/v1/users` - Most visible and explicit

**Header versioning**: `Accept: application/vnd.api.v1+json` - Cleaner URIs

**Choose one strategy**: Be consistent. Don't mix approaches.

**Version only when breaking changes**: Additive changes don't require new version.

## Filtering, Sorting, Searching

**Filtering**: Query parameters for field values: `/users?status=active&role=admin`

**Sorting**: `?sort=created_at` or `?sort=-created_at` (minus for descending)

**Searching**: `?q=query` for full-text search across fields

**Field selection**: `?fields=id,name,email` to reduce payload

## Idempotency

**Natural idempotency**: GET, PUT, DELETE are inherently idempotent

**POST idempotency**: Use Idempotency-Key header for safe retries. Store key with 24-hour expiration.

**Critical for payments and mutations**: Prevents duplicate operations from retries.

## HATEOAS (Hypermedia)

Include links to related resources and available actions. Makes API discoverable and self-documenting.

**Level 3 REST maturity**: Hypermedia controls guide client interactions

**Consider**: HAL, JSON:API, or custom link format for consistency

## Caching

**Use ETags**: For conditional requests and cache validation

**Cache-Control headers**: Specify caching behavior explicitly

**GET and HEAD are cacheable**: PUT, POST, PATCH, DELETE are not

**Vary header**: Indicate which request headers affect response

## Security

**See rest-api-security skill**: Comprehensive coverage of authentication, authorization, rate limiting, bot prevention,
and attack mitigation.

**Essential**: HTTPS only, input validation, proper error handling without exposing internals.

## API Documentation

**OpenAPI/Swagger**: Standard machine-readable format

**Document**: All endpoints, parameters, response codes, request/response schemas, authentication

**Examples**: Provide request/response examples for common scenarios

**Changelog**: Track API changes and deprecations

## Consistency

**Naming conventions**: Consistent across all resources

**Response structure**: Same format for similar operations

**Error handling**: Uniform error response format

**Behavior**: Similar resources behave similarly

## Standards Compliance

**Follow RFC specifications**: HTTP/1.1 (RFC 7231), URI (RFC 3986), JSON (RFC 8259)

**Consider standard formats**: JSON:API, HAL, or Collection+JSON for consistency

**HTTP header conventions**: Use standard headers (Accept, Content-Type, Authorization)
