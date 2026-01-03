---
name: web-design
description:
  Design modern web applications including landing pages, feature pages, and admin dashboards. Use when designing layouts,
  navigation, visual hierarchy, and component systems. Focuses on design patterns and principles, not form handling or
  implementation.
---

# Web Design

## Layout Systems

**Grid-based layouts**: Use CSS Grid for complex layouts, Flexbox for component-level layout.

**12-column grid**: Standard for responsive design. Adapt columns based on breakpoints.

**Container widths**: Max-width for readability. 1200px-1400px for desktop, fluid below.

**Spacing scale**: Consistent spacing system. Use 4px or 8px base unit (4, 8, 16, 24, 32, 48, 64).

**Whitespace**: More whitespace = cleaner, more premium feel. Don't cram content.

## Responsive Breakpoints

**Mobile-first**: Design for mobile, enhance for larger screens.

**Standard breakpoints**:
- Mobile: < 640px
- Tablet: 640px - 1024px
- Desktop: > 1024px
- Large desktop: > 1440px

**Adapt patterns**: Stack on mobile, side-by-side on desktop. Hide/show elements appropriately.

## Typography

**Font pairing**: Maximum 2-3 fonts. One for headings, one for body, optionally one for code/data.

**Scale**: Use type scale for hierarchy (1.125, 1.25, or 1.333 ratio). Tools: type-scale.com

**Line height**: 1.5-1.6 for body text, 1.2-1.3 for headings.

**Line length**: 60-80 characters per line for optimal readability.

**Font weights**: Use 2-3 weights max. Regular (400), Medium (500), Bold (700).

**Hierarchy**: Clear size and weight differences between heading levels.

## Color Systems

**Primary color**: Brand color. Use for CTAs, links, key actions.

**Neutral grays**: 7-10 shades from white to black. Use for text, borders, backgrounds.

**Semantic colors**: Success (green), Error (red), Warning (yellow), Info (blue).

**Background layers**: Subtle differences between surfaces. Base, raised, overlay levels.

**Text contrast**: WCAG AA minimum (4.5:1 for normal text, 3:1 for large text).

**Color palette size**: 5-7 primary shades, 7-10 gray shades, 5 shades per semantic color.

## Dark Mode

**Design both modes together**: Not just inverted colors. Different considerations for each.

**Background layers**: Lighter backgrounds for elevated surfaces in dark mode (reverse of light mode).

**Reduce saturation**: Bright colors are harsh in dark mode. Use desaturated versions.

**Text colors**: Use lighter grays, not pure white. Reduces eye strain.

**Toggle placement**: User preference setting, not contextual. Remember user choice.

## Landing Page Design

**Hero section**: Full viewport height or 60-70% viewport. Clear value proposition and CTA.

**Above the fold**: Most important message and action visible without scrolling.

**Single CTA focus**: One primary action per section. Don't overwhelm with choices.

**Social proof**: Testimonials, logos, metrics. Build trust early.

**Visual hierarchy**: Guide eye through page with size, color, spacing.

**Scannable content**: Short paragraphs, bullet points, clear headings.

**Section rhythm**: Alternate content/image sides. Vary section backgrounds subtly.

## Feature Pages

**Feature showcase**: Lead with benefit, not feature list. Show, don't just tell.

**Screenshots/demos**: High-quality visuals. Use annotations to highlight key points.

**Progressive disclosure**: Don't dump all information at once. Reveal details on demand.

**Comparison tables**: When applicable, show how features compare to alternatives.

**CTAs throughout**: Multiple opportunities to convert as users scroll.

## Dashboard Design

**Information hierarchy**: Most important metrics at top. Drill-down details below.

**Card-based layout**: Group related information in cards. Clear visual boundaries.

**Consistent card anatomy**: Title, metric/chart, action (optional), timestamp/meta.

**Scannable metrics**: Large numbers, clear labels, trend indicators (up/down arrows).

**Data density**: Balance information with whitespace. Don't overwhelm.

**Color coding**: Use sparingly for status/severity. Too much color is noise.

**Loading states**: Show skeleton screens or progressive loading, not spinners alone.

## Navigation Patterns

**Landing pages**: Minimal top nav (logo, links, CTA). Sticky on scroll optional.

**Dashboards**: Side navigation for many sections. Top nav for global actions.

**Breadcrumbs**: Essential for deep hierarchies. Show path to current location.

**Mobile navigation**: Hamburger menu or bottom tab bar. Clear, accessible toggle.

**Active states**: Clearly indicate current page/section in navigation.

**Search**: Include search for content-heavy sites and dashboards.

## Visual Hierarchy

