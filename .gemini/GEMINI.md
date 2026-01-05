# Project Context: Micros Monorepo

## Overview
You are working in micros, a monorepo containing multiple microservices. The first active service is auth-service.
This project adheres to strict engineering standards defined in the skills/ directory.

## Core Mandates & Development Standards
All development must strictly follow the guidelines in `skills/`.

### 1. Spec-Driven Development (skills/spec-driven-development)
*   **Workflow:** Epics -> Stories -> Tasks.
*   **Git Issues:** Work is tracked via Git issues.
*   **Commit Protocol:** Every commit must reference an issue (e.g., Fixes #15, Relates to #10).
*   **Before Coding:** Always check for an existing spec or task. If none exists, ask the user to provide the context or issue number.
*   **Task Completion Protocol:**
    *   When completing a Task or Story, you MUST update the GitHub issue description.
    *   Find all relevant checkboxes for Acceptance Criteria and sub-tasks (e.g., [ ] Item) and mark them as complete (e.g., [x] Item).
    *   Use `gh issue view <id>` to get the body, modify it, and `gh issue edit <id> --body "..."` to update it.
    *   This applies to both the specific Task issue and the parent Story issue.

### 2. Rust Development (skills/rust-development)
*   **Functional-First:** Prefer immutability, iterators, and functional combinators (map, filter, fold) over imperative loops and mutation.
*   **Security by Design:**
    *   Use **Newtypes** to prevent type confusion (e.g., Password(String) vs Hash(String)).
    *   Use `secrecy` for sensitive data.
    *   Parse, don't validate: Make invalid states unrepresentable.
*   **Error Handling:** Use `anyhow` for apps, `thiserror` for libs. No `unwrap()` in production.

### 3. REST API (skills/rest-api-development)
*   **Resource-Oriented:** URLs represent resources (nouns).
*   **Semantic HTTP:** Use correct methods (POST for non-idempotent creation, PUT for replacement, PATCH for updates) and status codes (201 Created, 422 Unprocessable Entity).
*   **Standardization:** Snake_case for JSON fields. ISO 8601 for dates.

### 4. Functional Programming (skills/functional-programming)
*   **Pure Functions:** Minimize side effects. Push I/O to the boundaries (handlers/services).
*   **Composition:** Build complex logic from small, reusable functions.

## Service: auth-service
A self-contained authentication microservice.

### Current Status (from Git Log)
*   **Implemented Features:**
    *   User Registration & Email Verification (Tasks #11, #12, #13, #14)
    *   JWT Authentication (Access + Refresh Tokens) (Task #16)
    *   Login/Logout with Rate Limiting (Tasks #15, #17, #18)
    *   MongoDB Integration (User & Token schemas) (Task #10)
    *   Argon2 Password Hashing (Task #11)
    *   Password Reset Flow (Story #6)
    *   JWT Management & Refresh (Story #5)
    *   Token Revocation & Distributed Auth (Story #40)
    *   Social Auth (Story #4)
    *   User Profile Management (Story #7)

### Key Technologies
*   **Framework:** Axum (Async Rust)
*   **DB:** MongoDB
*   **Auth:** jsonwebtoken, argon2
*   **Email:** lettre (SMTP)
*   **Validation:** validator

## Usage Instructions
1.  **Read the Skills:** When in doubt about style or pattern, consult the relevant file in skills/.
2.  **Check the Spec:** Verify requirements against the implied or provided spec/issue.
3.  **Implement functionally:** Avoid mut where possible. Use impl From and strict typing.
4.  **Test:** Ensure unit tests cover the logic.

## Environment
*   **Config:** .env file (loaded via dotenvy).
*   **Run:** cargo run
*   **Test:** cargo test

## Gemini Added Memories
- always summarize what you are going to do before making any code changes, once done summarize your actions. Be specific
- always use revelant instructions from skills folder for implementation. REST API, functional programming, rust development are the key ones. spec-driven-development is the way you organize and complete your work
- read all the tasks defined in the story by listing git issues so that you don't guess anything
- Ensure that you write only critical path tests
- you need to update epic, story, and tasks
- ensure that integration tests cover all acceptance criteria for a task
- read skills/logging-design and skills/git-pre-commit for strictly implementing logging and pre-commit guidelines
