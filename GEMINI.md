# Project Context: Micros Monorepo

## Overview
You are working in `micros`, a monorepo containing multiple microservices. The first active service is `auth-service`.
This project adheres to **strict** engineering standards defined in the `.claude/skills/` directory.

## Core Mandates & Development Standards
**All development must strictly follow the guidelines in `.claude/skills/`.**

### 1. Spec-Driven Development (`.claude/skills/spec-driven-development`)
*   **Workflow:** Epics -> Stories -> Tasks.
*   **Git Issues:** Work is tracked via Git issues.
*   **Commit Protocol:** Every commit must reference an issue (e.g., `Fixes #15`, `Relates to #10`).
*   **Before Coding:** Always check for an existing spec or task. If none exists, ask the user to provide the context or issue number.

### 2. Rust Development (`.claude/skills/rust-development`)
*   **Functional-First:** Prefer immutability, iterators, and functional combinators (`map`, `filter`, `fold`) over imperative loops and mutation.
*   **Security by Design:**
    *   Use **Newtypes** to prevent type confusion (e.g., `Password(String)` vs `Hash(String)`).
    *   Use `secrecy` for sensitive data.
    *   Parse, don't validate: Make invalid states unrepresentable.
*   **Error Handling:** Use `anyhow` for apps, `thiserror` for libs. No `unwrap()` in production.

### 3. REST API (`.claude/skills/rest-api-development`)
*   **Resource-Oriented:** URLs represent resources (nouns).
*   **Semantic HTTP:** Use correct methods (`POST` for non-idempotent creation, `PUT` for replacement, `PATCH` for updates) and status codes (`201 Created`, `422 Unprocessable Entity`).
*   **Standardization:** Snake_case for JSON fields. ISO 8601 for dates.

### 4. Functional Programming (`.claude/skills/functional-programming`)
*   **Pure Functions:** Minimize side effects. Push I/O to the boundaries (handlers/services).
*   **Composition:** Build complex logic from small, reusable functions.

## Service: `auth-service`
A self-contained authentication microservice.

### Current Status (from Git Log)
*   **Implemented Features:**
    *   User Registration & Email Verification (Tasks #11, #12, #13, #14)
    *   JWT Authentication (Access + Refresh Tokens) (Task #16)
    *   Login/Logout with Rate Limiting (Tasks #15, #17, #18)
    *   MongoDB Integration (User & Token schemas) (Task #10)
    *   Argon2 Password Hashing (Task #11)

### Key Technologies
*   **Framework:** Axum (Async Rust)
*   **DB:** MongoDB
*   **Auth:** `jsonwebtoken`, `argon2`
*   **Email:** `lettre` (SMTP)
*   **Validation:** `validator`

## Usage Instructions
1.  **Read the Skills:** When in doubt about style or pattern, consult the relevant file in `.claude/skills/`.
2.  **Check the Spec:** Verify requirements against the implied or provided spec/issue.
3.  **Implement functionally:** Avoid `mut` where possible. Use `impl From` and strict typing.
4.  **Test:** Ensure unit tests cover the logic.

## Environment
*   **Config:** `.env` file (loaded via `dotenvy`).
*   **Run:** `cargo run`
*   **Test:** `cargo test`