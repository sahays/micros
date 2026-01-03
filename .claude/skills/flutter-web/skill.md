---
name: flutter-web
description:
  Develop Flutter applications specifically for web deployment. Use when building Flutter web apps with considerations
  for SEO, performance, browser compatibility, and web-specific features. Focuses on web platform specifics, not general
  Flutter development.
---

# Flutter Web Development

## Rendering Modes

**HTML renderer**: Better text rendering, smaller download size, better SEO. Default for mobile browsers.

**CanvasKit renderer**: Better performance for graphics-heavy apps, consistent rendering. Default for desktop browsers.

**Auto mode**: Flutter chooses based on device. Recommended for most cases.

**Override**: Use `--web-renderer html` or `--web-renderer canvaskit` flag when building.

**Choose HTML for**: Text-heavy apps, SEO-critical content, smaller initial load.

**Choose CanvasKit for**: Games, complex graphics, custom painting, consistent cross-browser rendering.

## Loading and Splash Screen

**Initial load time**: Flutter web downloads engine binaries before app starts. Show splash screen during download.

**Custom splash screen**: Edit `web/index.html` to customize loading experience.

**Loading indicator**: Add CSS animation or spinner in index.html body. Visible until Flutter takes over.

**Progress tracking**: Use `window.flutter_loading_progress` event to show download progress.

**Optimize perception**: Branded splash screen makes wait feel intentional, not broken.

**Keep simple**: Plain HTML/CSS only. No JavaScript frameworks. Must load instantly.

## SEO and Metadata

**Meta tags**: Configure in `web/index.html` - title, description, keywords, Open Graph, Twitter cards.

**Structured data**: Add JSON-LD for rich search results.

**Dynamic metadata**: Use flutter_seo package or custom head management for route-specific metadata.

**Sitemap**: Generate sitemap.xml for search engines. Update when routes change.

**robots.txt**: Configure crawling rules in web/ directory.

**Limitation**: Client-side rendering limits SEO. Consider server-side rendering for SEO-critical apps.

## URL Strategy

**Hash-based routing** (`#/route`): Default. Works everywhere, no server config needed.

**Path-based routing** (`/route`): Clean URLs, better SEO. Requires server configuration.

**Configure path-based**:

```dart
import 'package:flutter_web_plugins/url_strategy.dart';

void main() {
  usePathUrlStrategy();  // Remove # from URLs
  runApp(MyApp());
}
```

**Server requirement**: Serve index.html for all routes when using path-based routing.

## Performance Optimization

**Initial load time**:

- Minimize app size with tree shaking
- Use deferred loading for routes
- Lazy load images
- Compress assets
- Use CDN for static assets

**Bundle size**:

- HTML renderer produces smaller bundles
- Remove unused dependencies
- Use code splitting with deferred imports

**Web workers**: Offload heavy computations to web workers. Use flutter_isolate or native web workers.

**Caching**:

- Configure service worker for offline support
- Cache static assets aggressively
- Use appropriate Cache-Control headers

**Images**:

- Use WebP format for better compression
- Implement lazy loading for images
- Specify image dimensions to avoid layout shifts
- Use responsive images with srcset

## Browser Compatibility

**Target browsers**: Chrome, Firefox, Safari, Edge (Chromium). Test on all.

**Mobile browsers**: iOS Safari, Chrome Mobile. Different rendering characteristics.

**Feature detection**: Check browser capabilities before using web-specific APIs.

**Polyfills**: Not automatically included. Add if supporting older browsers.

**Test extensively**: Different browsers handle Flutter web differently, especially with CanvasKit.

## Responsive Design

**Breakpoints**: Design for mobile, tablet, desktop. Use MediaQuery for screen size.

**Adaptive layouts**: Use LayoutBuilder to adapt to available space.

**Navigation**: Drawer for mobile, sidebar for desktop. NavigationRail for medium screens.

**Touch vs mouse**: Support both touch and mouse interactions. Hover states for desktop.

**Keyboard navigation**: Essential for web accessibility. Support tab navigation and shortcuts.

## PWA Features

**Manifest**: Configure `web/manifest.json` for PWA features.

**Service worker**: Enable in `web/flutter_service_worker.js` for offline support.

**Install prompt**: Configure app installation on supported browsers.

**Offline functionality**: Cache critical assets and data. Handle offline state gracefully.

**Push notifications**: Use web push notifications API. Require user permission.

**App icons**: Provide icons in various sizes in web/ directory.

## JavaScript Interop

**js package**: Call JavaScript from Dart using package:js.

**@JS() annotation**: Define JavaScript APIs in Dart.

