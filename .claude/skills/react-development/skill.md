---
name: react-development
description:
  Develop optimized client-only React applications that call APIs for dynamic behavior. Use when building React SPAs
  with focus on performance, bundle optimization, and best practices. Excludes Next.js and styling frameworks.
---

- Component Design
  - Use functional components with hooks, not class components
  - Single responsibility: each component does one thing well
  - Composition over inheritance: build complex UI by composing simple components
  - Define clear prop types: use TypeScript interfaces or PropTypes
  - Avoid prop drilling: use Zustand or context when passing props through 3+ levels

- Component Library and Icons
  - Shadcn UI: copy-paste component library, own the code, customize freely, not a dependency, full TypeScript support, accessible by default (Radix UI primitives)
  - Install components individually with CLI, modify as needed
  - Lucide React for icons: lightweight, tree-shakable, consistent design, simple API
  - Don't mix icon libraries: pick Lucide and stick with it

- Performance Optimization
  - Memoization: React.memo() for expensive components with same props, useMemo() for expensive calculations, useCallback() for functions passed to memoized children, don't over-optimize
  - Code splitting: React.lazy() and Suspense for route-based splitting, dynamic imports for large dependencies, split by route not by component
  - List rendering: use stable key prop (prefer ID over index), virtualize long lists (react-window, react-virtuoso), avoid inline functions in map
  - Bundle optimization: tree shaking with ES modules, analyze bundle with webpack-bundle-analyzer, replace heavy libraries, lazy load non-critical features

- State Management
  - Local state first: use useState for component-local state, don't lift unnecessarily
  - Zustand for global client state: simple, lightweight, less boilerplate than Redux, use for auth, UI state, user preferences
  - TanStack Query for server state: API data caching, automatic refetching, optimistic updates, don't store API data in Zustand
  - Separate concerns: TanStack Query for server state (API data), Zustand for client state (UI, preferences, auth), useState for component-local state
  - Context: use sparingly, Zustand is better for shared state
  - Avoid Redux for new projects: Zustand is simpler

- Hooks Best Practices
  - Rules of hooks: only call at top level, never in conditions/loops, only call from React functions
  - useEffect: include all dependencies, clean up side effects, avoid for derived state (use useMemo), avoid for event handlers
  - Custom hooks: extract reusable logic, prefix with use, return arrays for multiple values, objects for many values
  - useState: use functional updates when new state depends on previous
  - useRef: for DOM references and mutable values that don't trigger re-renders

- API Integration
  - TanStack Query for all API calls: preferred pattern for data fetching
  - useQuery: GET requests, automatic caching and refetching
  - useMutation: POST/PUT/DELETE requests
  - Query invalidation: refetch queries after mutations
  - Optimistic updates: update UI before server response
  - Error handling: TanStack Query provides error state, show user-friendly messages, retry logic built-in, error boundaries for component errors
  - Loading states: use isLoading and isFetching, skeleton screens better than spinners
  - Authentication: store tokens in memory or httpOnly cookies, attach auth headers via interceptors, refresh tokens automatically, redirect on 401, store auth state in Zustand

- Routing
  - React Router: standard for client-side routing
  - Route-based code splitting: lazy load route components
  - Protected routes: wrap routes requiring auth in guard component
  - URL state: use query params for filterable/shareable state (search, filters, pagination)
  - Programmatic navigation: use useNavigate() hook
  - 404 handling: catch-all route for undefined paths

- Build Optimization
  - Production build: npm run build creates optimized bundle
  - Environment variables: prefix with REACT_APP_ (CRA) or VITE_ (Vite)
  - Source maps: enable in dev, disable or use hidden-source-map in production
  - Compression: enable gzip/brotli on server
  - Asset optimization: optimize images (WebP), use SVG for icons, lazy load below fold, use CDN
  - Cache busting: build tool adds hashes automatically
  - Bundle analysis: regularly check bundle size, remove unused dependencies

- Error Boundaries
  - Implement error boundaries: catch errors in component tree, show fallback UI
  - Granularity: wrap critical sections, not entire app
  - Logging: log errors to error tracking service (Sentry, Rollbar)
  - Fallback UI: show helpful message, offer recovery action
  - Development vs production: detailed errors in dev, user-friendly in production

- Security
  - XSS prevention: React escapes by default, don't use dangerouslySetInnerHTML unless necessary, sanitize input with DOMPurify if needed, validate URLs
  - API security: never expose API keys in client code, use environment variables, implement CORS properly on backend, always use HTTPS
  - Dependencies: regularly audit with npm audit, update vulnerable packages
  - Secrets: never commit secrets, use environment variables

- Code Organization
  - Feature-based structure: src/features/, src/components/ (shared), src/hooks/, src/services/ (API clients), src/utils/, src/context/, src/types/
  - One component per file
  - Co-locate styles if component-scoped

- TypeScript Integration
  - Use TypeScript: type safety catches errors early
  - Component props: define interfaces for all component props
  - Hooks typing: type useState, useReducer, custom hooks
  - API types: define types for API responses, generate from OpenAPI if possible
  - Avoid any: use unknown and type guards

- Testing
  - Testing Library: use @testing-library/react, not enzyme
  - Test: user interactions and workflows, component rendering with different props, error states, custom hooks logic
  - Don't test implementation details: test behavior, not internal state
  - Mock API calls: use MSW (Mock Service Worker)
  - Integration tests over unit tests

- Development Tools
  - Vite: modern build tool, faster than webpack
  - ESLint: enforce code quality, use react-hooks plugin
  - Prettier: consistent code formatting
  - React DevTools: debug component hierarchy and state
  - Network tab: monitor API calls

- Performance Monitoring
  - React DevTools Profiler: identify slow renders
  - Web Vitals: monitor LCP, FID, CLS, use web-vitals package
  - Bundle size: track over time, set budgets
  - Lighthouse: run regularly for performance, accessibility, SEO scores
  - Real User Monitoring: track actual user performance (Sentry, Datadog)

- Common Patterns
  - TanStack Query: useQuery for GET with caching, useMutation for POST/PUT/DELETE with invalidation, query keys for cache management, optimistic updates
  - Zustand: create stores with create(), simple selectors, middleware for persistence, no boilerplate, no providers
  - Shadcn UI: start with base components (Button, Card, Dialog), customize in codebase, compose complex UI, use with Lucide icons

- Anti-Patterns to Avoid
  - Never mutate state directly
  - Never use index as key in lists
  - Never use excessive useEffect
  - Never prop drill through many levels
  - Never define inline functions in JSX (when memoization matters)
  - Never ignore cleanup of subscriptions/timers
  - Never ignore warning messages
  - Never prematurely optimize
  - Always use immutable updates
  - Always use stable, unique keys
  - Always prefer derived state over useEffect
  - Always use context or state management to avoid drilling
  - Always extract callbacks when needed
  - Always return cleanup from useEffect
  - Always fix all warnings
  - Always measure before optimizing

- Production Checklist
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
