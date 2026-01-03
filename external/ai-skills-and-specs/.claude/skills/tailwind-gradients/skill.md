---
name: tailwind-gradients
description:
  Create stunning gradient effects with Tailwind CSS for backgrounds, text, borders, and animations. Use when
  implementing modern gradient designs for hero sections, cards, buttons, and UI elements.
---

# Tailwind Gradients

## Gradient Directions

**Linear gradients**: `bg-gradient-to-{direction}`

**Directions**:
- `to-r`: Left to right →
- `to-l`: Right to left ←
- `to-t`: Bottom to top ↑
- `to-b`: Top to bottom ↓
- `to-br`: Top-left to bottom-right ↘
- `to-bl`: Top-right to bottom-left ↙
- `to-tr`: Bottom-left to top-right ↗
- `to-tl`: Bottom-right to top-left ↖

**Most common**: `to-r` (horizontal), `to-b` (vertical), `to-br` (diagonal).

## Color Stops

**Two-color gradient**: `from-{color}` and `to-{color}`

**Three-color gradient**: Add `via-{color}` for middle stop

**Example**: `bg-gradient-to-r from-blue-500 via-purple-500 to-pink-500`

**Stop positioning**: Tailwind auto-distributes stops evenly.

## Background Gradients

**Hero backgrounds**: `bg-gradient-to-br from-blue-600 via-purple-600 to-pink-600`

**Subtle sections**: `bg-gradient-to-r from-gray-50 to-gray-100`

**Dark mode**: `dark:from-gray-900 dark:via-gray-800 dark:to-gray-900`

**Overlay gradients**: `bg-gradient-to-t from-black/80 to-transparent` over images

**Card accents**: Subtle gradient backgrounds for elevated cards

## Text Gradients

**Pattern**: Use `bg-gradient-*`, `bg-clip-text`, `text-transparent`

**Example**: `bg-gradient-to-r from-blue-500 to-purple-600 bg-clip-text text-transparent`

**Animated text**: Combine with `animate-gradient` for color shift effect

**Headings**: Large gradient headings for hero sections

**Logo text**: Gradient brand names for visual interest

## Border Gradients

**Gradient border technique**: Use padding with gradient background

```
relative p-[1px] bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg
```

Then inner element with `bg-white dark:bg-gray-900 rounded-lg`

**Card borders**: Gradient borders for premium feel

**Button outlines**: Animated gradient borders on hover

## Button Gradients

**Solid gradient button**: `bg-gradient-to-r from-blue-600 to-purple-600`

**Hover transition**: `hover:from-blue-700 hover:to-purple-700`

**With animation**: Add `hover:scale-105 transition-all`

**Ghost button with gradient**: Border gradient + transparent background

## Gradient Overlays

**Image overlays**: Layer gradient over image for text readability

**Pattern**: `bg-gradient-to-t from-black/60 to-transparent`

**Use cases**: Hero images with text, card images with captions

**Multiple overlays**: Stack gradients for complex effects

## Animated Gradients

**Moving gradient**: Use `bg-[length:200%_200%] animate-gradient-move`

**Define in config**: Create `gradient-move` keyframe animation

**Shimmer effect**: Diagonal gradient animation for loading states

**Hover gradients**: Transition gradient colors on hover

## Theme Gradient Combinations

**Define in config**: Create reusable gradient combinations

**Brand gradients**: Primary, secondary, accent gradient presets

**Consistent usage**: Use same gradient combinations throughout app

**Example names**: `gradient-brand`, `gradient-success`, `gradient-sunset`

## Gradient Patterns

**Hero section**: Large diagonal gradient background

**Feature cards**: Subtle top-to-bottom gradient

**Pricing cards**: Gradient accent on featured plan

**Stats section**: Gradient backgrounds for metric cards

**Footer**: Subtle gradient for visual interest

**CTA sections**: Bold gradient backgrounds for calls-to-action

## Radial Gradients

**Not built-in**: Use arbitrary values or custom utilities

**Add to config**: Define radial gradients as custom utilities

**Use case**: Spotlight effects, hero backgrounds

## Conic Gradients

**Not built-in**: Use arbitrary values or custom utilities

**Add to config**: For color wheel effects

**Rare usage**: Specialized design needs only

## Gradient + Glassmorphism

**Combine**: Gradient background + backdrop blur

**Pattern**: `bg-gradient-to-br from-blue-500/20 to-purple-500/20 backdrop-blur-md`

**Modern look**: Translucent gradient panels

## Performance Considerations

**Simple gradients**: Two-color gradients perform best

**Complex gradients**: Multiple stops impact performance slightly

**Animated gradients**: Use sparingly, can be CPU intensive

**Mobile**: Test gradient performance on mobile devices

## Accessibility

**Text on gradients**: Ensure contrast on all parts of gradient

**Critical text**: Avoid text directly on complex gradients

**Fallback colors**: Gradient-unsupported browsers show first color

## Dark Mode Gradients

**Muted colors**: Desaturate gradient colors in dark mode

**Different gradients**: Use completely different gradients for dark mode

**Example**: Light mode bright blue→purple, dark mode dark blue→dark purple

## Common Combinations

**Blue to purple**: Classic, modern, tech-focused

**Pink to orange**: Warm, energetic, creative

**Green to blue**: Natural, calm, trustworthy

**Purple to pink**: Premium, luxury, creative

**Gray to gray**: Subtle, professional, minimal

## Anti-Patterns

**Avoid**:
- Too many gradient stops (over 3)
- High contrast gradients (jarring)
- Gradients on all elements (overwhelming)
- Poor text contrast on gradients
- Ignoring dark mode gradient variants
- Random gradient directions

**Do**:
- Use 2-3 colors maximum
- Choose harmonious color combinations
- Use gradients strategically for emphasis
- Test text contrast thoroughly
- Design dark mode gradients intentionally
- Be consistent with gradient directions
