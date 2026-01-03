---
name: tailwind-typography
description:
  Create clear, readable typography systems with Tailwind CSS. Use when defining font scales, hierarchies, and text
  styling for consistent, accessible typography across applications.
---

# Tailwind Typography

## Typography Scale

**Font sizes**: Systematic scale for consistency

**Base scale**:
- `text-xs`: 0.75rem (12px) - Tiny text, captions
- `text-sm`: 0.875rem (14px) - Small text, labels
- `text-base`: 1rem (16px) - Body text default
- `text-lg`: 1.125rem (18px) - Emphasized body
- `text-xl`: 1.25rem (20px) - Small headings
- `text-2xl`: 1.5rem (24px) - H4 headings
- `text-3xl`: 1.875rem (30px) - H3 headings
- `text-4xl`: 2.25rem (36px) - H2 headings
- `text-5xl`: 3rem (48px) - H1 headings
- `text-6xl`: 3.75rem (60px) - Large display
- `text-7xl`: 4.5rem (72px) - Hero headings
- `text-8xl`: 6rem (96px) - Extra large display
- `text-9xl`: 8rem (128px) - Massive display

**Body text**: `text-base` (16px) is optimal for readability

**Mobile**: Often use smaller sizes, scale up on larger screens

## Font Families

**Define in config**: Add custom fonts to theme

**System fonts**: `font-sans`, `font-serif`, `font-mono`

**Custom fonts**: Import Google Fonts or local fonts

**Headings vs body**: Different font families for hierarchy

**Fallbacks**: Always include system font fallbacks

## Font Weights

**Scale**: 100-900 in increments of 100

**Common weights**:
- `font-thin`: 100 - Rarely used
- `font-extralight`: 200 - Very light
- `font-light`: 300 - Light
- `font-normal`: 400 - Body text default
- `font-medium`: 500 - Slightly emphasized
- `font-semibold`: 600 - Strong emphasis
- `font-bold`: 700 - Headings, important text
- `font-extrabold`: 800 - Very strong
- `font-black`: 900 - Maximum weight

**Limit weights**: Use 2-3 weights max (e.g., 400, 500, 700)

**Headings**: `font-semibold` or `font-bold`

**Body**: `font-normal`

**Emphasis**: `font-medium` or `font-semibold`

## Line Height

**Spacing between lines**: Critical for readability

**Scale**:
- `leading-none`: 1 - No extra space (large headings)
- `leading-tight`: 1.25 - Tight (headings)
- `leading-snug`: 1.375 - Slightly tight
- `leading-normal`: 1.5 - Body text default
- `leading-relaxed`: 1.625 - Comfortable reading
- `leading-loose`: 2 - Very spacious

**Body text**: `leading-normal` (1.5) or `leading-relaxed` (1.625)

**Headings**: `leading-tight` (1.25) or `leading-snug` (1.375)

**Responsive**: Adjust line height at different breakpoints if needed

## Letter Spacing

**Tracking**: Space between characters

**Scale**:
- `tracking-tighter`: -0.05em - Very tight
- `tracking-tight`: -0.025em - Tight (large headings)
- `tracking-normal`: 0 - Default
- `tracking-wide`: 0.025em - Wide (small caps)
- `tracking-wider`: 0.05em - Wider
- `tracking-widest`: 0.1em - Widest (all caps)

**Headings**: `tracking-tight` for large sizes

**All caps**: `tracking-wide` or `tracking-wider`

