---
name: rust-htmx-frontend
description:
  Build secure, performant frontends using Rust web frameworks and Htmx for progressive enhancement. Use when developing
  server-rendered applications with dynamic behavior, minimal JavaScript, and focus on performance and security.
---

- Framework Selection

  - Axum: use Axum
  - Template engines: use Askama (compile-time)

- Htmx Core Principles

  - Progressive enhancement: start with working HTML forms, enhance with Htmx
  - Hypermedia-driven: server returns HTML fragments, not JSON
  - Minimal JavaScript: use Htmx attributes for interactivity, avoid custom JS when possible
  - Semantic HTML: proper forms, buttons, links work without JavaScript
  - Graceful degradation: application functions without Htmx when necessary

- Htmx Attributes

  - hx-get/post/put/delete/patch: make AJAX requests from any element
  - hx-target: specify where to insert response (CSS selector), use #id or .class or this, closest, next, previous
  - hx-swap: control how content is swapped (innerHTML, outerHTML, beforebegin, afterbegin, beforeend, afterend, delete,
    none)
  - hx-trigger: specify events that trigger requests (click, submit, change, load, revealed, every Xs, custom events)
  - hx-swap-oob: out-of-band swaps for updating multiple parts of page from single response
  - hx-push-url: update browser URL and history
  - hx-boost: progressively enhance normal links and forms
  - hx-indicator: show loading indicators during requests
  - hx-vals: include additional values in request as JSON
  - hx-headers: add custom headers to requests
  - hx-confirm: show confirmation dialog before request

- Response Headers

  - HX-Trigger: trigger client-side events after swap (for custom JS, close modals, refresh other sections)
  - HX-Redirect: client-side redirect to new URL
  - HX-Refresh: force client-side page refresh
  - HX-Push-Url: push new URL to browser history
  - HX-Retarget: override hx-target attribute
  - HX-Reswap: override hx-swap attribute
  - HX-Location: client-side redirect with context for Htmx

- Template Patterns

  - Partial templates: create reusable HTML fragments for Htmx responses
  - Layout composition: base layout with blocks for full pages, minimal fragments for Htmx responses
  - Conditional rendering: render different content based on request headers (HX-Request for Htmx detection)
  - Component templates: reusable components that work in full pages and as fragments
  - Form templates: extract forms as partials for reuse in create/edit flows

- Routing Architecture

  - Full page routes: return complete HTML documents for initial page loads
  - Fragment routes: return HTML fragments for Htmx requests
  - Dual routes: same endpoint serves full page or fragment based on HX-Request header
  - RESTful structure: GET /items (list), GET /items/:id (detail), GET /items/:id/edit (edit form), POST /items
    (create), PUT /items/:id (update), DELETE /items/:id (delete)
  - Fragment endpoints: GET /items/:id/fragment for partial updates

- State Management

  - Server-side sessions: use session cookies for user state, tower-sessions or actix-session
  - CSRF protection: required for all mutations, use tower-csrf or actix-csrf
  - Form state: preserve form data across validation failures, return pre-filled forms
  - URL state: use query parameters for filters, pagination, sort order
  - Hidden fields: maintain state in HTML forms with hidden inputs when appropriate
  - Flash messages: temporary messages stored in session, display once then clear

- Form Handling

  - POST-Redirect-GET: prevent duplicate submissions on refresh for full page forms
  - Inline validation: use hx-post with hx-target on individual fields for live validation
  - Error display: return form fragment with errors highlighted, show errors next to fields
  - Optimistic updates: swap UI immediately, revert on error
  - Multi-step forms: use wizard pattern with hidden fields or session state
  - File uploads: use multipart/form-data, validate size/type server-side, show progress with HX-Trigger events

- Common UI Patterns

  - Infinite scroll: hx-get on sentinel element with hx-trigger="revealed", append new items with hx-swap="beforeend"
  - Search-as-you-type: hx-get hx-trigger="keyup changed delay:300ms" on input
  - Inline editing: click to show edit form, save with hx-put, cancel returns read-only view
  - Modals: return modal HTML fragment, use Alpine.js or hyperscript for show/hide, close via HX-Trigger header
  - Toasts/notifications: out-of-band swap notification fragment, auto-dismiss with Alpine.js
  - Lazy loading: hx-get hx-trigger="load" on placeholder divs
  - Polling: hx-get hx-trigger="every 5s" for live updates
  - Dependent dropdowns: hx-get on first dropdown change, update second dropdown with response
  - Delete confirmation: hx-confirm attribute or modal pattern

- Performance Optimization

  - Minimize fragment size: return only changed HTML, use hx-swap-oob for multiple updates
  - Template caching: cache compiled templates, not rendered output
  - Conditional rendering: skip expensive rendering if not needed for Htmx fragments
  - Database queries: use connection pooling, optimize N+1 queries, index properly
  - HTTP/2: enable for multiplexing parallel Htmx requests
  - Compression: enable gzip/brotli for HTML responses
  - Static assets: serve CSS/JS/images from CDN, use cache headers aggressively
  - Lazy loading: defer loading non-critical content until needed

