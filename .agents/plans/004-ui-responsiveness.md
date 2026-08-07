# Plan 004: UI Responsiveness Fix (Mobile S - Desktop 2K)

## 1. Objective
Audit and fix UI responsiveness across the entire application (Mobile S 320px to Desktop 2K 2560px). Specifically address the immediate issue of duplicate sidebar toggles (`UDashboardSidebarToggle` and `UDashboardSidebarCollapse`) rendering simultaneously on the chat page header.

## 2. Immediate Fixes (Double Button Issue)
### Location
- `app/pages/chat/[id].vue`
- `app/layouts/default.vue` (and any other layouts using `UDashboardNavbar`)

### Problem
The `#leading` slot of `UDashboardNavbar` renders both the mobile toggle and the desktop collapse button because they lack mutually exclusive responsive display classes.

### Resolution
- **Idiomatic Nuxt UI Fix**: Remove `<UDashboardSidebarToggle />` entirely from `#leading` slots where `<UDashboardSidebarCollapse />` is used.
- Nuxt UI 4 handles the mobile/desktop behavior of `<UDashboardSidebarCollapse />` automatically. Adding manual breakpoint classes like `lg:hidden` violates the framework's conventions (see `.agents/knowledge/nuxt-way.md`).

## 3. Comprehensive Responsiveness Audit

### 3.1 Chat Interface (`app/pages/chat/[id].vue`)
- **Messages (`UChatMessages`)**: Ensure message bubbles do not stretch infinitely on Desktop 2K. Add a comfortable `max-w` to the container or bubbles if needed.
- **Prompt Input (`UChatPrompt`)**: Ensure the input field and bottom actions (tool picker, model selector) do not overflow or overlap the virtual keyboard on Mobile S (320px).
- **Header**: Ensure the conversation title truncates (`truncate` class) and doesn't push the header action buttons out of view on small screens.

### 3.2 Layout & Navigation (`app/layouts/default.vue`)
- Verify the sidebar correctly transforms into an off-canvas drawer on mobile (`< 1024px`) and functions as a collapsible sidebar on desktop.
- Check padding and spacing adjustments (`px-2` vs `px-4` vs `px-6`) across all breakpoints for `UDashboardNavbar` and `UDashboardPanel`.

### 3.3 Settings Pages
- Check grid layouts in `app/pages/settings/*.vue`.
- Ensure forms stack correctly on mobile (one column) and use multi-column grids or inline forms on desktop where appropriate.
- Ensure wide components (tables, horizontal lists) have horizontal scrolling on mobile.

### 3.4 Landing & Auth Pages
- Test `app/pages/index.vue`, `login.vue`, and `register.vue`.
- Ensure responsive typography (`text-3xl` on mobile scaling up to `text-6xl` or `text-7xl` on desktop).
- Ensure hero sections stack their content vertically on mobile.

## 4. Execution Steps
1. **Apply the Immediate Fix**: Remove `<UDashboardSidebarToggle />` from `app/pages/chat/[id].vue`, `app/pages/chat/index.vue`, and `app/pages/settings.vue`.
2. **Execute Component Reviews**: Go through the Chat, Settings, and Auth pages applying standard Nuxt UI v4 responsive utilities (`sm:`, `md:`, `lg:`, `xl:`, `2xl:`).
3. **Verify with Browser DevTools**: Emulate devices starting from 320px (e.g., iPhone SE) up to 2560px. Ensure no horizontal scrolling (unless intentional) and no overlapping elements.
