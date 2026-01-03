---
name: react-development
description:
  Develop optimized client-only React applications that call APIs for dynamic behavior. Use when building React SPAs
  with focus on performance, bundle optimization, and best practices. Excludes Next.js and styling frameworks.
---

# React Development

## Component Design

**Functional components only**: Use function components with hooks. Class components are legacy.

**Single Responsibility**: Each component does one thing well. Extract when component grows complex.

**Composition over inheritance**: Build complex UI by composing simple components.

**Props interface**: Define clear prop types. Use TypeScript interfaces or PropTypes.

**Avoid prop drilling**: Use Zustand or context when passing props through 3+ levels.

## Component Library and Icons

**Shadcn UI**: Copy-paste component library. Own the code, customize freely.

**Benefits**:

- Not a dependency, just code in your project
- Full TypeScript support
- Accessible by default (Radix UI primitives)
- Customizable without fighting abstractions

**Usage**: Install components individually with CLI. Modify as needed.

**Lucide React for icons**: Consistent, tree-shakable icon set.

**Benefits**:

- Lightweight (only bundle icons you use)
- Consistent design language
- Simple API: `<Icon size={24} />`
- Works well with Shadcn components

**Don't**: Mix icon libraries. Pick Lucide and stick with it.

## Performance Optimization

**Memoization**:

- `React.memo()` for expensive components that re-render often with same props
- `useMemo()` for expensive calculations
- `useCallback()` for functions passed as props to memoized children
- Don't over-optimize. Measure first.

**Code splitting**:

- `React.lazy()` and `Suspense` for route-based splitting
- Dynamic imports for large dependencies
- Split by route, not by component unless component is very large

**List rendering**:

- Always use stable `key` prop (prefer ID over index)
- Virtualize long lists (react-window, react-virtuoso)
- Avoid inline functions in map when possible

**Bundle optimization**:

- Tree shaking: Use ES modules, avoid barrel imports
- Analyze bundle: Use webpack-bundle-analyzer or similar
- Replace heavy libraries with lighter alternatives
- Lazy load non-critical features

## State Management

**Local state first**: Use `useState` for component-local state. Don't lift unnecessarily.

**Zustand for global client state**: Simple, lightweight, less boilerplate than Redux. Use for auth, UI state, user
preferences.

**TanStack Query for server state**: API data caching, automatic refetching, optimistic updates. Don't store API data in
Zustand.

**Separate concerns**:

- TanStack Query: Server state (API data)
- Zustand: Client state (UI, preferences, auth)
- useState: Component-local state

**Context**: Use sparingly. Zustand is better for shared state. Context acceptable for theme provider.

**Avoid**: Redux for new projects. Zustand is simpler and sufficient for most apps.

## Hooks Best Practices

**Rules of hooks**:

- Only call at top level, never in conditions/loops
- Only call from React functions
- Use ESLint plugin to enforce

**useEffect**:

- Include all dependencies in dependency array
- Clean up side effects (return cleanup function)
- Avoid useEffect for derived state (use useMemo instead)
- Avoid useEffect for event handlers (use event handlers)

**Custom hooks**:

- Extract reusable logic into custom hooks
- Prefix with "use" (useAuth, useFetch, useForm)
- Return arrays for multiple values, objects for many values

**useState**: Use functional updates when new state depends on previous state.

**useRef**: For DOM references and mutable values that don't trigger re-renders.

## API Integration

**TanStack Query for all API calls**: Preferred pattern for data fetching.

**Core patterns**:

- `useQuery`: GET requests, automatic caching and refetching
- `useMutation`: POST/PUT/DELETE requests
- Query invalidation: Refetch queries after mutations
- Optimistic updates: Update UI before server response

**Error handling**:

- TanStack Query provides error state
- Show user-friendly error messages
- Retry logic built-in (configure retry count)
- Error boundaries for component errors

**Loading states**: Use `isLoading` and `isFetching` states. Skeleton screens better than spinners.

**Request cancellation**: TanStack Query handles cancellation automatically on unmount.

**Authentication**:

- Store tokens in memory or httpOnly cookies
- Attach auth headers to requests (axios/fetch interceptors)
- Refresh tokens automatically
- Redirect to login on 401
- Store auth state in Zustand

## Routing

**React Router**: Standard for client-side routing in React.

**Route-based code splitting**: Lazy load route components for smaller initial bundle.

**Protected routes**: Wrap routes requiring auth in guard component.

**URL state**: Use query params for filterable/shareable state (search, filters, pagination).

**Programmatic navigation**: Use `useNavigate()` hook, not direct history manipulation.

**404 handling**: Catch-all route for undefined paths.

## Build Optimization

**Production build**: `npm run build` creates optimized bundle.

