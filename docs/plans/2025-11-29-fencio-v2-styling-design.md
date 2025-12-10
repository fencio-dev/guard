# Fencio v2.0 Style Implementation Design

**Date:** 2025-11-29
**Scope:** Complete redesign of mcp-ui application to match fencio.dev v2.0 style guide
**Approach:** Component Enhancement (phased migration)

## Overview

This design applies the Composio-inspired fencio.dev v2.0 aesthetic to the entire mcp-ui application. The implementation uses a three-phase approach: update global foundations, create enhanced component variants, then migrate pages one by one. This strategy delivers the core aesthetic—deep dark backgrounds, gradient text, and physics-based motion—without breaking existing functionality.

## Design Constraints

- **Full application scope:** All pages receive the new style (LoginPage, console pages, AppShell layout)
- **No smooth scrolling:** Skip Lenis library; the app is a dashboard, not a marketing site
- **Core aesthetic focus:** Implement deep dark backgrounds, gradient text, and framer-motion animations; skip complex interactive effects (mouse tracking, border beams, spotlight cards)
- **Zero breaking changes:** Existing component APIs remain unchanged
- **Tech stack:** Vite + React 19 + React Router + Tailwind CSS + Framer Motion (already installed)

## Architecture

### Phase 1: Foundation Layer

Update global styles that affect the entire application immediately.

#### Color System

Replace the current light-first color system with the fencio.dev v2.0 dark palette.

**Target Colors:**
- Canvas background: `#0A0A0A` (neutral-950, not pure black)
- Card surfaces: `#171717` (neutral-900)
- Subtle borders: `#262626` (neutral-800)
- Highlight borders: `#404040` (neutral-700)
- Primary text: White
- Secondary text: `#A3A3A3` (neutral-400)
- Muted text: `#737373` (neutral-500)

**Implementation:**
1. Update `.dark` CSS variables in `mcp-ui/src/index.css`
2. Set dark mode as default in `mcp-ui/src/components/ThemeProvider.tsx`
3. Remove light mode variables (can restore later if needed)

#### Typography

Add gradient text utilities to Tailwind configuration.

**New Classes:**
- `.text-gradient-hero` → `bg-clip-text text-transparent bg-gradient-to-b from-white to-neutral-400`

**Font Configuration:**
- Primary: Inter (already configured)
- Ensure weights 400, 500, 600, 700 are loaded

**Implementation:**
1. Add gradient utilities to `mcp-ui/tailwind.config.js`
2. Verify Inter font loading in `index.css`

### Phase 2: Enhanced Component Variants

Create new component variants that coexist with existing components.

#### GradientCard Component

**File:** `mcp-ui/src/components/ui/gradient-card.tsx`

**Purpose:** Wrapper around existing Card with enhanced visual effects.

**Props:**
- `variant?: 'default' | 'gradient' | 'glass'`
- All standard Card props

**Styling:**
- `default`: Standard dark card with new color tokens
- `gradient`: Subtle gradient border glow on hover
- `glass`: Semi-transparent with backdrop blur

**Implementation:** New file that imports and wraps `Card` component.

#### Button Enhancements

**File:** `mcp-ui/src/components/ui/button.tsx` (update existing)

**New Variant:** `variant="primary-composio"`

**Styling:** `bg-white text-black hover:bg-neutral-200 shadow-[0_0_15px_rgba(255,255,255,0.1)]`

**Implementation:** Add variant to existing `buttonVariants` via `class-variance-authority`.

#### Badge Enhancements

**File:** `mcp-ui/src/components/ui/badge.tsx` (update existing)

**New Variant:** `variant="tagline"`

