---
name: rest-api-security
description:
  Secure REST APIs against common attacks with authentication, authorization, rate limiting, and bot prevention. Use
  when implementing API security controls for production services.
---

# REST API Security

## Transport Security

**HTTPS only**: Enforce TLS 1.2+ for all endpoints. Redirect HTTP to HTTPS.

**HSTS headers**: Force browsers to use HTTPS. Set max-age to 1+ year.

**Certificate validation**: Use valid certificates from trusted CAs. Enable certificate pinning for mobile.

**No sensitive data in URLs**: Tokens, passwords, PII belong in headers or encrypted body.

## Authentication

**OAuth 2.0**: Industry standard for delegated access. Use authorization code flow with PKCE.

**JWT tokens**: Stateless authentication. Sign with RS256 (asymmetric). Avoid HS256 in distributed systems.

**Token placement**: Authorization header with Bearer scheme: `Authorization: Bearer <token>`

**API keys**: For machine-to-machine only. Rotate regularly. Never embed in client apps.

**Short-lived tokens**: Access tokens expire in 15-60 minutes. Use refresh tokens for renewal.

**Token revocation**: Implement token blacklist or use short expiration with frequent rotation.

## Authorization

**Principle of least privilege**: Grant minimum permissions needed.

**Role-based access control (RBAC)**: Assign permissions through roles.

**Attribute-based access control (ABAC)**: Fine-grained control using attributes and policies.

**Resource ownership**: Users access only their own resources unless explicitly granted.

**Scope validation**: Verify token scopes match required permissions for endpoint.

**Defense in depth**: Validate authorization at multiple layers - gateway, service, database.

## Rate Limiting

**Per-client limits**: Track by API key, user ID, or IP address.

**Multiple tiers**: Different limits for authenticated vs unauthenticated, free vs paid.

**Algorithms**: Token bucket for burst handling, sliding window for accuracy.

**Response headers**: Include X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset.

**429 status code**: Return with Retry-After header specifying wait time in seconds.

**Distributed rate limiting**: Use Redis for shared state across instances.

**Endpoint-specific limits**: Stricter limits on expensive operations (search, reports).

## Bot Detection and Prevention

**Challenge-response**: CAPTCHA for suspicious patterns. Use invisible reCAPTCHA v3 for UX.

**Fingerprinting**: Device and browser fingerprinting to identify bots. Track patterns over time.

**Behavioral analysis**: Detect non-human patterns - request timing, navigation flow, user-agent.

**User-Agent validation**: Block known bot signatures. Require valid modern browsers.

**Honeypot fields**: Hidden form fields that bots fill but humans don't.

**Rate of change**: Flag rapid state changes impossible for humans.

**Proof of work**: Require computational challenge for high-risk operations.

**Bot scoring**: Assign risk scores based on multiple signals. Block or challenge high scores.

## Input Validation

**Whitelist approach**: Accept only known-good patterns. Reject everything else.

**Type validation**: Enforce data types. Strings are strings, numbers are numbers.

**Length limits**: Enforce maximum lengths on all inputs. Prevent buffer overflow and DoS.

**Format validation**: Use regex for emails, URLs, phone numbers. Be strict.

**Content validation**: Scan for SQL injection, XSS, command injection patterns.

**Reject unexpected fields**: Strict mode rejects unknown properties in JSON.

**Encoding validation**: Verify UTF-8. Reject invalid byte sequences.

## Output Encoding

**Context-aware encoding**: HTML encode for HTML, JavaScript encode for JS, URL encode for URLs.

**JSON encoding**: Properly escape strings in JSON responses. Prevent XSS.

**Content-Type headers**: Set correct Content-Type. Prevent MIME sniffing with X-Content-Type-Options: nosniff.

**No user content in responses without sanitization**: Always escape user-generated content.

## CORS Configuration

**Specific origins**: List allowed origins explicitly. Never use `*` in production with credentials.

