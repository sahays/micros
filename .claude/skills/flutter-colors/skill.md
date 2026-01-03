---
name: flutter-colors
description: Implement color systems and themes in Flutter using Material Design and custom palettes. Use when building color schemes, theming, dark mode, or brand-specific color systems.
---

# Flutter Colors

## Core Principles

**Theme-based colors**: Define colors in `ThemeData` rather than hardcoding. Enables dark mode, brand switching, and consistent styling.

**Material Design palette**: Use `ColorScheme` for semantic colors (primary, secondary, error, surface, etc.). Provides accessibility-tested contrast ratios.

**Context-aware access**: Get theme colors via `Theme.of(context).colorScheme` for dynamic updates when theme changes.

## Color Scheme Setup

**Define ColorScheme in MaterialApp**: Use `ColorScheme.fromSeed` to generate harmonious palette from single seed color, or define custom `ColorScheme` manually for full control.

**Light and dark themes**: Set both `theme` and `darkTheme` in MaterialApp, changing only `brightness` property.

## Accessing Theme Colors

**Get via context**: Access colors through `Theme.of(context).colorScheme`.

**Semantic naming**: Use `primary`, `secondary`, `tertiary`, `error`, `surface` for consistent meaning.

**On-colors**: Use `onPrimary`, `onSecondary`, `onSurface` for text/icons on colored backgrounds to ensure proper contrast.

**Surface variants**: `surface`, `surfaceVariant` for cards, dialogs, elevated surfaces.

## Custom Colors

**Extend ColorScheme**: Create extension on `ColorScheme` for brand-specific colors (success, warning, info).

**Brightness-aware**: Return different color values based on `brightness` property for light/dark mode support.

**Access pattern**: Use `Theme.of(context).colorScheme.customColor` to access extended colors.

## Dark Mode

**Auto-detect system**: Set `themeMode: ThemeMode.system` to respect device preference.

**Manual toggle**: Use `ThemeMode.light` or `ThemeMode.dark` for user-controlled theme switching.

**Consistent colors**: Use same seed color for both themes, only change brightness.

## Color Utilities

**Opacity**: Use `withOpacity(0.5)` for transparency adjustments.

**Lighten/darken**: Use `Color.lerp` to interpolate between color and white/black.

**Alpha channels**: Use `withAlpha(0-255)` for specific alpha values.

**Disabled states**: Apply `onSurface.withOpacity(0.38)` for disabled text/icons.

## Accessibility

**Contrast ratios**: ColorScheme provides WCAG-compliant contrast (4.5:1 for text).

**On-colors**: Always use `onPrimary` on `primary`, `onSurface` on `surface`, etc.

**Avoid color-only indicators**: Combine with icons or text for color-blind users.

## Performance

**Const colors**: Use `const Color(0xFFRRGGBB)` for compile-time constants.

**Cache theme**: Access `Theme.of(context)` once per build, store in variable.

**Avoid rebuilds**: Don't wrap entire tree in `Theme` widgets unnecessarily.

## Common Patterns

**Surface tints**: Use `surfaceTint` for elevation-based color changes in Material 3.

**Containers**: Use `primaryContainer` with `onPrimaryContainer` for filled containers.

**Outlined elements**: Use `outline` and `outlineVariant` for borders.

**State layers**: Apply opacity (8%, 12%, 16%) to create hover/focus/press states.
