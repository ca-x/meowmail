# Astryx Frontend Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the complete Meowmail Web interface on Astryx, present settings as focused tabs, and improve email reading typography without changing the Rust API contract.

**Architecture:** Keep React 19 and Vite as the application runtime, then place Astryx `Theme`, `LayerProvider`, and component primitives at the root. Product-specific mail layout remains in Meowmail CSS, while buttons, inputs, dialogs, tabs, switches, feedback, loading, and empty states use Astryx. The sandboxed email body receives a dedicated editorial stylesheet based on Kami reading principles, isolated from application chrome.

**Tech Stack:** React 19, Vite 8, TypeScript 7, Astryx Core and Neutral Theme 0.1.8, Lucide React, Vitest, Testing Library, Playwright, Rust embedded static assets.

## Global Constraints

- Pin `@astryxdesign/core`, `@astryxdesign/theme-neutral`, `@astryxdesign/cli`, and `@stylexjs/stylex` to exact compatible versions.
- Keep `lucide-react` as the MIT-licensed application icon library; do not replace icons with emoji or text glyphs.
- Preserve the existing Rust API paths, request bodies, authentication behavior, and embedded frontend build contract.
- Preserve Chinese and English UI coverage; every new visible label must exist in both locales.
- Keep high-frequency mail navigation and keyboard actions immediate, without entrance animation.
- Animate only `transform`, `opacity`, color, and shadow for interactive feedback; do not use `transition: all`.
- Respect `prefers-reduced-motion`, `prefers-reduced-transparency`, and `prefers-contrast`.
- Use Astryx for application chrome and controls; use Kami principles only inside the document-style email reading surface.
- Keep the setting categories: General, Mail, Automation, Security, and Data.
- Keep exactly one Astryx `AppShell` and one `Layout` at the application root. Build the three-pane workspace with `LayoutPanel`, `useResizable`, and `ResizeHandle` rather than a custom grid or a nonexistent panel-group abstraction.
- Use `TreeList` with `TreeListItemData[]` for account/folder hierarchy, and `List` plus `Item as="li"` for message rows. Do not use `Table` for mail navigation or message lists.
- Use Astryx components directly. Do not introduce generic wrappers around `Button`, `IconButton`, `Dialog`, `TextInput`, or other primitives.
- Organize the migration by feature; keep TS/TSX implementation files near 250 lines by splitting page composition, state, data adapters, and focused components.
- Prefer Astryx component props and tokens. Business CSS may own mail-specific layout and the reading surface, but must not target Astryx internal class names.

---

### Task 1: Install Astryx and establish application providers

**Files:**
- Modify: `web/package.json`
- Modify: `web/package-lock.json`
- Create: `web/src/app/Providers.tsx`
- Modify: `web/src/main.tsx`
- Modify: `web/src/theme/ThemeProvider.tsx`
- Modify: `web/src/theme/tokens.css`
- Test: `web/src/test/theme-bootstrap.test.js`

**Interfaces:**
- Consumes: existing `ThemeMode`, `I18nProvider`, and React application root.
- Produces: `Providers({ children }: { children: ReactNode })` and an Astryx theme mode synchronized with `meowmail-theme`.

- [ ] **Step 1: Extend the theme bootstrap test**

Assert that `main.tsx` imports Astryx reset, core, and neutral-theme CSS, and that `Providers.tsx` mounts one `LayerProvider`.

```js
expect(mainSource).toContain('@astryxdesign/core/reset.css')
expect(mainSource).toContain('@astryxdesign/core/astryx.css')
expect(mainSource).toContain('@astryxdesign/theme-neutral/theme.css')
expect(providersSource).toContain('<LayerProvider')
```

- [ ] **Step 2: Run the focused test and confirm it fails**

Run: `cd web && npm run test:ci -- src/test/theme-bootstrap.test.js`

Expected: FAIL because the Astryx imports and provider do not exist.

- [ ] **Step 3: Install exact dependencies**

Run:

```bash
cd web
npm install --save-exact @astryxdesign/core@0.1.8 @astryxdesign/theme-{neutral,stone,butter,matcha,chocolate,gothic,y2k}@0.1.8 @stylexjs/stylex@0.19.0
npm install --save-dev --save-exact @astryxdesign/cli@0.1.8
```

- [ ] **Step 4: Add the provider composition**

Create `Providers.tsx` with Astryx `Theme`, the seven stable 0.1.8 visual themes, `LayerProvider`, the existing Meowmail theme state, and the existing i18n provider. Keep visual theme selection separate from the `system`, `light`, and `dark` color modes.

