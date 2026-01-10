---
name: rest-api-security
description:
  Secure REST APIs against common attacks with authentication, authorization, rate limiting, and bot prevention. Use
  when implementing API security controls for production services.
---

- Transport Security
  - Enforce HTTPS with TLS 1.2+ for all endpoints, redirect HTTP to HTTPS
  - HSTS headers: force browsers to use HTTPS, set max-age to 1+ year
  - Use valid certificates from trusted CAs
  - Enable certificate pinning for mobile apps
  - No sensitive data in URLs (use headers or encrypted body)

- Authentication
  - OAuth 2.0: use authorization code flow with PKCE
  - JWT tokens: sign with RS256 (asymmetric), avoid HS256 in distributed systems
  - Token placement: Authorization header with Bearer scheme
  - API keys: machine-to-machine only, rotate regularly, never embed in client apps
  - Short-lived tokens: access tokens expire in 15-60 minutes, use refresh tokens for renewal
  - Token revocation: implement blacklist or use short expiration with frequent rotation

- Authorization
  - Principle of least privilege: grant minimum permissions needed
  - RBAC: assign permissions through roles
  - ABAC: fine-grained control using attributes and policies
  - Resource ownership: users access only their resources unless explicitly granted
  - Scope validation: verify token scopes match required endpoint permissions
  - Defense in depth: validate authorization at multiple layers (gateway, service, database)

- Rate Limiting
  - Per-client limits: track by API key, user ID, or IP address
  - Multiple tiers: different limits for authenticated vs unauthenticated, free vs paid
  - Algorithms: token bucket for burst handling, sliding window for accuracy
  - Response headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset
  - 429 status code: return with Retry-After header specifying wait time in seconds
  - Distributed rate limiting: use Redis for shared state across instances
  - Endpoint-specific limits: stricter limits on expensive operations (search, reports)

- Bot Detection and Prevention
  - Challenge-response: CAPTCHA for suspicious patterns, use invisible reCAPTCHA v3
  - Fingerprinting: device and browser fingerprinting, track patterns over time
  - Behavioral analysis: detect non-human patterns (request timing, navigation flow, user-agent)
  - User-Agent validation: block known bot signatures, require valid modern browsers
  - Honeypot fields: hidden form fields that bots fill but humans don't
  - Rate of change: flag rapid state changes impossible for humans
  - Proof of work: require computational challenge for high-risk operations
  - Bot scoring: assign risk scores based on multiple signals, block or challenge high scores

- Input Validation
  - Whitelist approach: accept only known-good patterns, reject everything else
  - Type validation: enforce data types strictly
  - Length limits: enforce maximum lengths on all inputs, prevent buffer overflow and DoS
  - Format validation: use regex for emails, URLs, phone numbers
  - Content validation: scan for SQL injection, XSS, command injection patterns
  - Reject unexpected fields: strict mode rejects unknown properties in JSON
  - Encoding validation: verify UTF-8, reject invalid byte sequences

- Output Encoding
  - Context-aware encoding: HTML encode for HTML, JavaScript encode for JS, URL encode for URLs
  - JSON encoding: properly escape strings in JSON responses, prevent XSS
  - Content-Type headers: set correct Content-Type, use X-Content-Type-Options: nosniff
  - Always escape user-generated content in responses

- CORS Configuration
  - Specific origins: list allowed origins explicitly, never use * with credentials
  - Credentials handling: set Access-Control-Allow-Credentials: true only for trusted origins
  - Allowed methods: restrict to needed methods only
  - Allowed headers: whitelist specific headers
  - Preflight caching: set max age for OPTIONS preflight to reduce overhead

- Request Signing
  - HMAC signatures: sign requests with shared secret, verify server-side
  - Timestamp validation: include timestamp in signature, reject old requests (>5 minutes)
  - Nonce for replay prevention: require unique nonce, store recently used nonces
  - Follow AWS Signature v4 pattern for robust authentication

- Security Headers
  - X-Content-Type-Options: nosniff (prevent MIME sniffing)
  - X-Frame-Options: DENY (prevent clickjacking)
  - Content-Security-Policy: restrict resource loading
  - X-XSS-Protection: 1; mode=block (enable browser XSS filter)
  - Referrer-Policy: strict-origin-when-cross-origin
  - Permissions-Policy: restrict browser features

- API Keys Management
  - Prefix for identification: pk_live_, sk_test_, etc.
  - Hash storage: store bcrypt hashed keys, not plaintext
  - Key rotation: support rotation without downtime, allow multiple active keys
  - Scoped keys: limit keys to specific operations or resources
  - Environment separation: different keys for dev, staging, production
  - Expiration: set expiration dates, force renewal

- Secrets Management
  - Never hardcode credentials, API keys, tokens in source code
  - Environment variables: load from environment or secrets manager at runtime
  - Use Vault, AWS Secrets Manager, or cloud provider solution
  - Rotate secrets regularly, automate when possible
  - Limit who can read/write secrets, audit access

- Common Attacks Prevention
  - SQL injection: use parameterized queries, never concatenate user input into SQL
  - XSS: encode output, set CSP headers, validate and sanitize input
  - CSRF: use CSRF tokens for state-changing operations, verify Origin/Referer headers
  - Path traversal: validate file paths, reject .. sequences, use allow-list
  - XXE: disable external entity processing in XML parsers
  - SSRF: validate and restrict outbound requests, block internal IPs
  - Timing attacks: use constant-time comparisons for sensitive data

- Monitoring and Alerting
  - Alert on repeated failed auth attempts from same IP or account
  - Track and alert on patterns of rate limit hits
  - Detect anomalies: geographic, timing, volume
  - Monitor 401, 403, 429 response rates
  - Track token age, frequency, geographic distribution

- Audit Logging
  - Log authentication, authorization failures, permission changes
  - Log: user ID, IP, timestamp, action, resource, result
  - Never log: passwords, tokens, credit cards, PII in plaintext
  - Write-only access, consider immutable storage
  - Keep security logs minimum 90 days, often 1+ year for compliance

- Best Practices
  - Enforce HTTPS everywhere
  - Validate all inputs with whitelist approach
  - Use short-lived tokens with proper scopes
  - Implement multiple rate limiting tiers
  - Monitor and alert on suspicious patterns
  - Use request signing for critical APIs
  - Store secrets in secrets manager
  - Log security events for audit
  - Never expose stack traces or internal errors
  - Never trust client-side validation
  - Never use API keys in frontend code
  - Never skip authorization checks
  - Avoid revealing user enumeration
  - Avoid sequential or predictable IDs for sensitive resources
  - Never roll your own crypto
