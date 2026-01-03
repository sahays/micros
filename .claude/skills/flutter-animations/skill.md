---
name: flutter-animations
description: Implement animations in Flutter using implicit, explicit, and physics-based approaches. Use when adding motion, transitions, or interactive animations to UI.
---

# Flutter Animations

## Core Principles

**Implicit first**: Use `AnimatedFoo` widgets (AnimatedContainer, AnimatedOpacity) for simple property animations. Less code, automatic animation.

**Explicit for control**: Use `AnimationController` when timing, sequencing, or custom curves needed.

**Physics-based for realism**: Use springs and friction for natural motion (BouncingScrollPhysics, SpringSimulation).

**60 FPS target**: Keep animations smooth. Avoid heavy computation in animation callbacks.

## Implicit Animations

**Built-in widgets**: AnimatedContainer, AnimatedOpacity, AnimatedPositioned, AnimatedAlign, AnimatedPadding, AnimatedSize.

**Pattern**: Change property value, widget animates automatically over specified duration.

**Custom implicit**: Use `TweenAnimationBuilder` for animating any custom property.

**Duration and curves**: Set `duration` and `curve` parameters for timing and easing.

## Explicit Animations

**Setup requirements**: AnimationController + Animation<T> + AnimatedBuilder or AnimatedWidget.

**Ticker provider**: Use `SingleTickerProviderStateMixin` (one controller) or `TickerProviderStateMixin` (multiple).

**Lifecycle**: Initialize in `initState`, dispose in `dispose` to prevent memory leaks.

**Control methods**: Use `forward()`, `reverse()`, `repeat()`, `stop()` to control animation playback.

**Listen for updates**: Use `AnimatedBuilder` to rebuild only animated portions of widget tree.

## Animation Curves

**Standard**: `Curves.easeInOut` for most transitions.

**Entrances**: `Curves.easeOut` for elements appearing.

**Exits**: `Curves.easeIn` for elements disappearing.

**Bouncy**: `Curves.elasticOut`, `Curves.bounceOut` for playful motion.

**Custom**: Define `Cubic` curves for brand-specific motion design.

## Staggered Animations

**Sequential timing**: Use `Interval` curve to offset multiple animations on single controller.

**Coordinated motion**: Animate multiple properties with different timing using same controller.

**Overlap control**: Adjust interval start/end values (0.0 to 1.0) to control overlap.

## Hero Animations

**Shared element transitions**: Wrap widget in `Hero` with matching tag on both screens.

**Automatic flight**: Flutter automatically animates position, size, and shape between routes.

**Tag uniqueness**: Ensure tag is unique within each route but identical across routes.

**Custom flight**: Override `createRectTween` for custom path curves.

## Page Transitions

**Custom routes**: Use `PageRouteBuilder` with custom `transitionsBuilder`.

**Built-in transitions**: `SlideTransition`, `FadeTransition`, `ScaleTransition`, `RotationTransition`.

**Combine transitions**: Layer multiple transition widgets for complex effects.

**Secondary animation**: Use `secondaryAnimation` for exit transitions of previous route.

## Performance

**Repaint boundaries**: Wrap animated widgets with `RepaintBoundary` to isolate repaints.

**AnimatedBuilder**: Rebuild only animated subtree, not entire widget tree.

**Avoid setState**: Use AnimatedBuilder instead of setState in animation listeners.

**Dispose controllers**: Always dispose controllers in `dispose()` to prevent memory leaks.

**Reduce layers**: Minimize `Opacity` widget (expensive), prefer `AnimatedOpacity`.

## Common Patterns

**Fade in on mount**: AnimatedOpacity with delayed setState or TweenAnimationBuilder.

**Slide from edge**: AnimatedPositioned or SlideTransition with Offset tween.

**Expand/collapse**: AnimatedSize or SizeTransition for height/width changes.

**Loading indicators**: CircularProgressIndicator or custom with RotationTransition.

**Shimmer effect**: Gradient animation with repeating AnimationController.

**Pull to refresh**: RefreshIndicator with physics-based spring animation.

## Accessibility

**Reduce motion**: Check `MediaQuery.of(context).disableAnimations` and reduce/skip decorative animations.

**Duration limits**: Keep essential animations under 500ms for better accessibility.

**Optional animations**: Allow users to disable non-essential animations in settings.

**Maintain functionality**: Ensure animations don't prevent access to content or features.