- [ ] **Step 5: Replace root imports and align Meowmail tokens**

Import Astryx CSS before Meowmail overrides. Keep Meowmail semantic tokens as a thin product layer mapped to Astryx color and spacing variables rather than duplicating component defaults.

- [ ] **Step 6: Verify Task 1**

Run:

```bash
cd web
npm run typecheck
npm run test:ci -- src/test/theme-bootstrap.test.js
```

Expected: PASS.

### Task 2: Migrate authentication, global feedback, and reusable controls

**Files:**
- Create: `web/src/shared/ui/AppBrand.tsx`
- Modify: `web/src/app/App.tsx`
- Modify: `web/src/features/auth/LoginPage.tsx`
- Modify: `web/src/features/auth/LockScreen.tsx`
- Modify: `web/src/features/auth/LoginPage.test.tsx`
- Modify: `web/src/styles/app.css`

**Interfaces:**
- Produces: `AppBrand` with the existing logo and localized product name.

- [ ] **Step 1: Add failing authentication interaction assertions**

Test that username/password remain correctly labelled, the password control stays `type="password"` until toggled, OIDC remains a link, and loading feedback is exposed through an Astryx spinner or busy button.

- [ ] **Step 2: Run authentication tests and confirm the new component expectations fail**

Run: `cd web && npm run test:ci -- src/features/auth/LoginPage.test.tsx`

- [ ] **Step 3: Implement the shared brand component and use Astryx controls directly**

Build only `AppBrand`. Use Astryx `IconButton` and `Tooltip` directly at each call site so labels, disabled state, variants, and context remain visible without a generic wrapper.

- [ ] **Step 4: Rebuild login and lock screens with Astryx controls**

Use `Card`, `Button`, `Banner`, and `Spinner`. Use Astryx `TextInput` where its typed API preserves required browser semantics; retain a narrowly scoped native password input when `autoComplete` and password-manager behavior cannot be expressed safely by Astryx 0.1.8. Keep the Apple-inspired split layout, but remove handcrafted input shells and duplicate button state logic.

- [ ] **Step 5: Replace the boot screen spinner and persistent errors**

Use Astryx `Spinner` for boot state and `Banner` for blocking authentication errors; do not use toast feedback for form validation.

- [ ] **Step 6: Verify Task 2**

Run: `cd web && npm run typecheck && npm run test:ci -- src/features/auth/LoginPage.test.tsx`

Expected: PASS.

### Task 3: Rebuild the mail application shell and message list

**Files:**
- Modify and split: `web/src/features/mail/MailWorkspace.tsx`
- Modify: `web/src/features/mail/MessageList.tsx`
- Create: focused shell/navigation files under `web/src/features/mail/workspace/`
- Modify: `web/src/features/mail/MailInteractions.test.tsx`
- Modify: `web/src/styles/app.css`

**Interfaces:**
- Consumes: existing API state and `Filter` behavior.
- Produces: Astryx-based toolbar controls, account/folder list controls, segmented filters, toast notifications, loading skeletons, and empty states.

- [ ] **Step 1: Add failing tests for shell behavior**

Cover search focus with Ctrl/Cmd+K, folder selection, account selection, sync loading, compose opening, toast feedback, mobile sidebar dismissal, and message selection.

- [ ] **Step 2: Run mail interaction tests and confirm the new semantics fail**

Run: `cd web && npm run test:ci -- src/features/mail/MailInteractions.test.tsx`

- [ ] **Step 3: Replace global controls with Astryx**

Use one `AppShell` and one `Layout`, with `LayoutPanel`, `useResizable`, and `ResizeHandle` for desktop panes. Use `MobileNav` for compact navigation. Use `Button`, `IconButton`, `TextInput`, `SegmentedControl`, `Badge`, `Tooltip`, `Spinner`, `Skeleton`, `EmptyState`, and `useToast` directly.

- [ ] **Step 4: Rebuild folder and account navigation**

Build `TreeListItemData[]` for accounts and folders and render them through `TreeList`, preserving unread counts, active account indicators, disabled future folders, WAI-ARIA tree keyboard behavior, and 44px mobile targets.

- [ ] **Step 5: Rebuild message rows**

Keep the current sender/avatar/subject/preview information architecture. Render rows with Astryx `List` and `Item as="li"`, Lucide icons, immediate selection, no list-navigation animation, and compact/default density attributes. Preserve the app's J/K focus and selection logic because Astryx `List` does not provide it.

- [ ] **Step 6: Replace the custom toast timer**

Use Astryx `useToast` with translated message bodies, stable `uniqueID` values, four-second informational auto-hide, and persistent error toasts.

