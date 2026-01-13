---
name: glassmorphic-design
description:
  Apply a production-grade, design-studio like, high-end dashboard style using glassmorphism, non-standard typography,
  and high-contrast complementary colors conforming to the new Apple liquid glass system.
---

# Avant-Garde Glass Design System

This skill implements a versatile dashboard layout based on glassmorphism but with a bold, non-standard visual identity.

## 1. Visual DNA

- **Theme:** High-depth Glassmorphism.
- **Base Surfaces:** Cards use `rgba(255, 255, 255, 0.7)` with a `backdrop-filter: blur(16px)` and a 1.5px solid white
  border (`rgba(255, 255, 255, 0.4)`).
- **Shadows:** Multi-layered soft shadows. Use
  `box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 20px 25px -5px rgb(0 0 0 / 0.05)`.
- **Corner Radius:** Extreme rounding (`32px`) for main containers; `16px` for nested elements.
- Refer to the screenshot in the assets folder

## 2. Complementary Color Palette (High Contrast)

We are moving away from the monochromatic/purple HR look to a high-energy complementary scheme:

- **Primary (Action):** Electric Tangerine (`#FF8C42`)
- **Complementary (Accent):** Deep Royal Blue (`#08415C`)
- **Success:** Emerald Teal (`#12E193`)
- **Background:** Cool Slate Grey (`#E2E8F0`) to provide contrast for the orange/blue elements.

## 3. Non-Standard Typography

Instead of standard UI fonts like Inter, this system uses a high-character pairing:

- **Headings:** **"Clash Display"** or **"Space Grotesk"** (Bold/Medium). Focus on tight letter-spacing (`-0.02em`) and
  large scale.
- **Body/Data:** **"JetBrains Mono"** or **"IBM Plex Mono"**. Using a monospaced font for UI data gives a
  "tech-forward," non-standard editorial feel.

## 4. Generic Component Logic

- **Sidebar:** Minimalist icon-only or text-only vertical bar with `32px` padding.
- **Metric Cards:** Use a "Glass-on-Glass" stack where the stat value sits on a secondary blurred layer.
- **Interactive Elements:** Use the Complementary Orange (`#FF8C42`) for primary calls to action to create a focal point
  against the Deep Blue and Slate background.
