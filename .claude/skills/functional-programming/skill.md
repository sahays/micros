---
name: functional-programming
description: Write functional code following best practices for reusability, extensibility, and maintainability. Use when developing code that should minimize bugs through immutability, pure functions, and composition. Applies across all programming languages.
---

# Functional Programming Best Practices

## Pure Functions

Write functions that depend only on inputs and produce no side effects. Same input always produces same output. Makes code predictable, testable, and reusable.

**Isolate side effects**: Push I/O, mutations, and external dependencies to boundaries. Keep core logic pure.

## Immutability First

Default to immutable data structures. Create new values instead of modifying existing ones. Prevents unexpected state changes and makes code easier to reason about.

**When mutation is necessary**: Isolate it to small, well-defined scopes. Make mutability explicit and local.

## Composition Over Inheritance

Build complex behavior by composing small, focused functions. Each function does one thing well.

**Function composition**: Chain operations where output of one becomes input to next. Prefer pipelines over nested calls.

**Avoid deep hierarchies**: Prefer flat composition of behaviors over inheritance trees.

## Higher-Order Functions

Use functions that accept or return other functions. Enables powerful abstractions and code reuse.

**Common patterns**: Map, filter, reduce for data transformation. Partial application for configuration. Function decorators for cross-cutting concerns.

## Declarative Over Imperative

Express what you want, not how to do it. Reduces cognitive load and makes intent clear.

**Prefer**: Descriptive transformations over step-by-step procedures. Let abstractions handle implementation details.

## Type Safety

Use strong typing to encode constraints and invariants. Make illegal states unrepresentable.

**Leverage type inference**: Let the compiler verify correctness. Use types as documentation.

## Self-Documenting Code

**Function names**: Use clear, descriptive names that explain purpose. Verb phrases for actions, noun phrases for queries.

**Small functions**: Each function should fit in your head. If it's complex, decompose it.

**Meaningful variable names**: Avoid abbreviations. Use domain language.

**Types as documentation**: Function signatures should reveal intent. Good types eliminate need for comments.

## Minimize Shared State

Avoid global variables and shared mutable state. Pass dependencies explicitly.

**Data flow**: Make data dependencies visible through function parameters. Makes testing and reasoning easier.

## Error Handling

Use explicit error types (Result, Either, Option) instead of exceptions for expected errors. Makes error handling visible in type signatures.

**Railway-oriented programming**: Chain operations that may fail. Handle errors at appropriate boundaries.

## Referential Transparency

Expressions should be replaceable with their values without changing program behavior. Enables reasoning, refactoring, and optimization.

**Avoid**: Hidden dependencies, global state, time-dependent operations in core logic.

## Testing

Pure functions are trivial to test. No mocking or setup required.

**Property-based testing**: Test invariants and laws rather than specific examples. Reveals edge cases.

**Focus on boundaries**: Test impure code at system boundaries where side effects occur.

## Refactoring Principles

**Extract functions**: When logic repeats or becomes complex.

**Extract constants**: For magic numbers and strings.

**Parameterize differences**: When functions are similar, extract common logic and parameterize variations.

**Name intermediate steps**: Break complex expressions into named values that explain each transformation.
