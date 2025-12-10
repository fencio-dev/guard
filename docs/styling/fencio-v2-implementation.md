# Fencio v2.0 Style Implementation

**Date:** 2025-11-29
**Status:** Complete
**Version:** 1.0.0

## Overview

This document describes the implementation of the fencio.dev v2.0 design system across the mcp-ui application. The implementation applies a modern, luxurious dark aesthetic inspired by Composio, featuring deep dark backgrounds, gradient text treatments, and physics-based animations.

## Implementation Summary

All planned phases have been completed:
- **Phase 1:** Foundation layer (color system, typography, dark mode)
- **Phase 2:** Enhanced component variants (GradientCard, Button, Badge)
- **Phase 3:** Animation system (standardized entrance animations)
- **Page Migrations:** All pages migrated to new styling
- **AppShell:** Sidebar and TopBar updated with new design

## Color System

### Updated Dark Palette

The application now uses the following color scheme from [mcp-ui/src/index.css](../../mcp-ui/src/index.css):

| Token | Color | Hex | Usage |
|-------|-------|-----|-------|
| `--background` | `0 0% 4%` | `#0A0A0A` | Canvas background (neutral-950) |
| `--card` | `0 0% 9%` | `#171717` | Card surfaces (neutral-900) |
| `--border` | `0 0% 15%` | `#262626` | Subtle borders (neutral-800) |
| `--input` | `0 0% 25%` | `#404040` | Highlight borders (neutral-700) |
| `--foreground` | `0 0% 100%` | `#FFFFFF` | Primary text (white) |
| `--muted-foreground` | `0 0% 45%` | `#737373` | Muted text (neutral-500) |

### Dark Mode Configuration

Dark mode is now the default theme. Configuration in [mcp-ui/src/components/ThemeProvider.tsx](../../mcp-ui/src/components/ThemeProvider.tsx):

```typescript
React.useEffect(() => {
  document.documentElement.classList.add("dark");
}, []);
```

## Typography

### Gradient Text Utility

Added gradient text treatment for headings in [mcp-ui/tailwind.config.js](../../mcp-ui/tailwind.config.js):

```javascript
backgroundImage: {
  'gradient-hero': 'linear-gradient(to bottom, white, rgb(163 163 163))',
}
```

**Usage:**
```jsx
<h1 className="bg-gradient-hero bg-clip-text text-transparent">
  Page Title
</h1>
```

**Applied to:**
- All page titles (text-4xl or text-5xl)
- Sidebar "Console" heading
- LoginPage main heading

## Component Variants

### GradientCard

**Location:** [mcp-ui/src/components/ui/gradient-card.tsx](../../mcp-ui/src/components/ui/gradient-card.tsx)

A wrapper around the standard Card component with enhanced visual effects.

**Variants:**
- `default`: Standard dark card with new color tokens
- `gradient`: Subtle gradient border glow on hover
- `glass`: Semi-transparent with backdrop blur

**Example:**
```tsx
import { GradientCard, CardHeader, CardTitle, CardContent } from "@/components/ui/gradient-card";

<GradientCard variant="gradient" hoverable>
  <CardHeader>
    <CardTitle>Your Title</CardTitle>
  </CardHeader>
  <CardContent>
    {/* Content */}
  </CardContent>
</GradientCard>
```

### Button Enhancements

**Location:** [mcp-ui/src/components/ui/button.tsx](../../mcp-ui/src/components/ui/button.tsx)

**New Variant:** `primary-composio`

**Styling:** White button with black text and subtle glow
```typescript
"bg-white text-black hover:bg-neutral-200 shadow-[0_0_15px_rgba(255,255,255,0.1)]"
```

**Example:**
```tsx
<Button variant="primary-composio">
  Generate API Key
</Button>
```

### Badge Enhancements

**Location:** [mcp-ui/src/components/ui/badge.tsx](../../mcp-ui/src/components/ui/badge.tsx)

**New Variant:** `tagline`

**Styling:** Uppercase text with wide tracking for taglines/labels
```typescript
"uppercase tracking-wider text-xs font-semibold border-transparent bg-primary/10 text-primary"
```

## Animation System

**Location:** [mcp-ui/src/lib/animations.ts](../../mcp-ui/src/lib/animations.ts)

Provides standardized animation variants for consistent entrance animations across the app.

### Available Animations

**1. staggerContainer**
- Purpose: Stagger animation for page layouts with multiple child elements
- Usage: Wrap page root element

**2. fadeUp**
- Purpose: Standard entry animation for sections and headers
- Physics: Spring animation with stiffness: 50, damping: 20
- Usage: Individual page sections

**3. scaleReveal**
- Purpose: Entry animation for important cards and images
- Effect: Gentle scale-up from 0.95 to 1.0
- Usage: Primary content cards

