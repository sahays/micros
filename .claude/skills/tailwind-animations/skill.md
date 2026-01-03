---
name: tailwind-animations
description:
  Implement smooth animations and transitions with Tailwind CSS for interactive, polished UI. Use when adding micro-interactions,
  page transitions, loading states, and hover effects. Focuses on performance and accessibility.
---

# Tailwind Animations

## Custom Animations in Config

**Define keyframes**: Add custom animations to `tailwind.config.js`

**Animation system**:
- Define @keyframes for animation steps
- Create animation utilities using keyframes
- Use animation utilities in components

**Common custom animations**:
- `animate-fade-in`: Opacity 0→1
- `animate-slide-up`: Translate-y + fade
- `animate-scale-in`: Scale 0.95→1
- `animate-shimmer`: Loading shimmer effect

## Built-in Animations

**Spin**: `animate-spin` - Continuous rotation (loading spinners)

**Ping**: `animate-ping` - Pulsing circle (notifications)

**Pulse**: `animate-pulse` - Opacity pulse (loading states)

**Bounce**: `animate-bounce` - Bounce effect (attention grabbers)

**Use sparingly**: Built-in animations can feel dated. Prefer custom subtle animations.

## Transitions

**Transition properties**: `transition-{property}`
- `transition-all`: All properties
- `transition-colors`: Colors only
- `transition-opacity`: Opacity only
- `transition-transform`: Transform only
- `transition-shadow`: Shadow only

**Prefer specific**: Use `transition-colors` not `transition-all` for better performance.

## Transition Duration

**Scale**: `duration-{time}`
- `duration-75`: Very fast (75ms)
- `duration-150`: Fast (150ms) - Quick interactions
- `duration-200`: Normal (200ms) - Standard hover
- `duration-300`: Slow (300ms) - Emphasized transitions
- `duration-500`: Very slow (500ms) - Dramatic effects
- `duration-700`: Extra slow (700ms) - Special cases
- `duration-1000`: 1 second - Rare, very noticeable

**Default**: `duration-200` for most interactions.

## Timing Functions

**Easing**: `ease-{type}`
- `ease-linear`: Constant speed
- `ease-in`: Slow start
- `ease-out`: Slow end (most natural for UI)
- `ease-in-out`: Slow start and end

**Prefer ease-out**: Most natural feeling for user interactions.

## Hover Animations

**Scale**: `hover:scale-105` - Subtle grow effect

**Lift**: `hover:-translate-y-1 hover:shadow-lg` - Card lift effect

**Brightness**: `hover:brightness-110` - Image brightening

**Color shift**: `hover:bg-blue-600 transition-colors` - Button color change

**Combined**: `hover:scale-105 hover:shadow-xl transition-all duration-200`

## Group Hover

**Pattern**: Parent has `group` class, children use `group-hover:`

**Use case**: Hover card to animate inner elements

**Example**: Card with `group`, icon with `group-hover:scale-110`

**Multiple groups**: Use `group/{name}` for nested groups

## Focus Animations

**Ring animation**: `focus:ring-2 focus:ring-blue-500 transition-shadow`

**Scale on focus**: `focus:scale-105` for emphasis

**Smooth ring**: Always include `transition` for smooth ring appearance

## Loading Animations

**Shimmer skeleton**: Gradient animation across placeholder

**Pulse skeleton**: `animate-pulse` on gray backgrounds

**Spinner**: `animate-spin` on circular element

**Progressive reveal**: Stagger `animate-fade-in` with delays

## Stagger Animations

**Animation delay**: `animation-delay-{time}` (define in config)

**Use case**: List items appearing sequentially

**Pattern**: First item no delay, subsequent items increasing delay

**Example**: `delay-0`, `delay-100`, `delay-200`, etc.

## Page Transitions

**Route changes**: Fade out old content, fade in new

**Slide transitions**: Slide content left/right for navigation

**Implementation**: Use with React Router or Next.js transitions