**Styling:** Uppercase text, wide tracking, primary color with soft background (matches style guide's "Tagline/Badge" spec).

**Implementation:** Add variant to existing `badgeVariants` via `class-variance-authority`.

### Phase 3: Animation System

Standardize entrance animations using existing Framer Motion library.

#### Animation Variants

**File:** `mcp-ui/src/lib/animations.ts` (new file)

**Exports:**

```typescript
// Stagger container for page layouts with multiple elements
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

// Standard entry animation for cards and sections
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

// Entry animation for important cards and images
export const scaleReveal = {
  hidden: { opacity: 0, scale: 0.95 },
  show: {
    opacity: 1,
    scale: 1,
    transition: { duration: 0.5, ease: "easeOut" }
  },
};
```

#### Integration Strategy

**Current state:** `AppShell.tsx` already uses page transitions with framer-motion (keep this).

**Enhancements:**
1. Add stagger animations to page sections
2. Use `viewport={{ once: true, margin: "-100px" }}` to prevent animation replay on scroll-up
3. Respect `useReducedMotion` hook (already implemented)

## Page Migration Plan

Migrate pages one by one after foundation and components are ready.

### Priority Order

1. **LoginPage** (highest visibility, simpler structure)
2. **AgentsIndexPage** (main dashboard)
3. **McpServersPage, ApiKeysPage, SettingsPage**
4. **AgentDetailPage, AgentPoliciesPage**
5. **AppShell** (Sidebar, TopBar)

### LoginPage Migration

**Current:** Two-column layout with Card component and basic styling.

**Changes:**
- Background: `bg-neutral-950` with subtle radial gradient from center
- Left panel: Use `GradientCard` with glass effect, apply gradient text to heading, increase heading size (text-4xl → text-5xl)
- Right panel: Darker background (`bg-neutral-900/50`), fade-up animation for onboarding stepper
- Overall: Page-level fade-in animation on mount

### Console Pages Migration

**Changes applied to AgentsIndexPage, McpServersPage, ApiKeysPage, SettingsPage, AgentDetailPage, AgentPoliciesPage:**

**Headers:**
- Apply gradient text to page titles
- Add fade-up animation
- Increase font size and weight

**Cards:**
- Filters card: Update to new dark background, keep standard Card
- Data table card: Migrate to `GradientCard` with subtle border glow on hover
- Add scale-reveal animation on page load

**Interactive Elements:**
- Buttons: Use `primary-composio` variant for primary actions
- Badges: Migrate decision badges (ALLOW/BLOCK) to enhanced Badge with better contrast
- Table rows: Enhance hover state

### AppShell Migration

**Sidebar:**
- Background: `bg-neutral-900` (slightly lighter than main canvas)
- Active nav item: Add subtle gradient border on left edge
- Logo area: Optional gradient text treatment

**TopBar:**
- Background: Match new color system
- Add subtle border-bottom with neutral-800

**Main Content Area:**
- Background: `bg-neutral-950` (deepest layer)
- Max-width container: Keep existing, ensure proper contrast

## Testing Strategy

### Per-Phase Testing

**Phase 1 (Foundation):**
- Visual regression check on all pages
- Verify no broken layouts
- Confirm dark mode applies correctly

**Phase 2 (Components):**
- Manual testing of new components in isolation
- Verify variants render correctly
- Test all prop combinations

**Phase 3 (Page Migration):**
- Test each page individually after migration
- Manual QA checklist (see below)
- Check responsive breakpoints (mobile, tablet, desktop)

### Page Migration Checklist

For each migrated page:
- [ ] Colors match fencio.dev style guide (backgrounds, text, borders)
- [ ] Gradient text renders without FOUC (flash of unstyled content)
- [ ] Animations play smoothly on first render
- [ ] Reduced motion preference is respected
- [ ] Mobile and tablet responsive layouts work
- [ ] No accessibility regressions (contrast ratios, focus states)
- [ ] Dark theme tokens are used consistently

### Rollback Safety

- **Git branching:** Work in feature branch `feat/fencio-v2-styling`
- **Component coexistence:** Old and new components work side-by-side during migration
- **CSS variables:** Can be reverted independently if color choices need adjustment
- **No breaking changes:** Existing component APIs remain unchanged

## Documentation

**Create:** `docs/styling/fencio-v2-implementation.md`

**Contents:**
- Component usage examples
- Animation patterns
- Color token reference
- Migration progress tracker

## Success Criteria

- All pages use the fencio.dev v2.0 color palette
- Page titles use gradient text treatment
- Cards have subtle depth and hover effects
- Animations enhance the luxurious feel without distraction
- No functionality is broken
- All tests pass
- Reduced motion preferences are respected
