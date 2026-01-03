---
name: flutter-gradients
description: Create linear, radial, and sweep gradients in Flutter for backgrounds, buttons, and visual effects. Use when implementing gradient backgrounds, overlays, or gradient text.
---

# Flutter Gradients

## Core Principles

**BoxDecoration for containers**: Use `decoration: BoxDecoration(gradient: ...)` for gradient backgrounds.

**ShaderMask for text**: Apply gradients to text and icons using `ShaderMask` with gradient shader.

**Performance**: Gradients render on GPU, but avoid excessive complexity (too many color stops).

**Accessibility**: Ensure text contrast over gradients meets WCAG standards (4.5:1 minimum).

## Linear Gradients

**Basic pattern**: Two or more colors transitioning in straight line direction.

**Directions**: Use `Alignment` values - `topLeft` to `bottomRight` (diagonal), `topCenter` to `bottomCenter` (vertical), `centerLeft` to `centerRight` (horizontal).

**Color stops**: Define specific positions (0.0 to 1.0) for each color, or omit for even distribution.

**Multiple colors**: Add three or more colors for complex multi-stop gradients.

## Radial Gradients

**Circular pattern**: Colors radiate from center point outward.

**Center position**: Adjust `center` using Alignment values for off-center gradients.

**Radius control**: Set `radius` (0.0 to 1.0+) relative to container size to control spread.

**Focal points**: Use `focal` and `focalRadius` for elliptical gradient effects.

## Sweep Gradients

**Conical pattern**: Colors rotate around center point like color wheel.

**Angles**: Define `startAngle` and `endAngle` in radians (0 to 2π for full circle).

**Use cases**: Loading spinners, color pickers, circular progress indicators, decorative elements.

**Color repetition**: Repeat first color at end for seamless circular gradient.

## Gradient Text

**ShaderMask approach**: Wrap Text widget in ShaderMask, use gradient as shader via `createShader`.

**BlendMode**: Use `blendMode: BlendMode.srcIn` for clean gradient masking.

**Required color**: Set text color (gets masked but required for sizing).

**Icons support**: Same ShaderMask pattern works for Icon widgets.

## Gradient Buttons

**Container wrapper**: Wrap ElevatedButton in Container with gradient BoxDecoration.

**Transparent button**: Set button `backgroundColor` and `shadowColor` to transparent.

**Ink widget**: Alternative approach with better Material ripple effects.

**Border radius**: Match button and container border radius for consistent shape.

## Gradient Overlays

**Image scrims**: Layer gradient Container over images using Stack for text readability.

**Common patterns**: Dark gradient at bottom for light text, light gradient at top for dark text.

**Transparency**: Use `Colors.transparent` as one gradient color for fade effect.

**Opacity control**: Adjust alpha channel of gradient colors for subtlety.

## TileMode

**Beyond bounds behavior**: Control how gradient repeats or extends past defined area.

**TileMode.clamp**: Default - extends edge colors infinitely.

**TileMode.repeated**: Repeats gradient pattern continuously.

**TileMode.mirror**: Alternates gradient direction with each repeat.

**TileMode.decal**: Renders transparent beyond gradient bounds.

## Common Patterns

**Glassmorphism**: Combine gradient with `BackdropFilter` blur effect.

**Neumorphism**: Layer multiple gradients with shadows for 3D depth illusion.

**Mesh gradients**: Overlay multiple RadialGradients with opacity for complex effects.

**Animated gradients**: Use `TweenAnimationBuilder` to animate color stop positions.

**Hero gradients**: Match gradient properties in Hero transitions for seamless animation.

## Performance Tips

**Limit color stops**: Keep under 5-6 colors for optimal performance.

**Cache decorations**: Store BoxDecoration instances in const or variables, reuse.

**Avoid nesting**: Minimize layered gradients - combine into single gradient when possible.

**Use const**: Mark BoxDecoration as const when gradient properties are compile-time constants.

## Accessibility

**Contrast testing**: Test text contrast using darkest gradient color as background reference.

**Solid fallbacks**: Provide solid color alternatives for reduced motion/effects preferences.

**Decorative only**: Don't use gradients to convey critical information - use for visual enhancement only.

**Color independence**: Combine with text labels or icons, don't rely on gradient colors alone.