- Security

  - CSRF tokens: include in all forms, validate on mutations
  - Input validation: validate all inputs server-side, sanitize for XSS
  - Output encoding: template engines auto-escape by default, be careful with raw HTML
  - Authentication: session-based auth with httpOnly secure cookies
  - Authorization: check permissions server-side for every request
  - Rate limiting: prevent abuse of Htmx endpoints, use tower-governor or actix-governor
  - Content Security Policy: strict CSP, allow inline styles/scripts only when necessary
  - HTTPS only: redirect HTTP to HTTPS, use HSTS header
  - Secret management: use secrecy crate, never log sensitive data

- Error Handling

  - Validation errors: return 422 with form fragment showing errors
  - Server errors: return 500 with user-friendly error fragment
  - Not found: return 404 with helpful message fragment
  - Unauthorized: return 401, redirect to login via HX-Redirect header
  - Htmx error events: handle with hx-on::after-request for client-side error display
  - Error boundaries: show fallback UI for unexpected errors
  - Logging: structured logging with tracing, include request context

- Progressive Enhancement Strategy

  - Start with working HTML: forms submit, links navigate
  - Add Htmx: enhance with hx attributes for better UX
  - Add minimal JS: use Alpine.js or hyperscript for client-only interactions (modals, dropdowns)
  - Test without JS: ensure core functionality works
  - Accessibility: keyboard navigation, screen reader support, ARIA attributes

- CSS and Styling

  - Utility-first CSS: Tailwind CSS works excellently with Htmx
  - Scoped styles: use BEM or CSS modules to prevent conflicts
  - Transition classes: CSS transitions for smooth swaps, use htmx.config.defaultSwapDelay
  - Loading states: show spinners or skeleton screens with hx-indicator
  - Theme support: CSS variables for theming, persist theme choice in session
  - Page transitions: eliminate browser view flickers on page navigation
  - Page loading progressbars: use YouTube-like progressbar at the top of the page while page transitions

- Integration with JavaScript

  - Minimal JS: prefer Htmx and server-side logic
  - Alpine.js: lightweight reactivity for client-only state (modals, dropdowns, tabs)
  - Hyperscript: event-oriented scripting alternative to Alpine.js
  - Custom events: trigger from Htmx via HX-Trigger header, listen with JS
  - Htmx events: listen to htmx:afterSwap, htmx:beforeRequest for custom behavior
  - Libraries: integrate charts, maps, rich text editors on specific pages only

- Testing

  - Integration tests: test full request/response cycle with real templates
  - Template tests: verify templates render correctly with different data
  - Form tests: test validation, error handling, success flows
  - Fragment tests: verify Htmx endpoints return correct fragments
  - CSRF tests: verify protection works
  - Accessibility tests: automated checks with axe-core

- Development Workflow

  - Live reload: use cargo-watch with browser auto-refresh
  - Template hot reload: some engines support reloading without restart
  - Request logging: log all Htmx requests for debugging
  - Dev tools: browser DevTools network tab shows Htmx requests
  - Htmx debug: enable htmx.logAll() in development

- Code Organization

  - handlers/: request handlers grouped by feature
  - templates/: HTML templates, organized by feature with shared layouts/components
  - models/: database models and business logic
  - services/: business logic services
  - middleware/: authentication, CSRF, logging middleware
  - extractors/: custom Axum extractors for common patterns

- Common Patterns with Axum

  - HxRequest extractor: detect Htmx requests via HX-Request header
  - Conditional rendering: return full page or fragment based on HxRequest
  - Template response: custom response type that renders templates
  - Form extractor: parse form data with validation
  - Session extractor: access session data
  - CSRF middleware: validate tokens automatically

- Anti-Patterns to Avoid

  - Never return JSON to Htmx (return HTML fragments)
  - Never build complex client-side state machines (use server state)
  - Never skip CSRF protection on mutations
  - Never trust client-side validation alone
  - Never log sensitive data or tokens
  - Never use synchronous I/O (use async/await)
  - Never ignore authentication on Htmx endpoints
  - Always return appropriate HTTP status codes
  - Always validate input server-side
  - Always use async handlers for I/O operations
  - Always check authorization for every endpoint
  - Always use htmlOnly secure cookies for sessions
  - Always enable CSRF protection
  - Always sanitize and escape output

- Production Checklist
  - Set secure session cookies (httpOnly, secure, sameSite)
  - Enable CSRF protection
  - Configure CSP headers
  - Set up error logging and monitoring
  - Enable rate limiting
  - Optimize database queries and indexes
  - Enable HTTP/2 and compression
  - Set cache headers for static assets
  - Run security audit
  - Test without JavaScript enabled
  - Verify accessibility with screen reader
  - Load test critical paths
  - Set up health checks for monitoring
  - Zero flickers on page navigation
