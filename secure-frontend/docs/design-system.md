# Secure Frontend Design System

This document outlines the design principles, tokens, and component patterns used in the Secure Frontend application. The design is influenced by Dieter Rams' philosophy, prioritizing user experience and clarity over ornamental complexity.

## Design Philosophy

- **Functional Aesthetics:** Every element serves a purpose.
- **Precision:** Meticulous implementation of spacing, typography, and shadows.
- **Spec-Driven:** Consistent patterns across all microservices.

## Typography

- **Display & Body:** `Scope One` (Serif)
  - Used for all headings and body text to provide a unique, professional look.
- **Monospace:** `JetBrains Mono`
  - Used for technical data, file sizes, timestamps, and code snippets.
- **Headers:** 
  - Light Mode: `#000000`
  - Dark Mode: `#FFFFFF`

## Color System

The system uses a **60-20-10 rule** for color distribution.

| Token | Value (Light) | Value (Dark) | Description |
| :--- | :--- | :--- | :--- |
| **Brand Primary** | `#FF8C42` | `#FF8C42` | Core Orange Brand Color |
| **Brand Dark** | `#E06915` | `#E06915` | Used for gradients and hover states |
| **Background** | `#FFFFFF` | `#000000` | Main application background |
| **Surface** | `#F9FAFB` | `#111111` | Card and component surfaces |
| **Border** | `#E5E7EB` | `#333333` | Subtle boundaries |

## Components

### Buttons
- **Gradient Buttons (`.btn-gradient`):** 
  - Background: Monochromatic orange gradient (`135deg`, Primary to Dark).
  - Shadow: Custom `shadow-button` for depth.
  - Interaction: 1.1x brightness on hover with subtle lift.
- **Action Buttons (`.btn-action`):**
  - Minimalist style with subtle shadows, used for secondary actions in footers.

### Cards
Cards follow a strict anatomical structure:
1. **Header:** Contains a Lucide-style icon (bg-tinted) and the primary title.
2. **Body:** Contains supporting information and metadata.
3. **Footer:** Contains action-oriented buttons (e.g., Download, Share).
4. **Shadows:** Always uses `shadow-md` (resting) and `shadow-lg` (hover).

### Icons
- **System:** Lucide React (standardized SVG paths).
- **Stroke Width:** 2px.
- **Sizing:** Standardized at 14px (buttons), 20px (card headers).

## CSS Architecture
- **Framework:** Tailwind CSS (via CLI).
- **Input:** `static/input.css` (defines CSS variables and component layers).
- **Output:** `static/glassmorphic.css` (built output).
- **Variables:** Uses CSS Custom Properties for all tokens to support seamless Dark/Light mode switching via `prefers-color-scheme`.
