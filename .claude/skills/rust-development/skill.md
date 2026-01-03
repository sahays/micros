---
name: rust-development
description: Develop secure, thread-safe Rust using functional patterns. Use when writing Rust code requiring memory safety, concurrency, zero-cost abstractions, or systems programming with security constraints.
---

# Rust Development

## Functional-First Approach

Prefer immutability and functional combinators over imperative patterns. Use iterators, map/filter/fold instead of mutable loops. Minimize use of `mut`.

## Security by Design

**Type safety for security**: Use newtype pattern for sensitive data. Make invalid states unrepresentable through enums and type system.

**Validate at boundaries**: Sanitize all external input. Never log or expose sensitive data in errors.

**Cryptographic safety**: Use constant-time operations for comparisons. Consider timing attacks. Use `secrecy` crate for sensitive data in memory.

**Minimize unsafe**: Only use when necessary and document invariants. Prefer safe abstractions.

## Thread Safety

**Design for Send + Sync**: Consider thread safety from the start, not as afterthought.

**Prefer message passing**: Use channels over shared state when possible.

**Shared state patterns**:
- Arc for shared ownership
- Mutex/RwLock for shared mutation
- Atomic types for simple counters/flags

**Async patterns**: Use Tokio or async-std for I/O-bound work. Use rayon for CPU-bound parallelism.

## Error Handling

Use Result and Option. Avoid `unwrap`/`expect` in production. Use `thiserror` for libraries, `anyhow` for applications. Add context when propagating errors.

## Key Patterns

**Newtype**: Wrap primitives to prevent type confusion and add semantic meaning.

**Type state**: Encode state machine transitions in types to make illegal states impossible.

**Builder**: For complex initialization with many optional fields.

**Interior mutability**: Cell/RefCell for single-threaded, Mutex/RwLock for multi-threaded.

## Testing

Test error paths and edge cases. Use property-based testing for complex logic. Test thread safety with `loom` for concurrent code.