**Environment variables**: Prefix with `REACT_APP_` for Create React App, or use Vite's `VITE_` prefix.

**Source maps**: Enable in dev, disable or use hidden-source-map in production.

**Compression**: Enable gzip/brotli compression on server.

**Asset optimization**:

- Optimize images (WebP, appropriate sizes)
- Use SVG for icons
- Lazy load images below fold
- Use CDN for static assets

**Cache busting**: Build tool adds hashes to filenames automatically.

**Bundle analysis**: Regularly check bundle size. Remove unused dependencies.

## Error Boundaries

**Implement error boundaries**: Catch errors in component tree, show fallback UI.

**Granularity**: Wrap critical sections, not entire app. Allow partial failures.

**Logging**: Log errors to error tracking service (Sentry, Rollbar).

**Fallback UI**: Show helpful message, not technical error. Offer recovery action.

**Development vs production**: Show detailed errors in dev, user-friendly in production.

## Security

**XSS prevention**:

- React escapes by default. Don't use `dangerouslySetInnerHTML` unless necessary
- Sanitize user input if using dangerouslySetInnerHTML (DOMPurify)
- Validate URLs before rendering links

**API security**:

- Never expose API keys in client code
- Use environment variables for config
- Implement CORS properly on backend
- Always use HTTPS

**Dependencies**: Regularly audit with `npm audit`. Update vulnerable packages.

**Secrets**: Never commit secrets. Use environment variables.

## Code Organization

**Feature-based structure**:

```
src/
├── features/
│   ├── auth/
│   ├── dashboard/
│   └── profile/
├── components/       # Shared components
├── hooks/           # Custom hooks
├── services/        # API clients
├── utils/           # Helper functions
├── context/         # React contexts
└── types/           # TypeScript types
```

**Component file structure**:

- One component per file
- Co-locate styles if component-scoped
- Index file for clean imports (optional)

## TypeScript Integration

**Use TypeScript**: Type safety catches errors early, better DX.

**Component props**: Define interfaces for all component props.

**Hooks typing**: Type useState, useReducer, custom hooks.

**API types**: Define types for API responses. Generate from OpenAPI if possible.

**Avoid `any`**: Use `unknown` and type guards instead.

## Testing

**Testing Library**: Use @testing-library/react, not enzyme.

**What to test**:

- User interactions and workflows
- Component rendering with different props
- Error states and edge cases
- Custom hooks logic

**Don't test implementation details**: Test behavior, not internal state.

**Mock API calls**: Use MSW (Mock Service Worker) for API mocking.

**Integration tests over unit**: Test component integration, not isolated units.

## Development Tools

**Vite**: Modern build tool, faster than webpack for development.

**ESLint**: Enforce code quality. Use react-hooks plugin.

**Prettier**: Consistent code formatting.

**React DevTools**: Debug component hierarchy and state.

**Network tab**: Monitor API calls, check payloads and timing.

## Performance Monitoring

**React DevTools Profiler**: Identify slow renders.

**Web Vitals**: Monitor LCP, FID, CLS. Use web-vitals package.

**Bundle size**: Track over time. Set budgets, alert on increases.

**Lighthouse**: Run regularly for performance, accessibility, SEO scores.

**Real User Monitoring**: Track actual user performance (Sentry, Datadog).

## Common Patterns

**TanStack Query for API data**:

- useQuery for GET requests with caching
- useMutation for POST/PUT/DELETE with invalidation
- Query keys for organized cache management
- Optimistic updates for instant UI feedback

**Zustand for client state**:

- Create stores with `create()` function
- Simple selectors with hooks
- Middleware for persistence, devtools
- No boilerplate, no providers needed

**Shadcn UI components**:

- Start with base components (Button, Card, Dialog)
- Customize in your codebase, not fighting props
- Compose complex UI from primitives
- Use with Lucide icons for consistent design

**Compound components**: Components that work together (Shadcn Tabs, Accordion).

## Anti-Patterns

**Avoid**:

- Mutating state directly
- Using index as key in lists
- Excessive use of useEffect
- Prop drilling through many levels
- Inline function definitions in JSX (when memoization matters)
- Not cleaning up subscriptions/timers
- Ignoring warning messages
- Premature optimization

**Do**:

- Use immutable updates
- Use stable, unique keys
- Prefer derived state over useEffect
- Use context or state management
- Extract callbacks when needed
- Return cleanup from useEffect
- Fix all warnings
- Measure before optimizing

## Production Checklist

**Before deploying**:

- Remove console.logs
- Enable error tracking
- Configure analytics
- Set up monitoring
- Test on production-like environment
- Check bundle size
- Run Lighthouse audit
- Test on multiple browsers/devices
- Verify environment variables
- Enable compression on server