- [ ] **Step 7: Verify Task 3**

Run: `cd web && npm run typecheck && npm run test:ci -- src/features/mail/MailInteractions.test.tsx`

Expected: PASS.

### Task 4: Rebuild message detail, attachments, and compose flows

**Files:**
- Modify: `web/src/features/mail/MessageDetail.tsx`
- Modify: `web/src/features/mail/ComposeDialog.tsx`
- Modify: `web/src/features/mail/AttachmentPreviewDialog.tsx`
- Modify: `web/src/features/mail/MailInteractions.test.tsx`
- Modify: `web/src/styles/app.css`

**Interfaces:**
- Produces: Astryx `Dialog`-based compose and attachment preview surfaces.
- Preserves: `ComposeDraft`, attachment preview mounting with `@file-viewer/web-full`, reply/forward/delete callbacks, and sandboxed HTML reading.

- [ ] **Step 1: Add failing dialog and attachment assertions**

Test Escape/backdrop behavior, form preservation, send loading, attachment preview/download affordances, toolbar labels, and reply/forward/delete actions.

- [ ] **Step 2: Run focused mail tests and confirm failure**

Run: `cd web && npm run test:ci -- src/features/mail/MailInteractions.test.tsx`

- [ ] **Step 3: Migrate detail toolbar and attachment controls**

Use Astryx `Toolbar`, `IconButton`, `Button`, `Badge`, `Banner`, and `EmptyState`. Keep sticky positioning and use Lucide icons for every action.

- [ ] **Step 4: Migrate compose to Astryx Dialog and form controls**

Use `Dialog`, `DialogHeader`, `Layout`, `LayoutContent`, `LayoutFooter`, `TextInput`, `TextArea`, `Selector`, `Button`, and `Banner`. Preserve recipient autocomplete-compatible markup and the current send API.

- [ ] **Step 5: Migrate attachment preview to fullscreen Astryx Dialog**

Retain the viewer container lifecycle and fallback download action. Use a centered Astryx spinner while the renderer loads and a persistent banner on viewer failure.

- [ ] **Step 6: Verify Task 4**

Run: `cd web && npm run typecheck && npm run test:ci -- src/features/mail/MailInteractions.test.tsx`

Expected: PASS.

### Task 5: Rebuild account management and tabbed settings

**Files:**
- Modify: `web/src/features/accounts/AccountDialog.tsx`
- Modify: `web/src/features/settings/SettingsDialog.tsx`
- Modify: `web/src/features/settings/MailExperienceSettings.tsx`
- Modify: `web/src/features/settings/ReceiveRulesEditor.tsx`
- Modify: `web/src/features/settings/SettingsDialog.test.tsx`
- Modify: `web/src/i18n/messages.ts`
- Modify: `web/src/styles/app.css`

**Interfaces:**
- Produces: `SettingsTab = "general" | "mail" | "automation" | "security" | "data"`.
- Produces: a controlled Astryx `TabList`/`Tab` settings navigation and one visible panel at a time.
- Preserves: all account, proxy, notification, MCP, retention, rule, profile, signature, and migration API behavior.

- [ ] **Step 1: Add failing tab accessibility tests**

Assert a five-tab `tablist`, correct localized names, one selected tab, arrow-key navigation, one visible panel, preserved control state across switches, and horizontal scrolling on narrow screens.

```tsx
expect(screen.getByRole("tablist", { name: "设置分类" })).toBeInTheDocument()
expect(screen.getByRole("tab", { name: "通用" })).toHaveAttribute("aria-selected", "true")
expect(screen.queryByRole("heading", { name: "MCP 访问" })).not.toBeInTheDocument()
```

- [ ] **Step 2: Run settings tests and confirm failure**

Run: `cd web && npm run test:ci -- src/features/settings/SettingsDialog.test.tsx`

- [ ] **Step 3: Add bilingual tab copy**

Add keys for settings category navigation and concise descriptions in Chinese and English. Keep “邮件签名” / “Email signature” terminology unchanged.

- [ ] **Step 4: Rebuild SettingsDialog with Astryx Dialog and tabs**

Map sections as follows:

```text
General: profile, avatar, language, theme, mail-account entry
Mail: reading, sending, signatures, reply and forward preferences
Automation: retention, sync range, cleanup rules, notifications
Security: PIN lock and MCP access
Data: selective encrypted import/export
```

Use a sticky tab rail on desktop and an overflow-x tab row on mobile. Tab switching is immediate; only color and the active indicator use a 160ms transition.

