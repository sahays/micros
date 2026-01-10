---
name: web-design
description:
  Create distinctive, production-grade web interfaces with high design quality. Use when building landing pages, dashboards,
  feature pages, or any web UI requiring both creative aesthetics and practical design patterns. Generates polished designs
  that avoid generic AI aesthetics while maintaining accessibility and usability.
---

- Design Thinking
  - Before coding, understand context and commit to a BOLD aesthetic direction
  - Purpose: what problem does this interface solve, who uses it, what is the context
  - Tone: pick a clear direction - brutally minimal, maximalist, retro-futuristic, organic/natural, luxury/refined, playful, editorial/magazine, brutalist/raw, art deco/geometric, soft/pastel, industrial/utilitarian
  - Constraints: technical requirements (framework, performance, accessibility, browser support)
  - Differentiation: what makes this memorable and unforgettable, what's the one thing someone will remember
  - Intentionality: both bold maximalism and refined minimalism work - the key is intentionality not intensity
  - Execute with precision: choose a clear conceptual direction and implement it meticulously in every detail

- Layout Systems
  - Grid-based layouts: use CSS Grid for complex layouts, Flexbox for component-level
  - 12-column grid: standard for responsive design, adapt columns based on breakpoints
  - Container widths: max-width for readability, 1200px-1400px for desktop, fluid below
  - Spacing scale: consistent spacing system, use 4px or 8px base unit (4, 8, 16, 24, 32, 48, 64)
  - Whitespace: more whitespace = cleaner, more premium feel, don't cram content

- Responsive Breakpoints
  - Mobile-first: design for mobile, enhance for larger screens
  - Standard breakpoints: Mobile (< 640px), Tablet (640px - 1024px), Desktop (> 1024px), Large desktop (> 1440px)
  - Adapt patterns: stack on mobile, side-by-side on desktop, hide/show elements appropriately

- Typography
  - Font selection: choose fonts that are beautiful, unique, and interesting - avoid generic fonts like Arial, Inter, Roboto, system fonts
  - Distinctive choices: use unexpected, characterful font choices that elevate the aesthetic - pair a distinctive display font with a refined body font
  - Font pairing: maximum 2-3 fonts (one for headings, one for body, optionally one for code/data)
  - Scale: use type scale for hierarchy (1.125, 1.25, or 1.333 ratio)
  - Line height: 1.5-1.6 for body text, 1.2-1.3 for headings
  - Line length: 60-80 characters per line for optimal readability
  - Font weights: use 2-3 weights max (Regular 400, Medium 500, Bold 700)
  - Hierarchy: clear size and weight differences between heading levels
  - Context-specific: typography should match the overall aesthetic direction and purpose

- Color Systems
  - Cohesive aesthetic: commit to a clear color direction that matches the overall tone and purpose
  - CSS variables: use for consistency and easy theming
  - Dominant colors with sharp accents: outperform timid, evenly-distributed palettes
  - Avoid cliched schemes: particularly purple gradients on white backgrounds or generic AI color choices
  - Primary color: brand color, use for CTAs, links, key actions
  - Neutral grays: 7-10 shades from white to black for text, borders, backgrounds
  - Semantic colors: Success (green), Error (red), Warning (yellow), Info (blue)
  - Background layers: subtle differences between surfaces (base, raised, overlay)
  - Text contrast: WCAG AA minimum (4.5:1 for normal text, 3:1 for large text)
  - Color palette size: 5-7 primary shades, 7-10 gray shades, 5 shades per semantic color
  - Context matters: vary between light and dark themes based on purpose, not defaults

- Dark Mode
  - Design both modes together: not just inverted colors
  - Background layers: lighter backgrounds for elevated surfaces in dark mode (reverse of light mode)
  - Reduce saturation: bright colors are harsh in dark mode, use desaturated versions
  - Text colors: use lighter grays not pure white, reduces eye strain
  - Toggle placement: user preference setting, not contextual, remember user choice

