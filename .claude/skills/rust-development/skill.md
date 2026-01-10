---
name: rust-development
description:
  Develop secure, thread-safe Rust using functional patterns. Use when writing Rust code requiring memory safety,
  concurrency, zero-cost abstractions, or systems programming with security constraints.
---

- Functional-First Approach

  - Prefer immutability and functional combinators over imperative patterns
  - Use iterators, map/filter/fold instead of mutable loops
  - Minimize use of `mut`
  - Reduce `.clone()` by using Rc, Arc, Cow, and other patterns

- Security by Design

  - Use newtype pattern for sensitive data
  - Make invalid states unrepresentable through enums and type system
  - Validate at boundaries: sanitize all external input
  - Never log or expose sensitive data in errors
  - Use constant-time operations for cryptographic comparisons
  - Use secrecy crate for sensitive data in memory
  - Minimize unsafe: only use when necessary and document invariants

- Thread Safety

  - Design for Send + Sync from the start
  - Prefer message passing with channels over shared state
  - Arc for shared ownership
  - Mutex/RwLock for shared mutation
  - Atomic types for simple counters/flags
  - Tokio or async-std for I/O-bound work
  - Rayon for CPU-bound parallelism

- Error Handling

  - Use Result and Option, avoid unwrap/expect in production
  - Use thiserror for libraries, anyhow for applications
  - Add context when propagating errors
  - For async operations, prefer `.await.map_err(|e| ...)` over `.await?` to provide error context
  - Never silently ignore errors; always handle or log them appropriately

- Key Patterns

  - Newtype: wrap primitives to prevent type confusion
  - Type state: encode state machine transitions in types
  - Builder: for complex initialization with many optional fields
  - Interior mutability: Cell/RefCell for single-threaded, Mutex/RwLock for multi-threaded

- Testing
  - Test error paths and edge cases
  - Use property-based testing for complex logic
  - Use loom for testing concurrent code thread safety