**Credentials handling**: Set Access-Control-Allow-Credentials: true only for trusted origins.

**Allowed methods**: Restrict to needed methods only. Avoid allowing all methods.

**Allowed headers**: Whitelist specific headers. Don't allow all.

**Preflight caching**: Set max age for OPTIONS preflight to reduce overhead.

## Request Signing

**HMAC signatures**: Sign requests with shared secret. Verify signature server-side.

**Timestamp validation**: Include timestamp in signature. Reject old requests (>5 minutes).

**Nonce for replay prevention**: Require unique nonce. Store recently used nonces.

**AWS Signature v4 pattern**: Follow AWS signing process for robust authentication.

## Security Headers

**X-Content-Type-Options**: nosniff - Prevent MIME sniffing

**X-Frame-Options**: DENY - Prevent clickjacking

**Content-Security-Policy**: Restrict resource loading

**X-XSS-Protection**: 1; mode=block - Enable browser XSS filter

**Referrer-Policy**: strict-origin-when-cross-origin - Control referer information

**Permissions-Policy**: Restrict browser features

## API Keys Management

**Prefix for identification**: Start keys with identifier (e.g., `pk_live_`, `sk_test_`)

**Hash storage**: Store bcrypt hashed keys, not plaintext. Like passwords.

**Key rotation**: Support rotation without downtime. Allow multiple active keys.

**Scoped keys**: Limit keys to specific operations or resources.

**Environment separation**: Different keys for dev, staging, production.

**Expiration**: Set expiration dates. Force renewal.

## Secrets Management

**Never in code**: No hardcoded credentials, API keys, tokens in source code.

**Environment variables**: Load from environment or secrets manager at runtime.

**Secrets manager**: Use Vault, AWS Secrets Manager, or cloud provider solution.

**Rotation**: Rotate secrets regularly. Automate when possible.

**Access control**: Limit who can read/write secrets. Audit access.

## Common Attacks Prevention

**SQL injection**: Use parameterized queries. Never concatenate user input into SQL.

**XSS**: Encode output. Set CSP headers. Validate and sanitize input.

**CSRF**: Use CSRF tokens for state-changing operations. Verify Origin/Referer headers.

**Path traversal**: Validate file paths. Reject `..` sequences. Use allow-list of paths.

**XXE**: Disable external entity processing in XML parsers.

**Server-side request forgery (SSRF)**: Validate and restrict outbound requests. Block internal IPs.

**Timing attacks**: Use constant-time comparisons for sensitive data (tokens, passwords).

## Monitoring and Alerting

**Failed auth attempts**: Alert on repeated failures from same IP or account.

**Rate limit violations**: Track and alert on patterns of rate limit hits.

**Unusual patterns**: Detect anomalies - geographic, timing, volume.

**Error rate spikes**: Monitor 401, 403, 429 response rates.

**Token usage**: Track token age, frequency, geographic distribution.

## Audit Logging

**Security events**: Log authentication, authorization failures, permission changes.

**What to log**: User ID, IP, timestamp, action, resource, result.

**What not to log**: Passwords, tokens, credit cards, PII in plaintext.

**Tamper-proof logs**: Write-only access. Consider immutable storage.

**Retention**: Keep security logs for minimum 90 days, often 1+ year for compliance.

## Best Practices

**Do**:

- Enforce HTTPS everywhere
- Validate all inputs with whitelist approach
- Use short-lived tokens with proper scopes
- Implement multiple rate limiting tiers
- Monitor and alert on suspicious patterns
- Use request signing for critical APIs
- Store secrets in secrets manager
- Log security events for audit

**Avoid**:

- Exposing stack traces or internal errors
- Trusting client-side validation
- Using API keys in frontend code
- Allowing unlimited rate for any client
- Skipping authorization checks
- Revealing user enumeration (email exists, etc.)
- Using sequential or predictable IDs for sensitive resources
- Rolling your own crypto