- Motion and Animation
  - High-impact moments: one well-orchestrated page load with staggered reveals creates more delight than scattered micro-interactions
  - CSS-first: prioritize CSS animations for HTML projects (transitions, keyframes, animation-delay)
  - Motion libraries: use Framer Motion for React, GSAP for complex sequences
  - Staggered reveals: use animation-delay to create sequential entrance effects
  - Scroll-triggered: animations that trigger on scroll or visibility
  - Hover states: micro-interactions that surprise and delight
  - Respect accessibility: honor prefers-reduced-motion, keep animations fast (200-400ms)
  - Purposeful motion: every animation should serve the aesthetic vision or improve UX
  - Match complexity: maximalist designs need elaborate animations, minimalist designs need subtle, refined motion

- Spatial Composition and Layout
  - Beyond grid: use unexpected layouts, asymmetry, overlapping elements, diagonal flow
  - Grid-breaking elements: strategically break the grid for visual interest
  - Negative space: generous whitespace OR controlled density - both can work with intentionality
  - Layering: use z-index, overlaps, and depth to create visual interest
  - Flow: guide the eye through the composition with intentional placement
  - Asymmetry: balanced asymmetry often more interesting than perfect symmetry
  - Scale variation: dramatic scale differences create hierarchy and interest

- Backgrounds and Visual Details
  - Create atmosphere: use backgrounds to create depth and mood, not just solid colors
  - Contextual effects: effects and textures that match the overall aesthetic direction
  - Gradient meshes: complex, multi-point gradients for organic feel
  - Noise textures: add grain and texture for depth and sophistication
  - Geometric patterns: repeating shapes, grids, lines for structure
  - Layered transparencies: overlay multiple semi-transparent elements for depth
  - Dramatic shadows: use shadows creatively, not just for elevation
  - Decorative borders: custom borders, outlines, frames that enhance the aesthetic
  - Custom cursors: context-specific cursors that enhance the experience
  - Grain overlays: subtle film grain for analog warmth

- Landing Page Design
  - Hero section: full viewport height or 60-70%, clear value proposition and CTA
  - Above the fold: most important message and action visible without scrolling
  - Single CTA focus: one primary action per section
  - Social proof: testimonials, logos, metrics, build trust early
  - Visual hierarchy: guide eye through page with size, color, spacing
  - Scannable content: short paragraphs, bullet points, clear headings
  - Section rhythm: alternate content/image sides, vary section backgrounds subtly

- Feature Pages
  - Feature showcase: lead with benefit not feature list, show don't just tell
  - Screenshots/demos: high-quality visuals, use annotations to highlight key points
  - Progressive disclosure: don't dump all information at once, reveal details on demand
  - Comparison tables: show how features compare to alternatives
  - CTAs throughout: multiple opportunities to convert as users scroll

- Dashboard Design
  - Information hierarchy: most important metrics at top, drill-down details below
  - Card-based layout: group related information in cards, clear visual boundaries
  - Consistent card anatomy: title, metric/chart, action (optional), timestamp/meta
  - Scannable metrics: large numbers, clear labels, trend indicators (up/down arrows)
  - Data density: balance information with whitespace, don't overwhelm
  - Color coding: use sparingly for status/severity, too much color is noise
  - Loading states: show skeleton screens or progressive loading, not spinners alone

- Navigation Patterns
  - Landing pages: minimal top nav (logo, links, CTA), sticky on scroll optional
  - Dashboards: side navigation for many sections, top nav for global actions
  - Breadcrumbs: essential for deep hierarchies, show path to current location
  - Mobile navigation: hamburger menu or bottom tab bar, clear accessible toggle
  - Active states: clearly indicate current page/section in navigation
  - Search: include search for content-heavy sites and dashboards

- Visual Hierarchy
  - Size: larger elements draw attention, use for primary actions and key content
  - Color: bright/saturated colors stand out, use for important elements only
  - Contrast: high contrast elements get noticed first
  - Spacing: isolated elements get more attention, group related items
  - Typography: bold, uppercase, or different font for emphasis
  - F-pattern: users scan in F-pattern, place important content top-left

- Component Design
  - Buttons: Primary (solid background, brand color), Secondary (outline or muted), Tertiary (text only), size hierarchy (large for primary, medium for secondary)
  - Cards: subtle shadow or border for definition, consistent padding (16-24px), hover state for interactive cards, group related content
  - Tables (dashboards): zebra striping for long tables, sticky headers for scrolling, row hover state, responsive (stack or scroll on mobile)
  - Badges/tags: small, rounded, subtle backgrounds, readable text, use for status/categories/counts

