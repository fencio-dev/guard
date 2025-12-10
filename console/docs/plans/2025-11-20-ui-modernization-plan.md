# MCP Console UI Modernization Plan

**Date:** 2025-11-20
**Status:** Draft
**Reference Style:** "Atlas" Theme (defined in `console/docs/style.md`)
**Goal:** Refactor the existing UI to match the new "Atlas" design system (Cool neutrals, clean cards, fluid motion) without altering core functionality.

---

## 1. Design System Implementation

### 1.1 Tailwind Configuration
*   **Objective:** Update `tailwind.config.js` and `index.css` to match the CSS variables defined in `style.md`.
*   **Tasks:**
    *   Update `colors` object in `tailwind.config.js`.
    *   Update `--background`, `--card`, `--primary`, etc., variables in `src/index.css`.
    *   Add custom box shadows (`shadow-card`, `shadow-hover`) to `tailwind.config.js`.

### 1.2 Component Refactoring
We will update the base Shadcn UI components to match the new style.

*   **`src/components/ui/card.tsx`**:
    *   Remove default heavy borders if present.
    *   Apply `rounded-xl` and new shadow utility.
    *   Ensure hover states (lift effect) are possible via utility classes or a wrapper.
*   **`src/components/ui/button.tsx`**:
    *   Update padding/height for `sm`, `md`, `lg` to match `style.md`.
    *   Ensure `primary` variant uses the new vibrant blue.
*   **`src/components/ui/table.tsx`**:
    *   Update header background to `--subtle-bg`.
    *   Adjust row padding.

---

## 2. Layout Modernization

### 2.1 App Shell (`src/layouts/AppShell.tsx`)
*   **Structure:** Implement the "detached sidebar" look.
*   **Background:** Apply `--background` (cool gray) to the main app container.
*   **Sidebar:**
    *   Background: `--subtle-bg` or White (floating).
    *   Animation: Add `framer-motion` `layoutId` for the active tab indicator.
*   **Top Bar:**
    *   Make it sticky and blurred (`backdrop-blur`).
    *   Remove heavy bottom borders; use subtle separation.

---

## 3. Page Redesigns

### 3.1 Agents Index (`src/pages/AgentsIndexPage.tsx`)
*   **View:** Convert the standard `Table` to a "List of Cards" or a "Rich Table" (Table with card-like styling).
*   **Cards:** Display Agent Name, Status (Pill), Last Active, and simple metrics.
*   **Motion:** Staggered entrance animation for list items.

### 3.2 MCP Servers (`src/pages/McpServersPage.tsx`)
*   **Grid:** Ensure responsive grid (3 columns on desktop).
*   **Card:** "Atlas" style server cards.
    *   Icon (Lucide) in a rounded square container.
    *   Status indicator (dot) in the header.
    *   Configuration summary in the body.

### 3.3 Login Page (`src/pages/LoginPage.tsx`)
*   **Layout:** Split screen or centered card on a graphical background.
*   **Animation:** Implement the "Abstract Shapes" or "Shield" animation (using Framer Motion) defined in the previous attempts, but styled with the new "Atlas" blue.

---

## 4. Execution Steps

1.  **Setup:** Update Tailwind config & CSS variables.
2.  **Layout:** Refactor `AppShell`, `Sidebar`, `TopBar`.
3.  **Components:** Bulk update `ui/` folder components.
4.  **Pages:** Go page-by-page (Login -> Agents -> MCP -> Keys) and apply new patterns.
5.  **Review:** Verify functionality remains intact (auth, data fetching).

---

## 5. Dependencies
*   `framer-motion` (Installed)
*   `lucide-react` (Installed)
*   `clsx`, `tailwind-merge` (Installed)

