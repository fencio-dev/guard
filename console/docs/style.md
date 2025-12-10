Here is the **v2.0 fencio.dev Style Guide**. I have integrated your existing preferences with the specific technical and visual breakdowns from the Composio.dev analysis.

This version adds **Smooth Scrolling (Lenis)**, **Bento Grid** layouts, **Mouse-tracking Spotlights**, and **Border Beams** to your standard component set.

***

# fencio.dev Style Guide v2.0

> [!IMPORTANT]
> **Core Upgrade:** To achieve the "heavy/luxurious" feel of Composio, we are adding **Lenis (Smooth Scrolling)** as a mandatory dependency. Standard browser scrolling is too jittery for this aesthetic.

## 1. Design Philosophy
*   **Immersive Dark Mode**: Avoid "Pure Black" (`#000`). Use deep, rich greys (`#0A0A0A`) to allow shadows and glows to create depth.
*   **Physics-Based Motion**: Animations should feel like they have weight. We use `Lenis` for scroll inertia and `Framer Motion` for spring-based transitions.
*   **Modular Layouts (Bento)**: Information is organized in grid-based, puzzle-like cards that create a dashboard feel.
*   **Mouse-Aware Interactivity**: Elements react to the cursor location (Spotlights, glowing borders) to encourage exploration.

## 2. Tech Stack & Libraries
*   **Framework**: Next.js (App Router)
*   **Styling**: Tailwind CSS
*   **Animation**: `framer-motion` (Layouts/Transitions)
*   **Smooth Scroll**: `@studio-freight/react-lenis` (Inertia scrolling)
*   **Icons**: `lucide-react` (Clean, consistent strokes)
*   **Utilities**: `clsx`, `tailwind-merge`

## 3. Color Palette (Refined)

### Backgrounds
| Token | Hex | Tailwind Class | Usage |
| :--- | :--- | :--- | :--- |
| **Canvas** | `#0A0A0A` | `bg-neutral-950` | Main page background. |
| **Surface** | `#171717` | `bg-neutral-900` | Card backgrounds. |
| **Surface Hover** | `#262626` | `group-hover:bg-neutral-800` | Card hover state. |

### Borders & Dividers
| Token | Hex | Tailwind Class | Usage |
| :--- | :--- | :--- | :--- |
| **Subtle** | `#262626` | `border-neutral-800` | Default card borders. |
| **Highlight** | `#404040` | `border-neutral-700` | Active or hover borders. |

### Gradients & Effects
*   **Hero Text**: `bg-clip-text text-transparent bg-gradient-to-b from-white to-neutral-400`
*   **Spotlight Glow**: `radial-gradient(circle at center, rgba(255,255,255,0.15), transparent 80%)`
*   **Border Beam**: A linear gradient loop used on `::before` or `::after` elements to simulate a moving light along a border.

## 4. Typography
**Font Family**: `Inter` (or `Geist Sans`)

### Hierarchy
*   **H1 (Hero)**: `text-5xl md:text-7xl font-bold tracking-tight text-transparent bg-clip-text bg-gradient-to-b from-white to-white/60`.
*   **H2 (Section)**: `text-3xl md:text-4xl font-semibold tracking-tight text-white`.
*   **Body**: `text-base md:text-lg text-neutral-400 leading-relaxed`.
*   **Tagline/Badge**: `text-xs font-medium uppercase tracking-widest text-primary-400 bg-primary-500/10 border border-primary-500/20 rounded-full px-3 py-1`.

## 5. Animation Strategy

### A. Smooth Scrolling (Mandatory)
Wrap the root layout:
```tsx
<ReactLenis root options={{ lerp: 0.1, duration: 1.5, smoothWheel: true }}>
  {children}
</ReactLenis>
```

### B. Framer Motion Variants
Standardized variants to ensure consistency across the site.

```typescript
// animations.ts

// 1. Staggered Container (For lists, grids, or hero text lines)
export const staggerContainer = {
  hidden: { opacity: 0 },
  show: {
    opacity: 1,
    transition: {
      staggerChildren: 0.1,
      delayChildren: 0.3,
    },
  },
};

// 2. Fade Up (The standard entry animation)
export const fadeUp = {
  hidden: { opacity: 0, y: 30 },
  show: { 
    opacity: 1, 
    y: 0,
    transition: { 
      type: "spring", 
      stiffness: 50, 
      damping: 20 
    } 
  },
};

// 3. Scale Reveal (For images or bento cards)
export const scaleReveal = {
  hidden: { opacity: 0, scale: 0.95 },
  show: { 
    opacity: 1, 
    scale: 1,
    transition: { duration: 0.5, ease: "easeOut" }
  },
};
```

### C. Usage Pattern
Always use `viewport={{ once: true, margin: "-100px" }}` so animations don't replay annoyingly when scrolling up.

```tsx
<motion.div
  variants={staggerContainer}
  initial="hidden"
  whileInView="show"
  viewport={{ once: true, margin: "-100px" }}
>
   <motion.h2 variants={fadeUp}>Title</motion.h2>
</motion.div>
```

## 6. Layout Patterns

### The Bento Grid
Grid layouts that combine spans of 1, 2, or 3 columns.
*   **Structure**: `grid grid-cols-1 md:grid-cols-3 gap-4`
*   **Tall Card**: `md:row-span-2`
*   **Wide Card**: `md:col-span-2`

### Sticky Features
For explaining complex flows (like Composio's "Integrations" section):
*   **Left Side**: Sticky Text (`position: sticky; top: 100px;`)
*   **Right Side**: Scrolling visual assets.

## 7. Component Styles

### 1. The "Spotlight" Card (Mouse Tracking)
*Instead of static glassmorphism, use this dynamic effect.*

```tsx
// Logic: Update CSS variables --x and --y on mouse move
<div className="relative border border-neutral-800 bg-neutral-900 rounded-xl overflow-hidden group">
  {/* The Glow */}
  <div 
    className="pointer-events-none absolute -inset-px opacity-0 group-hover:opacity-100 transition duration-300"
    style={{
      background: `radial-gradient(600px circle at var(--x) var(--y), rgba(255,255,255,0.06), transparent 40%)`
    }}
  />
  <div className="relative z-10 p-6">
    {/* Content */}
  </div>
</div>
```

### 2. Primary Button (Composio Style)
A white button (high contrast against dark mode) with a soft shadow.

*   **Class**: `bg-white text-black font-semibold rounded-lg px-6 py-3 hover:bg-neutral-200 transition-all shadow-[0_0_15px_rgba(255,255,255,0.1)]`

### 3. Secondary Button / Badge
*   **Class**: `bg-neutral-900 border border-neutral-800 text-neutral-300 hover:text-white hover:border-neutral-700 transition-colors rounded-lg px-6 py-3`