**dart:html**: Access browser APIs directly.

**postMessage**: Communicate between Flutter and external JavaScript.

**Avoid**: Excessive JS interop. Impacts performance and type safety.

## Web-Specific Security

**Content Security Policy**: Configure CSP headers to prevent XSS.

**CORS**: Handle cross-origin requests properly. Configure server CORS headers.

**HTTPS only**: Always serve Flutter web apps over HTTPS in production.

**Iframe embedding**: Use X-Frame-Options or CSP frame-ancestors to control embedding.

**Secrets**: Never expose API keys in client-side code. Use backend proxy.

**Input validation**: Validate all inputs. Web apps are exposed to broader attack surface.

## Routing and Navigation

**go_router**: Recommended for web routing. Type-safe, supports deep linking.

**Deep linking**: All routes should be directly accessible via URL.

**Browser back button**: Must work correctly. Test navigation history.

**Route parameters**: Extract from URL path or query parameters.

**Redirect handling**: Handle authentication redirects, route guards.

**404 handling**: Show appropriate error page for invalid routes.

## Asset Management

**Asset optimization**:

- Compress images (WebP, optimized PNG/JPEG)
- Minify JSON and other text assets
- Remove unused assets

**Asset loading**:

- Lazy load non-critical assets
- Preload critical assets
- Use asset variants for different screen densities

**Fonts**:

- Subset fonts to include only needed characters
- Use system fonts when possible for faster load
- Preload fonts to avoid FOUT (Flash of Unstyled Text)

**CDN**: Host large assets on CDN for faster global delivery.

## Web-Specific Packages

**Prefer web-compatible packages**: Check pub.dev for web platform support.

**Common web packages**:

- url_launcher_web: Open URLs and emails
- shared_preferences_web: Browser localStorage
- file_picker_web: File upload from browser
- image_picker_web: Image selection from browser

**Platform detection**: Use `kIsWeb` to conditionally use web-specific code.

## Deployment

**Build for production**: `flutter build web --release`

**Web server configuration**:

- Serve index.html for all routes (path-based routing)
- Enable gzip compression
- Set appropriate cache headers
- Serve over HTTPS

**Hosting options**:

- Firebase Hosting: Easy, supports Flutter web well
- Netlify/Vercel: Good for static sites
- GitHub Pages: Free for public repos
- Cloud providers: AWS S3, GCP Cloud Storage, Azure Blob

**Environment configuration**: Use `--dart-define` for environment-specific config.

## Performance Monitoring

**Web vitals**: Monitor Largest Contentful Paint (LCP), First Input Delay (FID), Cumulative Layout Shift (CLS).

**Lighthouse**: Run Lighthouse audits for performance, accessibility, SEO.

**Analytics**: Use firebase_analytics or Google Analytics for web.

**Error tracking**: Sentry, Crashlytics for web error monitoring.

**RUM**: Real User Monitoring to track actual user performance.

## Accessibility

**Semantic HTML**: Flutter web generates DOM. Ensure proper semantics.

**ARIA labels**: Use Semantics widget for screen reader support.

**Keyboard navigation**: All interactive elements must be keyboard accessible.

**Focus management**: Proper focus order and visible focus indicators.

**Color contrast**: Meet WCAG AA standards minimum.

**Screen reader testing**: Test with NVDA, JAWS, VoiceOver.

## Limitations and Workarounds

**No native mobile features**: Camera, GPS, sensors not available on web. Use web APIs instead.

**File system access**: Limited. Use File System Access API (limited browser support) or downloads.

**Background tasks**: Limited compared to mobile. Use service workers for background sync.

**App size**: Larger than mobile apps. Optimize aggressively.

**Performance**: Generally slower than native web frameworks for simple apps. Better for complex UI.

**Hot reload**: Works but slower than mobile hot reload.

## Web-Specific Debugging

**Browser DevTools**: Use Chrome/Firefox DevTools for debugging.

**Network inspection**: Monitor asset loading and API calls.

**Console logging**: Use debugPrint, appears in browser console.

**Performance profiling**: Use DevTools Performance tab.

**Responsive design testing**: Use browser responsive mode to test different screen sizes.

## Common Pitfalls

**Don't**:

- Use mobile-specific packages without checking web support
- Ignore SEO and metadata
- Assume all Flutter features work on web
- Neglect browser compatibility testing
- Use hash URLs without considering SEO impact
- Forget to configure server for path-based routing

**Do**:

- Test on multiple browsers and devices
- Optimize for initial load time
- Implement proper error boundaries
- Use web-specific packages when available
- Configure PWA features for better UX
- Monitor web vitals and performance