**4. defaultViewport**
- Purpose: Default viewport settings for scroll-based animations
- Config: `{ once: true, margin: "-100px" }`

### Example Usage

```tsx
import { motion } from "framer-motion";
import { staggerContainer, fadeUp, scaleReveal } from "@/lib/animations";

<motion.div
  variants={staggerContainer}
  initial="hidden"
  animate="show"
>
  <motion.div variants={fadeUp}>
    <h1>Page Title</h1>
  </motion.div>

  <motion.div variants={scaleReveal}>
    <GradientCard variant="gradient">
      {/* Content */}
    </GradientCard>
  </motion.div>
</motion.div>
```

## Page Migrations

All pages have been migrated to the new styling system:

### LoginPage
- ✅ Background: Radial gradient from `rgb(23 23 23 / 0.5)` to `rgb(10 10 10)`
- ✅ Left panel: GradientCard with glass effect
- ✅ Main heading: text-5xl with gradient treatment
- ✅ Page-level fade-in animation

### AgentsIndexPage
- ✅ Gradient text on page title (text-4xl)
- ✅ Main table card: GradientCard with gradient variant and hover effect
- ✅ Stagger animations for page sections
- ✅ Enhanced table row hover states

### McpServersPage
- ✅ Gradient text on page title
- ✅ "Add Server" button uses primary-composio variant
- ✅ Animations for server grid

### ApiKeysPage
- ✅ Gradient text on page title
- ✅ Main card with gradient variant
- ✅ "Generate API Key" button uses primary-composio variant
- ✅ Page-level stagger animations

### SettingsPage
- ✅ Gradient text on page title
- ✅ Settings card with gradient variant
- ✅ Page-level animations

## AppShell Migration

### Overall Layout
**Location:** [mcp-ui/src/layouts/AppShell.tsx](../../mcp-ui/src/layouts/AppShell.tsx)

- ✅ Root background: `bg-neutral-950`
- ✅ Main content area: `bg-neutral-950`
- ✅ Maintains existing page transitions with framer-motion

### Sidebar
**Location:** [mcp-ui/src/components/Sidebar.tsx](../../mcp-ui/src/components/Sidebar.tsx)

- ✅ Background: `bg-neutral-900` (slightly lighter than canvas)
- ✅ Border: `border-r border-neutral-800`
- ✅ Logo: White gradient with subtle shadow
- ✅ Console heading: Gradient text treatment
- ✅ Active nav item: `bg-neutral-800` with left white border
- ✅ Inactive nav item: `text-neutral-400 hover:text-neutral-200`

### TopBar
**Location:** [mcp-ui/src/components/TopBar.tsx](../../mcp-ui/src/components/TopBar.tsx)

- ✅ Background: `bg-neutral-900/80` with backdrop blur
- ✅ Border: `border-b border-neutral-800`
- ✅ Text: `text-neutral-400`

## Migration Checklist Results

All pages have been validated against the migration checklist:

- ✅ Colors match fencio.dev style guide
- ✅ Gradient text renders without FOUC
- ✅ Animations play smoothly on first render
- ✅ Reduced motion preference is respected (via existing useReducedMotion hook)
- ✅ Build completes successfully
- ✅ No TypeScript errors
- ✅ Dark theme tokens used consistently
- ✅ No breaking changes to component APIs

## Component Usage Guide

### When to Use GradientCard vs Card

**Use GradientCard:**
- Main content cards on dashboard pages
- Important feature cards that need visual emphasis
- Cards that benefit from subtle hover effects

**Use standard Card:**
- Filter panels and auxiliary content
- Form containers
- Dialogs and modals

### When to Use primary-composio Button

**Use primary-composio:**
- Primary CTAs that need high visibility on dark backgrounds
- "Generate", "Create", or "Add" actions
- Top-level action buttons in page headers

**Use default Button:**
- Secondary actions
- Table actions
- Form submit buttons

## Build Status

✅ Build successful with no errors
- TypeScript compilation: Pass
- Vite build: Pass
- Bundle size: 863.53 kB (gzipped: 260.74 kB)

## Future Enhancements

Potential improvements for future iterations:

1. **Performance:** Consider code splitting for large bundle size
2. **Accessibility:** Add comprehensive contrast ratio testing
3. **Theme Toggle:** Optional light mode support (currently dark-only)
4. **Animation Customization:** User preference for animation intensity
5. **Component Library:** Extract GradientCard and animations to shared component library

## References

- [Design Plan](../plans/2025-11-29-fencio-v2-styling-design.md)
- [Fencio Developer Platform](https://developer.fencio.dev)
- [Framer Motion Documentation](https://www.framer.com/motion/)
- [Tailwind CSS Documentation](https://tailwindcss.com)
