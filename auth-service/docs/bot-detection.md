# Bot Detection & Heuristic Analysis

## Overview
The `auth-service` includes a middleware layer designed to detect and block automated bot traffic. This system uses heuristic analysis of request metadata, primarily the `User-Agent` header and standard browser headers, to identify suspicious actors.

## Implementation Details

### Middleware
File: `src/middleware/bot_detection.rs`

The middleware performs the following checks:

1.  **Known Bot Detection**:
    -   Uses the `isbot` crate to identify known crawlers, spiders, and bots.
    -   **Action**: If a known bot is detected, the "Bot Score" increases by 100.

2.  **Heuristic Analysis**:
    -   **Empty User-Agent**: Requests with empty UA strings are flagged (Score +50).
    -   **Browser Impersonation**: If a User-Agent claims to be a standard browser (starting with "Mozilla/"), the middleware checks for the presence of standard headers:
        -   `Accept`
        -   `Accept-Language`
        -   `Accept-Encoding`
    -   **Action**: Missing these headers suggests a script or a poorly configured bot, increasing the Bot Score.

3.  **Scoring & Blocking**:
    -   **Threshold**: A score of **100** or higher triggers a block.
    -   **Action**: Returns `403 Forbidden` and logs the event with a warning.
    -   **Exclusions**: `OPTIONS` requests (CORS preflight) are exempt from these checks.

### Configuration
Currently, the blocking threshold and heuristics are hardcoded in the middleware. Future iterations may move these to the `Config` struct for dynamic adjustment.

## JA3 TLS Fingerprinting Investigation

### Findings
JA3 is a method for creating SSL/TLS client fingerprints that are easy to produce on any platform and can be easily shared for threat intelligence.

To implement JA3 in `auth-service`:
1.  **TLS Termination**: The service needs access to the raw TLS ClientHello packet.
2.  **Architecture Constraint**: In a typical microservices deployment (Docker/Kubernetes), TLS is terminated at the Ingress Controller, Load Balancer, or Reverse Proxy (e.g., Nginx, Traefik, AWS ALB). The application receives plain HTTP traffic.
3.  **Conclusion**: Implementing JA3 directly in the Rust application is not feasible without moving TLS termination to the application itself, which complicates certificate management and performance scaling.

### Recommendation
For stricter enforcement using JA3:
1.  **Edge Implementation**: Enable JA3 fingerprinting at the Ingress/Load Balancer level (e.g., using Cloudflare, AWS WAF, or Nginx modules).
2.  **Header Forwarding**: Configure the edge to calculate the fingerprint and forward it to `auth-service` as an HTTP header (e.g., `X-JA3-Fingerprint`).
3.  **Middleware Update**: Update the `bot_detection` middleware to validate this header against a blacklist of known malicious fingerprints.
