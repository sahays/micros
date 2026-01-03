---
name: flutter-development
description:
  Develop secure, performant Flutter applications using language and framework best practices. Use when writing Flutter
  code requiring state management, async operations, security, or performance optimization. Focuses on development
  practices, not UI design.
---

# Flutter Development

## State Management

**Prefer immutability**: Use immutable data classes. Rebuild widgets instead of mutating state.

**State scope**: Keep state as local as possible. Only lift state when multiple widgets need access.

**State management patterns**:

- **Local state**: setState for simple, widget-local state
- **InheritedWidget/Provider**: For sharing state down widget tree
- **Riverpod**: Type-safe, testable, compile-time safe provider alternative
- **BLoC**: For complex business logic with streams

**Avoid**: Global mutable state, singletons with mutable state.

## Widget Best Practices

**const constructors**: Use const for widgets that don't change. Prevents unnecessary rebuilds.

**Extract widgets**: When build() exceeds 20 lines or logic repeats.

**Builder methods vs widgets**: Prefer separate widget classes over builder methods. Better performance and readability.

**Keys**: Use when reordering lists or preserving state. ValueKey for data-driven, ObjectKey for objects.

**Avoid**: Deep widget trees. Extract subtrees into separate widgets.

## Performance Optimization

**Build method efficiency**:

- Keep build() pure and fast
- Don't perform expensive operations in build()
- Move computations to state initialization or async operations

**ListView optimization**:

- Use ListView.builder for long lists
- Implement itemExtent when items have fixed height
- Use AutomaticKeepAliveClientMixin for expensive list items

**Images**:

- Use cached_network_image for network images
- Specify image dimensions to avoid layout thrashing
- Use appropriate image formats (WebP for better compression)

**Async operations**:

- Never block UI thread
- Use isolates for CPU-intensive work
- Debounce user input for expensive operations

**Memory management**:

- Dispose controllers, streams, and subscriptions
- Use weak references for listeners when appropriate
- Avoid memory leaks from unclosed streams

## Security Best Practices

**Secure storage**:

- Use flutter_secure_storage for sensitive data (tokens, credentials)
- Never store secrets in SharedPreferences (unencrypted)
- Never hardcode API keys or secrets in code

**Network requests**:

- Always use HTTPS, never HTTP
- Certificate pinning for critical APIs
- Validate SSL certificates
- Timeout configurations for all requests

**Input validation**:

- Validate all user input before processing
- Sanitize input before display to prevent injection
- Use TextInputFormatter for real-time validation

**Authentication**:

- Store tokens securely (flutter_secure_storage)
- Implement token refresh logic
- Clear sensitive data on logout
- Use OAuth 2.0 / OIDC for authentication

**Data exposure**:

- Don't log sensitive data
- Obfuscate code in release builds
- Remove debug prints in production
- Be careful with screenshot security on sensitive screens

## Async Programming

**Future best practices**:

- Always handle errors with catchError or try-catch
- Use async/await for readability over then()
- Don't use async for synchronous code

**Stream management**:

- Always close StreamController and subscriptions
- Use StreamBuilder for UI updates from streams
- Prefer BroadcastStream when multiple listeners needed

**FutureBuilder / StreamBuilder**:

- Always handle all ConnectionState cases
- Show loading, error, and data states
- Don't initiate async operations inside builders

**Avoid**: Nested callbacks (callback hell). Use async/await instead.

## Error Handling

**Graceful degradation**: App should never crash. Catch and handle all errors.

**Error boundaries**:

- Use runZonedGuarded for global error handling
- FlutterError.onError for Flutter framework errors
- PlatformDispatcher.instance.onError for platform errors

**User-facing errors**: Show meaningful messages, not stack traces or technical details.

**Logging**: Log errors with context for debugging. Use error tracking service (Sentry, Crashlytics).

**Validation errors**: Validate early, show clear error messages inline.

## Code Organization

**Feature-based structure**: Organize by feature, not type (widgets, models, etc.).

```
lib/
├── features/
│   ├── auth/
│   ├── profile/
│   └── settings/
├── core/
│   ├── network/
│   ├── storage/
│   └── utils/
└── shared/
    └── widgets/
```

**Separation of concerns**: Separate UI, business logic, and data layers.

**Dependency injection**: Use Provider or Riverpod for dependency injection. Testable and decoupled.

## Platform-Specific Code

**Platform channels**: For native functionality not available in Flutter.

**Method channels**: Request/response pattern for calling native code.

**Event channels**: Stream events from native to Flutter.

**Conditional imports**: Use dart:io and kIsWeb for platform-specific code.

**Platform detection**: Check Platform.isIOS, Platform.isAndroid before platform-specific code.

## Dependencies and Packages

**Minimize dependencies**: Each package increases app size and potential vulnerabilities.

**Verify packages**: Check package popularity, maintenance, and security before adding.

**Pin versions**: Use exact versions or compatible ranges, not any (^).

**Update regularly**: Keep dependencies updated for security patches.

**Audit dependencies**: Review transitive dependencies for security issues.

## Null Safety

**Enable sound null safety**: Required for modern Flutter development.

**Avoid null checks**: Use non-nullable types by default. Nullable only when needed.

**Late variables**: Use late for non-nullable variables initialized after declaration. Be careful of LateInitializationError.

**Null-aware operators**: Use ?., ??, ??=, and conditional access.

## Testing

**Unit tests**: Test business logic, models, utilities. Fast and isolated.

**Widget tests**: Test widget behavior and interactions. Mock dependencies.

**Integration tests**: Test complete user flows. Run on device/emulator.

**Test coverage**: Aim for high coverage on business logic, lower on UI.

**Mocking**: Use mockito or mocktail for mocking dependencies.

**Golden tests**: For UI regression testing. Catch unintended visual changes.

## Build and Release

**Obfuscation**: Enable code obfuscation in release builds.

**Minification**: Remove unused code with tree shaking.

**Build modes**: Debug (development), Profile (performance), Release (production).

**Environment configuration**: Use --dart-define for environment-specific config.

**Version management**: Semantic versioning. Increment build numbers for each release.

## Common Security Pitfalls

**Don't**:

- Store API keys in code or assets
- Use HTTP for sensitive data
- Trust user input without validation
- Log sensitive information
- Use SharedPreferences for secrets
- Expose debugging tools in production

**Do**:

- Use environment variables for configuration
- Validate and sanitize all inputs
- Implement certificate pinning
- Use secure storage for sensitive data
- Clear sensitive data on logout
- Test security on physical devices

## Performance Monitoring

**DevTools**: Use Flutter DevTools for performance profiling.

**Frame rendering**: Keep frames under 16ms (60fps). Use performance overlay to monitor.

**Memory profiling**: Monitor for memory leaks and excessive allocations.

**Network profiling**: Track API call frequency and payload sizes.

**App size**: Monitor app size. Use deferred loading for features.
