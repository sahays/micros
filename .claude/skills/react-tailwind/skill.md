---
name: react-tailwind
description:
  Build modern UI with React and Tailwind CSS v4 for landing pages, feature pages, and admin dashboards. Use when
  implementing responsive layouts and component styling. References separate skills for colors, gradients, animations,
  and typography.
---

# React + Tailwind CSS

## Setup and Configuration

**Installation**: Use Tailwind v4 with Vite or Create React App.

**Theme-first approach**: Define comprehensive theme in config before building.

**JIT mode**: Always enabled. Generates only used styles.

**Related skills**: See tailwind-colors, tailwind-gradients, tailwind-animations, tailwind-typography for deep dives.

## Utility-First Approach

**Compose utilities**: Build components with utility classes, not custom CSS.

**Avoid custom CSS**: Use Tailwind utilities first. Extract to components if repeating.

**Class order**: Layout → spacing → sizing → colors → typography → effects.

**Use cn() helper**: Combine class names with conditional logic (from Shadcn utils).

## Component Patterns

**Extract reusable components**: Create React components for repeated utility combinations.

**Props for variants**: Control utilities via props (primary vs secondary).

**Wrapper components**: Semantic components wrapping Tailwind utilities.

## Responsive Design

**Mobile-first**: Base styles for mobile, breakpoints for larger screens.

**Breakpoints**: `sm:640px`, `md:768px`, `lg:1024px`, `xl:1280px`, `2xl:1536px`

**Container**: `container mx-auto px-4` for max-width layouts.

**Hide/show**: `hidden md:block` for responsive visibility.

## Layout Patterns

**Hero section**:

- `min-h-screen flex items-center justify-center`
- `bg-gradient-to-br from-blue-500 to-purple-600` (see tailwind-gradients)

**Feature grid**: `grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-8`

**Dashboard**:

- Sidebar: `fixed left-0 top-0 h-full w-64`
- Main: `ml-64 p-6`
- Responsive: `lg:ml-64` with mobile hamburger

**Sticky nav**: `sticky top-0 z-50 bg-white shadow-sm`

## Component Styling

**Buttons**:

- Base: `px-4 py-2 rounded-lg font-medium transition-colors duration-200`
- Primary: `bg-blue-500 text-white hover:bg-blue-600`
- With animation: `hover:scale-105 transition-all` (see tailwind-animations)

**Cards**:

- `bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700`
- Hover: `hover:shadow-md transition-shadow duration-200`

**Inputs**:

- `border border-gray-300 rounded-lg px-4 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`

## Dark Mode

**Class-based**: Add `dark` class to root element.

**Pattern**: `bg-white dark:bg-gray-900 text-gray-900 dark:text-white`

**Store preference**: localStorage with initial system preference detection.

**See**: tailwind-colors for complete dark mode color strategy.

## Spacing and Sizing

**Spacing scale**: 4px base - `p-1`(4px), `p-2`(8px), `p-4`(16px), `p-6`(24px), `p-8`(32px)

**Section padding**: `py-16` or `py-24` for vertical spacing.

**Gaps**: `gap-4`, `gap-6`, `gap-8` for flex/grid.

**Width constraints**: `max-w-7xl`, `max-w-4xl` for containers.

## Landing Page Components

**Hero**: Full viewport with gradient background, large heading, CTA.

**Feature grid**: 3-column grid with icons, benefits-focused copy.

**Stats**: Large numbers in 2-4 column grid.

**Testimonials**: Cards with avatar, quote, attribution.

**See**: web-design skill for component patterns.

## Dashboard Components

**Sidebar nav**: Fixed sidebar with hover states on items.

**Metrics cards**: Grid of cards with large numbers and trend indicators.

**Data tables**: Responsive table with hover rows and sticky headers.

**See**: web-design skill for dashboard patterns.

## Integration with Shadcn

**Shadcn uses Tailwind**: Pre-styled components with Tailwind classes.

**Customize via Tailwind**: Modify by editing utility classes.

**Theme matching**: Shadcn theme uses Tailwind config colors.

**Use together**: Shadcn for complex components, Tailwind for layouts.

## Dynamic Classes

**Avoid string concatenation**: `"text-" + color` doesn't work with JIT.

**Use full class names**: Write complete utilities.

**Conditional**: Use cn() helper or clsx for conditional classes.

**Data attributes**: `data-[state=active]:bg-blue-500` for state-based styling.

## Performance

**JIT mode**: Only generates used utilities.

**Content paths**: Specify template paths in config for accuracy.

**Avoid arbitrary values**: Use scale values for consistency and smaller bundles.

**Production build**: Automatically minified and optimized.

## Accessibility

**Focus states**: Always include `focus:ring-2` on interactive elements.

**Color contrast**: Use tailwind-colors skill for contrast guidelines.

**Screen reader**: `sr-only` for screen reader only content.

**Keyboard nav**: Ensure all interactive elements have visible focus.

## Common Patterns

**Centered container**: `container mx-auto px-4 max-w-7xl`

**Flex centering**: `flex items-center justify-center`

**Animated card**: See tailwind-animations for hover effects.

**Gradient text**: See tailwind-gradients for text gradient patterns.

**Glassmorphism**: `backdrop-blur-md bg-white/10 border border-white/20`

## Best Practices

**Do**:

- Use utility classes consistently
- Follow spacing scale
- Mobile-first responsive design
- Implement dark mode
- Include focus states
- Reference specific Tailwind skills for deep topics

**Avoid**:

- Custom CSS instead of utilities
- Arbitrary values everywhere
- Forgetting dark mode
- Missing focus states
- Not using responsive utilities

## Production Checklist

**Before deploying**:

- Dark mode implemented and tested
- All breakpoints verified
- Focus states on interactive elements
- Responsive design works on mobile
- Touch targets minimum 44x44px
- Color contrast meets WCAG AA (see tailwind-colors)
- Animations respect reduced motion (see tailwind-animations)

## Related Skills

**Deep dives**:

- **tailwind-colors**: Color systems, palettes, dark mode
- **tailwind-gradients**: Gradient effects and patterns
- **tailwind-animations**: Transitions and animations
- **tailwind-typography**: Font scales and text styling
- **web-design**: Component patterns and layouts
- **react-development**: React best practices
