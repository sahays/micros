---
name: tailwind-colors
description:
  Design and implement comprehensive color systems with Tailwind CSS. Use when defining brand colors, semantic tokens,
  dark mode variants, and maintaining color consistency across projects.
---

# Tailwind Colors

## Color System Design

**Theme-first**: Define complete color palette in config before building components.

**Brand palette**: Create 50-950 shade scale for each brand color (primary, secondary, accent).

**Semantic colors**: Define by purpose - success (green), error (red), warning (yellow), info (blue).

**Neutral grays**: 50-950 scale from near-white to near-black. Foundation for text, borders, backgrounds.

## Color Scale Structure

**50-950 shades**: Consistent lightness progression across all colors.

**50**: Lightest tint, subtle backgrounds
**100-200**: Very light, hover states, disabled backgrounds
**300-400**: Light, borders, muted text
**500**: Base color, primary usage
**600-700**: Darker, hover states, emphasis
**800-900**: Very dark, high contrast text
**950**: Darkest, maximum contrast

**Consistency**: All color families should have similar lightness at each level.

## Custom Color Palettes

**Define in config**: `theme.extend.colors` for brand colors.

**Color naming**: Use semantic names (brand, accent) not generic (blue, purple).

**Multiple brands**: Support multiple color schemes with prefixes (brand-primary, brand-secondary).

**Color generation tools**: Use tools to generate consistent 50-950 scales from base color.

## Semantic Color Tokens

**Purpose-based naming**:
- `bg-success`: Success state backgrounds
- `text-error`: Error message text
- `border-warning`: Warning state borders
- `bg-info`: Info message backgrounds

**Benefits**: Change color meaning globally, maintain consistency, clearer intent in code.

## Text Colors

**Hierarchy with grays**:
- `text-gray-900`: Primary text, headings
- `text-gray-700`: Secondary text, body
- `text-gray-500`: Tertiary text, captions
- `text-gray-400`: Disabled text, placeholders

**Dark mode**: Reverse hierarchy - lighter grays for dark backgrounds.

**Brand text**: Use sparingly - links, CTAs, emphasis.

## Background Colors

**Layering strategy**:
- `bg-white` / `bg-gray-950`: Base layer
- `bg-gray-50` / `bg-gray-900`: Raised surface
- `bg-gray-100` / `bg-gray-800`: Elevated surface
- `bg-white` / `bg-gray-700`: Overlay/modal

**Subtle differences**: Small shade differences create depth without heavy shadows.

**Dark mode**: Lighter backgrounds for elevated surfaces (reverse of light mode).

## Border Colors

**Subtle borders**: `border-gray-200` in light mode, `border-gray-700` in dark mode.

**Interactive borders**: `border-gray-300` → `border-blue-500` on focus.

**Error states**: `border-red-500` for validation errors.

**Dividers**: `divide-gray-200` for list/table dividers.

## Opacity Utilities

**Syntax**: `bg-blue-500/20` for 20% opacity.

**Overlay backgrounds**: `bg-black/50` for modal overlays.

**Glass effects**: `bg-white/10` with backdrop-blur.

**Hover states**: Increase opacity on hover - `hover:bg-blue-500/30`.

## Dark Mode Strategy

**Class-based**: Add `dark` class to html/root element.

**Color inversions**:
- Light: `bg-white text-gray-900`
- Dark: `dark:bg-gray-900 dark:text-white`

**Not just inversion**: Design dark mode colors intentionally, don't just invert.

**Desaturate in dark**: Bright colors are harsh in dark mode. Use muted versions.

**Test both modes**: Design light and dark together, not sequentially.

## Color Contrast

**WCAG AA minimum**: 4.5:1 for normal text, 3:1 for large text.

**Tools**: Use contrast checkers before committing to color choices.

**Text on brand colors**: Ensure sufficient contrast - `text-white` on `bg-blue-600` usually safe.

**Gray text**: `text-gray-600` on `bg-white` meets minimum. `text-gray-500` borderline.

## Gradient Colors

**Gradient-ready palette**: Design colors that work well together in gradients.

**Color harmony**: Use adjacent or complementary colors for smooth gradients.

**Reference**: See tailwind-gradients skill for gradient implementation.

## State Colors

**Interactive states**:
- Default: Base color
- Hover: Darker shade (100 higher on scale)
- Active: Even darker (200 higher)
- Focus: Add ring in same color family
- Disabled: Muted gray

**Form states**:
- Valid: Success color
- Invalid: Error color
- Warning: Warning color

## Color Consistency

**Limit palette**: Use 2-3 brand colors max. Too many colors = inconsistency.

**Systematic usage**: Document which colors for which purposes.

**Component variants**: Use same color scale for component variants.

**Avoid random colors**: Every color should come from theme, not arbitrary values.

## Best Practices

**Do**:
- Define complete palette upfront
- Use semantic color names
- Test color contrast
- Design dark mode intentionally
- Use opacity for variations
- Stick to defined palette

**Avoid**:
- Adding colors ad-hoc
- Using arbitrary color values
- Ignoring dark mode
- Poor contrast ratios
- Too many brand colors
- Inconsistent color usage