**Size**: Larger elements draw attention. Use for primary actions and key content.

**Color**: Bright/saturated colors stand out. Use for important elements only.

**Contrast**: High contrast elements get noticed first.

**Spacing**: Isolated elements get more attention. Group related items.

**Typography**: Bold, uppercase, or different font for emphasis.

**F-pattern**: Users scan in F-pattern. Place important content top-left.

## Component Design

**Buttons**:
- Primary: Solid background, brand color
- Secondary: Outline or muted color
- Tertiary: Text only, no background
- Size hierarchy: Large for primary actions, medium for secondary

**Cards**:
- Subtle shadow or border for definition
- Consistent padding (16-24px)
- Hover state for interactive cards
- Group related content

**Tables** (dashboards):
- Zebra striping for long tables
- Sticky headers for scrolling
- Row hover state
- Responsive: stack or scroll on mobile

**Badges/tags**:
- Small, rounded
- Subtle backgrounds, readable text
- Use for status, categories, counts

## Data Visualization (Dashboards)

**Chart selection**: Bar for comparisons, line for trends, pie for proportions (use sparingly).

**Color**: Use brand colors for primary data. Consistent color meaning across charts.

**Labels**: Direct labeling on charts when possible. Reduce reliance on legends.

**Axis**: Start at zero for bar charts. Appropriate scale for line charts.

**Interactivity**: Hover tooltips for details. Click for drill-down.

**Empty states**: Show helpful message when no data, not blank space.

## Spacing and Rhythm

**Consistent spacing**: Use spacing scale. Avoid random pixel values.

**Vertical rhythm**: Consistent spacing between sections and elements.

**Component padding**: Inner padding proportional to component size.

**Section spacing**: Larger spacing between sections than within sections.

**Compact vs spacious**: Dashboards can be denser. Marketing pages need more breathing room.

## Icons and Imagery

**Icon system**: Consistent style (outline vs filled). Same weight across icons.

**Icon size**: 16px, 20px, 24px standard sizes. Align to grid.

**Decorative images**: High quality, consistent style (photography vs illustration).

**Image optimization**: WebP format, appropriate sizes, lazy loading.

**Illustrations**: Use to explain complex concepts or add personality.

**Empty states**: Use illustrations to make empty states friendly.

## Accessibility

**Color contrast**: WCAG AA minimum. Use contrast checker tools.

**Touch targets**: Minimum 44x44px for interactive elements.

**Focus states**: Visible keyboard focus indicators on all interactive elements.

**Text scaling**: Design supports 200% text zoom without breaking.

**Semantic HTML**: Proper heading hierarchy, landmarks, roles.

**Alt text**: Meaningful descriptions for images. Decorative images: alt="".

## Performance Considerations

**Image optimization**: Next-gen formats (WebP), appropriate sizes, lazy loading.

**Font loading**: Subset fonts, preload critical fonts, font-display: swap.

**Critical CSS**: Inline critical CSS for above-fold content.

**Animations**: Respect prefers-reduced-motion. Keep animations subtle and fast.

**Layout shifts**: Reserve space for images and dynamic content. Avoid CLS.

## Common Patterns

**Hero section**: Large heading, subheading, CTA, background image/gradient.

**Feature grid**: 2-3 columns on desktop, icon + title + description cards.

**Stats section**: Large numbers with labels, often in 3-4 column grid.

**Testimonials**: Quote, author photo, name, title. Carousel or grid.

**Pricing table**: Side-by-side comparison, highlight recommended plan.

**Dashboard overview**: KPI cards at top, charts below, recent activity list.

**Table with actions**: Data table with row actions (view, edit, delete).

## Design Principles

**Consistency**: Reuse patterns, components, spacing. Build a design system.

**Simplicity**: Remove unnecessary elements. Every element should serve a purpose.

**Clarity**: Clear labels, obvious actions, unambiguous feedback.

**Progressive disclosure**: Show essential information first, details on demand.

**Feedback**: Loading states, success/error messages, hover/active states.

**Forgiveness**: Confirmations for destructive actions, easy undo when possible.

## Anti-Patterns

**Avoid**:
- Centered paragraphs (hard to read)
- All caps body text (reduces readability)
- Low contrast for aesthetic over readability
- Carousel for critical content (often ignored)
- Modal on page load (annoying)
- Auto-playing video with sound
- Hiding navigation on scroll (frustrating)
- Too many font weights/sizes (inconsistent)

**Do**:
- Left-align body text
- Use sentence case
- Prioritize readability
- Static content for important messages
- Earn modal interruptions
- User-initiated media playback
- Persistent or show-on-scroll-up navigation
- Systematic type scale
