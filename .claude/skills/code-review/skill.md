---
name: code-review
description:
  Project-wide code review skill (including git submodules). Reviews code for integration and unit tests, observability,
  improving code quality, detecting code smells, and enforcing secure coding practices
---

**Instructions**

- Observability: Review code for observability: metrics, metering, tracing, and logging standards defined in
  `logging-design` skill. Logs, traces, metrics should show in Grafana.
- Tests: Review the code from integration tests and test run setup (integ-tests, pre-commit script) perspective
- Robust code: Rust code must use `match` or `map_err` at each ? to eliminate panic. Must read `rust-development` Claude
  skill for more
- These services are low-latency, internal-only and used from inter-service calls. There is no rate-limiting,
  authentication, or key requirements

**Actions**

- Find gaps in implementation
- Find code smells and divergence from `rust-development` and `functional-programming` skills
- Find UI issues (not uniformly using established patterns, component macros, etc.)
- It is possible the specs are outdated per recent changes. Update the spec in that case after my approval
- This is a high-performance, secure, and feature-rich production-grade system. TODOs and placeholders are unacceptable
  (only when the upstream development is incomplete)
- Find missing critical path integration and user-journey tests
- Refactor if any .rs file is more than 300-400 (max) lines
- Enter plan mode to summarize the gaps and remediation steps