**Body**: `tracking-normal` (don't adjust)

## Text Colors

**Hierarchy with grays**:
- `text-gray-900`: Primary text (headings, important body)
- `text-gray-700`: Secondary text (normal body)
- `text-gray-500`: Tertiary text (captions, meta)
- `text-gray-400`: Disabled text

**Dark mode**: Use lighter grays (`text-gray-100`, `text-gray-300`)

**Brand colors**: Links, CTAs, emphasis

**Semantic colors**: Success, error, warning text

## Text Alignment

**Align**: `text-left`, `text-center`, `text-right`, `text-justify`

**Default**: `text-left` for most text (English)

**Center**: Headings, hero sections sparingly

**Avoid justify**: Can create awkward spacing

**Responsive**: Change alignment at breakpoints

## Text Transform

**Case**: `uppercase`, `lowercase`, `capitalize`, `normal-case`

**Uppercase**: Labels, small headings (use with `tracking-wide`)

**Capitalize**: Titles, names

**Avoid**: All caps for long text (reduces readability)

## Text Decoration

**Underline**: `underline` - Links default

**Line through**: `line-through` - Strikethrough

**No underline**: `no-underline` - Remove default underline

**Decoration color**: `decoration-blue-500`

**Decoration style**: `decoration-solid`, `decoration-dotted`, `decoration-dashed`

**Decoration thickness**: `decoration-1`, `decoration-2`

## Heading Hierarchy

**Consistent scale**: Clear size difference between levels

**Pattern**:
- H1: `text-4xl md:text-5xl font-bold`
- H2: `text-3xl md:text-4xl font-bold`
- H3: `text-2xl md:text-3xl font-semibold`
- H4: `text-xl md:text-2xl font-semibold`
- H5: `text-lg font-semibold`
- H6: `text-base font-semibold`

**Responsive**: Larger sizes on desktop, smaller on mobile

## Body Text Patterns

**Standard body**: `text-base text-gray-700 leading-relaxed`

**Large body**: `text-lg text-gray-700 leading-relaxed` - Landing pages

**Small body**: `text-sm text-gray-600 leading-normal` - Dense UIs

**Caption**: `text-xs text-gray-500` - Metadata, timestamps

## Link Styling

**Default**: `text-blue-600 hover:text-blue-700 underline`

**No underline**: `text-blue-600 hover:underline` - Cleaner, hover reveals

**Bold links**: `font-medium` or `font-semibold` for emphasis

**Visited**: Consider `visited:text-purple-600` for distinction

## List Styling

**Disc/decimal**: `list-disc`, `list-decimal`

**Position**: `list-inside`, `list-outside`

**Custom markers**: Use `before:` pseudo-element

**Spacing**: `space-y-2` between items

## Text Truncation

**Single line**: `truncate` - Adds ellipsis

**Multiple lines**: `line-clamp-{n}` - Clamps to n lines

**Overflow**: `overflow-hidden` with truncate

## Responsive Typography

**Mobile-first**: Start with smaller sizes

**Breakpoint scaling**: `text-lg md:text-xl lg:text-2xl`

**Hero headings**: Significant size increase on desktop

**Body text**: Usually consistent, sometimes scale up on desktop

## Dark Mode Typography

**Text colors**: Lighter grays on dark backgrounds

**Contrast**: Ensure readability in both modes

**Pattern**: `text-gray-900 dark:text-gray-100`

**Avoid**: Pure white text (harsh), use `text-gray-100` instead

## Prose Typography

**@tailwindcss/typography plugin**: For rich text content

**Prose classes**: Auto-styles markdown/HTML content

**Customization**: Extend prose styles in config

**Use case**: Blog posts, documentation, CMScontent

## Accessibility

**Minimum size**: 16px (1rem) for body text

**Contrast**: WCAG AA minimum (4.5:1 for normal text)

**Line length**: 60-80 characters per line optimal

**Line height**: 1.5 minimum for body text

**Scalability**: Support 200% text zoom

## Performance

**Font loading**: Preload critical fonts

**Font display**: Use `font-display: swap` in @font-face

**Subset fonts**: Only include needed characters

**System fonts**: Fastest loading option

## Custom Font Scale

**Extend in config**: Add custom sizes between defaults

**Maintain ratio**: Keep consistent scale relationship

**Tools**: Use modular scale calculators (1.2, 1.25, 1.333 ratio)

## Text Effects

**Text shadow**: `text-shadow-sm`, `text-shadow-md` (define in config)

**Gradient text**: See tailwind-gradients skill

**Glow effect**: Colored text shadow for neon effect

## Best Practices

**Do**:
- Use systematic scale
- Limit font weights (2-3)
- Set comfortable line height
- Left-align body text
- Test readability on devices
- Ensure sufficient contrast
- Use semantic heading levels

**Avoid**:
- Random font sizes
- Too many font weights
- Tight line height on body text
- Center-aligned paragraphs
- All caps for long text
- Low contrast text
- Skipping heading levels (H1→H3)

## Common Patterns

**Hero heading**: `text-5xl md:text-6xl font-bold text-gray-900 dark:text-white leading-tight`

**Section heading**: `text-3xl font-bold text-gray-900 dark:text-white`

**Card title**: `text-xl font-semibold text-gray-900 dark:text-white`

**Body text**: `text-base text-gray-700 dark:text-gray-300 leading-relaxed`

**Caption**: `text-sm text-gray-500 dark:text-gray-400`

**Link**: `text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300`
