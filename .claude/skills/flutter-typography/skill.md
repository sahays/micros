---
name: flutter-typography
description: Implement text styling and typography systems in Flutter using Material Design type scales and custom fonts. Use when building text themes, font hierarchies, or custom typography.
---

# Flutter Typography

## Core Principles

**Theme-based text styles**: Define typography in `TextTheme` for consistent styling across app.

**Material Design type scale**: Use semantic names (displayLarge, headlineMedium, bodySmall) rather than arbitrary sizes.

**Context-aware access**: Get text styles via `Theme.of(context).textTheme` for dynamic theme updates.

**Responsive sizing**: Consider MediaQuery or responsive packages for adaptive text scaling.

## Text Theme Setup

**Define in MaterialApp**: Create TextTheme in ThemeData with all required text styles.

**Material 3 scale**: displayLarge/Medium/Small, headlineLarge/Medium/Small, titleLarge/Medium/Small, bodyLarge/Medium/Small, labelLarge/Medium/Small.

**Auto-generated**: Use `Typography.material2021()` for platform-specific defaults.

**Custom sizing**: Override default sizes with project-specific font sizes and weights.

## Accessing Text Styles

**Get via context**: Access styles through `Theme.of(context).textTheme.styleName`.

**Null safety**: Use `?.` operator since textTheme styles can be null.

**Extend with copyWith**: Modify theme styles using `copyWith` rather than replacing entirely.

**Direct application**: Apply to Text widget via `style` parameter.

## Custom Fonts

**pubspec.yaml setup**: Declare font families with asset paths and weights.

**Multiple weights**: Define multiple font files for same family (regular, bold, etc.).

**Global application**: Set `fontFamily` in ThemeData to apply globally.

**TextTheme application**: Use Typography's `apply` method to apply font family to all styles.

**Google Fonts**: Use `google_fonts` package for easy integration without manual downloads.

## Font Weights

**Numeric weights**: `FontWeight.w100` through `FontWeight.w900` (increments of 100).

**Named weights**: `FontWeight.normal` (400), `FontWeight.bold` (700).

**Variable fonts**: Define multiple weight values in pubspec for font family.

**Weight selection**: Match font file weight declarations to ensure proper rendering.

## Text Styling

**Color inheritance**: Text inherits color from theme or set explicitly with `TextStyle.color`.

**Letter spacing**: Adjust tracking with `letterSpacing` property.

**Line height**: Control with `height` multiplier (1.5 = 1.5× font size).

**Decorations**: Apply underline, line-through, or overline with `decoration` property.

## Rich Text

**TextSpan for inline**: Create RichText with TextSpan children for mixed styling.

**Default style**: Set base style in root TextSpan, override in children.

**Text.rich shorthand**: Use Text.rich constructor for simpler rich text.

**WidgetSpan**: Embed widgets (icons, images) inline within text flow.

## Text Overflow

**Ellipsis**: Use `TextOverflow.ellipsis` for single-line truncation with "...".

**Fade**: Use `TextOverflow.fade` for gradient fade-out effect.

**Max lines**: Combine `maxLines` with overflow for multi-line clipping.

**Flexible wrapping**: Wrap Text in Expanded or Flexible to constrain width in flex layouts.

## Text Alignment

**Horizontal alignment**: Use `textAlign` - left, center, right, justify.

**Text direction**: Set `textDirection` (ltr/rtl) for internationalization support.

**Locale-specific**: Apply `locale` for language-specific typography rules.

## Responsive Typography

**Scale with screen**: Multiply base size by screen width percentage for responsive text.

**Text scale factor**: Respect user accessibility settings via `MediaQuery.textScaleFactor`.

**Clamp extremes**: Use min/max to prevent text from becoming too large or small.

**Breakpoint-based**: Define different text scales for mobile, tablet, desktop.

## Common Patterns

**Section headers**: Use `headlineMedium` or `titleLarge` with subtle color.

**Body content**: Apply `bodyLarge` for primary text, `bodyMedium` for secondary.

**Captions/labels**: Use `labelSmall` or `bodySmall` with reduced opacity.

**Button text**: Apply `labelLarge` (14px, medium weight) following Material guidelines.

**All caps labels**: Combine `toUpperCase()` with increased `letterSpacing` (1.5).

## Accessibility

**Minimum sizes**: Keep body text at 16px minimum, labels at 14px minimum.

**Contrast compliance**: Ensure WCAG AA (4.5:1 normal text, 3:1 large text >18px).

**Scale factor testing**: Test with `textScaleFactor: 2.0` for vision accessibility.

**Semantic labels**: Use Semantics widget for screen readers when text is decorative.

## Performance

**Const styles**: Mark TextStyle as const when properties are compile-time constants.

**Cache theme access**: Store `Theme.of(context).textTheme` in variable for repeated use.

**Avoid rebuilds**: Don't create new TextStyle instances in build method unnecessarily.

**Font preloading**: Load custom fonts in splash screen to avoid flash of unstyled text (FOUT).