- Data Visualization (Dashboards)
  - Chart selection: bar for comparisons, line for trends, pie for proportions (use sparingly)
  - Color: use brand colors for primary data, consistent color meaning across charts
  - Labels: direct labeling on charts when possible, reduce reliance on legends
  - Axis: start at zero for bar charts, appropriate scale for line charts
  - Interactivity: hover tooltips for details, click for drill-down
  - Empty states: show helpful message when no data, not blank space

- Spacing and Rhythm
  - Consistent spacing: use spacing scale, avoid random pixel values
  - Vertical rhythm: consistent spacing between sections and elements
  - Component padding: inner padding proportional to component size
  - Section spacing: larger spacing between sections than within sections
  - Compact vs spacious: dashboards can be denser, marketing pages need more breathing room

- Icons and Imagery
  - Icon system: consistent style (outline vs filled), same weight across icons
  - Icon size: 16px, 20px, 24px standard sizes, align to grid
  - Decorative images: high quality, consistent style (photography vs illustration)
  - Image optimization: WebP format, appropriate sizes, lazy loading
  - Illustrations: use to explain complex concepts or add personality
  - Empty states: use illustrations to make empty states friendly

- Accessibility
  - Color contrast: WCAG AA minimum, use contrast checker tools
  - Touch targets: minimum 44x44px for interactive elements
  - Focus states: visible keyboard focus indicators on all interactive elements
  - Text scaling: design supports 200% text zoom without breaking
  - Semantic HTML: proper heading hierarchy, landmarks, roles
  - Alt text: meaningful descriptions for images, decorative images alt=""

- Performance Considerations
  - Image optimization: next-gen formats (WebP), appropriate sizes, lazy loading
  - Font loading: subset fonts, preload critical fonts, font-display: swap
  - Critical CSS: inline critical CSS for above-fold content
  - Animations: respect prefers-reduced-motion, keep animations subtle and fast
  - Layout shifts: reserve space for images and dynamic content, avoid CLS

- Common Patterns
  - Hero section: large heading, subheading, CTA, background image/gradient
  - Feature grid: 2-3 columns on desktop, icon + title + description cards
  - Stats section: large numbers with labels, often in 3-4 column grid
  - Testimonials: quote, author photo, name, title, carousel or grid
  - Pricing table: side-by-side comparison, highlight recommended plan
  - Dashboard overview: KPI cards at top, charts below, recent activity list
  - Table with actions: data table with row actions (view, edit, delete)

- Design Principles
  - Consistency: reuse patterns, components, spacing, build a design system
  - Simplicity: remove unnecessary elements, every element should serve a purpose
  - Clarity: clear labels, obvious actions, unambiguous feedback
  - Progressive disclosure: show essential information first, details on demand
  - Feedback: loading states, success/error messages, hover/active states
  - Forgiveness: confirmations for destructive actions, easy undo when possible

- Anti-Patterns to Avoid
  - Never use generic AI aesthetics: avoid Inter/Roboto/Arial, purple gradients on white, predictable layouts, cookie-cutter designs
  - Never use overused font families: Inter, Roboto, Arial, system fonts unless specifically appropriate
  - Never use cliched color schemes: particularly purple gradients, generic SaaS blue, default Material Design colors
  - Never create predictable layouts: every design should feel custom and intentional for its context
  - Never make all designs the same: vary themes (light/dark), fonts, aesthetics between projects
  - Never center paragraphs (hard to read)
  - Never use all caps body text (reduces readability)
  - Never sacrifice contrast for aesthetics
  - Never use carousel for critical content (often ignored)
  - Never show modal on page load (annoying)
  - Never auto-play video with sound
  - Never hide navigation on scroll (frustrating)
  - Never use too many font weights/sizes (inconsistent)
  - Never mismatch complexity: maximalist designs need elaborate implementation, minimalist designs need precision and restraint
  - Always left-align body text
  - Always use sentence case
  - Always prioritize readability
  - Always use static content for important messages
  - Always earn modal interruptions
  - Always require user-initiated media playback
  - Always use persistent or show-on-scroll-up navigation
  - Always use systematic type scale
  - Always interpret creatively: make unexpected choices that feel genuinely designed for the context
  - Always commit fully: execute the aesthetic vision with precision in every detail
  - Always match implementation to vision: elegant designs need elegant code, complex designs need rich implementation