## Micro-interactions

**Button click**: `active:scale-95` - Slight shrink on click

**Checkbox check**: Scale in checkmark

**Toggle switch**: Smooth slide of indicator

**Input focus**: Border color change + ring appearance

**Dropdown open**: Fade + slide down

## Scroll Animations

**Not built-in**: Use Intersection Observer or libraries

**Fade on scroll**: Elements fade in as they enter viewport

**Slide on scroll**: Elements slide up when visible

**Stagger on scroll**: Sequential appearance of list items

## Performance Best Practices

**Prefer transform and opacity**: GPU-accelerated properties

**Avoid**: Animating width, height, top, left (causes reflow)

**Use will-change sparingly**: Only for complex animations

**Debounce**: Limit animation frequency on scroll/resize

**Reduced motion**: Respect `prefers-reduced-motion`

## Reduced Motion

**Auto-respected in v4**: Tailwind handles reduced motion automatically

**Custom animations**: Ensure config animations respect reduced-motion

**Disable on preference**: Animations disabled for users with motion sensitivity

## Animation Patterns

**Entrance animations**: Fade in, slide up, scale in

**Exit animations**: Fade out, slide down, scale out

**Loading states**: Pulse, shimmer, spin

**Attention**: Bounce, ping (use sparingly)

**Feedback**: Scale on click, color change, shake on error

## Button Animations

**Standard hover**: `hover:bg-blue-600 transition-colors duration-200`

**With scale**: `hover:scale-105 hover:shadow-lg transition-all duration-200`

**With lift**: `hover:-translate-y-0.5 hover:shadow-md transition-all duration-150`

**Active state**: `active:scale-95` for click feedback

**Loading button**: Disable + add spinner animation

## Card Animations

**Hover lift**: `hover:-translate-y-2 hover:shadow-xl transition-all duration-300`

**Scale**: `hover:scale-[1.02] transition-transform duration-200`

**Border glow**: Animate border color or shadow on hover

**Content reveal**: Slide in additional content on hover

## Modal Animations

**Backdrop**: Fade in `opacity-0` to `opacity-100`

**Modal content**: Scale in `scale-95` to `scale-100` + fade

**Exit**: Reverse entrance animation

**Fast entrance, slower exit**: Creates polished feel

## Notification Animations

**Slide in**: From top, right, or bottom based on position

**Fade + slide**: Combine for smooth appearance

**Auto-dismiss**: Fade out after timeout

**Stacked notifications**: Stagger animation for multiple

## Form Animations

**Error shake**: Horizontal shake on validation error

**Success check**: Scale in checkmark icon

**Field focus**: Smooth ring appearance

**Label float**: Animate label to top on focus (floating labels)

## List Animations

**Stagger entrance**: Items appear sequentially

**Hover highlight**: Background color transition on hover

**Expand/collapse**: Max-height transition (use with caution)

**Reorder**: Smooth position changes (requires JS library)

## Text Animations

**Gradient animation**: Animated gradient text

**Type writer**: Sequential character appearance (requires JS)

**Counter**: Animated number counting (requires JS)

**Fade in words**: Stagger fade for emphasis

## Best Practices

**Do**:
- Use subtle animations (duration-150 to duration-300)
- Prefer transform and opacity
- Add transitions to interactive elements
- Use ease-out for natural feel
- Respect reduced motion preference
- Test on actual devices
- Animate one property at a time

**Avoid**:
- Overly long animations (>500ms)
- Animating layout properties
- Too many simultaneous animations
- Animations without purpose
- Ignoring reduced motion
- Animation every element
- Constant motion (distracting)

## Animation Triggers

**Hover**: Most common, desktop-focused

**Focus**: Accessibility-required

**Active**: Click feedback

**Group hover**: Related element animation

**Scroll**: Entrance animations

**Time-based**: Auto-play (use sparingly)

**State change**: Loading→success, collapsed→expanded