- [ ] **Step 5: Migrate settings and account controls**

Use Astryx `Section`, `FormLayout`, `TextInput`, `TextArea`, `NumberInput`, `Switch`, `CheckboxInput`, `Selector`, `SegmentedControl`, `Button`, `IconButton`, `Banner`, `Badge`, and `FileInput` directly. Use sections and spacing instead of wrapping every group in a card.

- [ ] **Step 6: Verify all settings workflows**

Run: `cd web && npm run typecheck && npm run test:ci -- src/features/settings/SettingsDialog.test.tsx`

Expected: notification testing, MCP token generation/deletion permission, sync fetch range, administrator export scope, profile, signatures, and rules remain functional.

### Task 6: Apply editorial email reading typography

**Files:**
- Create: `web/src/features/mail/message-reading.css`
- Modify: `web/src/features/mail/MessageDetail.tsx`
- Modify: `web/src/features/mail/MailInteractions.test.tsx`
- Modify: `web/src/i18n/messages.ts`
- Modify: `web/src/styles/app.css`

**Interfaces:**
- Produces: a `.message-document` reading surface with language-aware typography and safe styles for text/HTML mail.
- Preserves: sandbox boundaries and remote-content security behavior.

- [ ] **Step 1: Add failing reading-surface tests**

Assert a constrained reading measure, language marker, text/HTML switch labels, accessible blockquote/table/code treatment hooks, and no change to iframe sandbox attributes.

- [ ] **Step 2: Run focused tests and confirm failure**

Run: `cd web && npm run test:ci -- src/features/mail/MailInteractions.test.tsx`

- [ ] **Step 3: Add the dedicated reading stylesheet**

Use the following screen targets:

```css
--message-measure: 72ch;
--message-leading: 1.62;
--message-paragraph-gap: 0.9em;
--message-heading-leading: 1.24;
```

Use a system serif stack for long plain-text reading, retain sender and toolbar chrome in the system sans stack, render quotes with a quiet ink-blue left rule, and give tables/code a warm neutral surface. Do not force sender-authored HTML into the Kami color palette.

- [ ] **Step 4: Add language-aware type behavior**

Chinese reading uses a CJK serif fallback with modest tracking; English uses Charter/Georgia fallback with zero tracking. Keep body text at least 15px desktop and 16px mobile, with user zoom and system text scaling intact.

- [ ] **Step 5: Verify Task 6**

Run: `cd web && npm run typecheck && npm run test:ci -- src/features/mail/MailInteractions.test.tsx`

Expected: PASS and unchanged iframe sandbox attributes.

### Task 7: Complete responsive, accessibility, build, and screenshot verification

**Files:**
- Modify: `web/src/styles/app.css`
- Modify: `web/src/theme/tokens.css`
- Modify: `web/src/test/theme-bootstrap.test.js`
- Modify: `README.md`
- Replace: existing six UI screenshot assets referenced by `README.md`

**Interfaces:**
- Produces: verified desktop/tablet/mobile behavior and current README screenshots.

- [ ] **Step 1: Run static checks**

Run:

```bash
cd web
npm run typecheck
npm run test:ci
npm run build
```

- [ ] **Step 2: Verify Astryx usage and remove obsolete custom primitives**

Run:

```bash
rg -n 'className="(primary-button|secondary-button|quiet-button|icon-button|input-shell|modal-card|spinner)' web/src
rg -n 'transition:\s*all|scale\(0\)|ease-in\b' web/src
```

Expected: no application controls remain on the obsolete handcrafted primitive classes, and no prohibited motion patterns remain.

- [ ] **Step 3: Run Rust embedding checks**

Run:

```bash
cargo test --locked
cargo build --release --locked
```

- [ ] **Step 4: Perform responsive visual checks**

Capture and inspect Chinese and English views at 1440×960, 1024×768, and 390×844. Confirm no clipped tabs, hidden attachment actions, unreachable list scroll areas, stale focus rings, or horizontal page overflow.

- [ ] **Step 5: Perform keyboard and accessibility checks**

Verify Tab/Shift+Tab, Enter/Space, Escape, arrow-key tab navigation, J/K message navigation, Ctrl/Cmd+K search focus, dialog focus return, reduced motion, high contrast, and 44px mobile targets.

- [ ] **Step 6: Refresh README screenshots**

Replace the three desktop and three mobile screenshots while preserving README ordering and bilingual captions.

- [ ] **Step 7: Final repository verification**

Run:

```bash
git diff --check
git status --short
```

Expected: only intentional frontend, test, plan, lockfile, README, and screenshot changes.
