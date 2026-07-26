# Meowmail Web Mail Client Implementation Plan

> Historical note: this superseded pre-release MVP plan describes the abandoned single-user PIN design. Meowmail 0.2.0 is the first published version; the current multi-user/OIDC contract is in `docs/specs/multi-user-oidc-0.2.0.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a self-hosted Web mail client named 妙邮 / Meowmail with one PIN-protected application session, multiple IMAP/SMTP mail accounts, per-account HTTP/SOCKS5 proxy support, configurable command/HTTP notifications, themes, Chinese/English UI, SQLite storage, and automated release builds.

**Architecture:** A single Axum binary owns authentication, SQLite persistence, encrypted mail credentials, IMAP synchronization, SMTP delivery, notification hooks, and embedded SPA assets. The React 19 + Vite frontend consumes a versioned JSON API, keeps authentication in an HttpOnly cookie, and stores only non-sensitive appearance preferences in local storage. Release builds compile the Vite bundle first and embed it into the Rust executable, following Raindrop's delivery model.

**Tech Stack:** Rust 2024, Axum, Tokio, SQLx/SQLite, XChaCha20-Poly1305, async-imap, rustls, React 19, TypeScript, Vite, Vitest, Playwright, Docker, GitHub Actions.

## Global Constraints

- Chinese product name is `妙邮`; English product name is `Meowmail`; executable and repository name are `meowmail`.
- `MEOWMAIL_PIN` is the only security-setting environment variable and is required at startup.
- Mail credentials and proxy passwords are encrypted at rest with a key derived from `MEOWMAIL_PIN`; secret fields never appear in API responses or logs.
- The application is single-user. “Multiple accounts” means multiple external IMAP/SMTP mail accounts.
- Each mail account independently supports direct, HTTP CONNECT, or SOCKS5 connectivity for both IMAP and SMTP.
- SQLite is the only database backend for this release.
- Mail connections must use implicit TLS or STARTTLS; plaintext credential transport is not supported.
- Notification commands are executed without a shell. The executable is fixed by configuration and mail-controlled placeholders are substituted only into already-parsed arguments.
- Notification placeholders include `{account}`, `{email}`, `{sender}`, `{sender_email}`, `{subject}`, `{preview}`, and `{message}`.
- Frequent mail navigation and keyboard actions remain instant; modal, popover, and toast motion stays below 300 ms and uses transform/opacity with the Emil design engineering easing curves.
- Apple-inspired design is applied through system typography, restrained translucent chrome, strong spatial consistency, immediate pointer-down feedback, and adaptive reduced-transparency/high-contrast behavior; mail content surfaces remain solid and readable.

---

### Task 1: Rust foundation, configuration, SQLite, and protected session

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `build.rs`, `.env.example`
- Create: `src/lib.rs`, `src/main.rs`, `src/config.rs`, `src/db.rs`, `src/security.rs`, `src/auth.rs`, `src/error.rs`, `src/web.rs`
- Test: `tests/auth_api.rs`, `tests/security.rs`, `tests/embedded_web.rs`

**Interfaces:**
- Produces: `Config::from_env() -> Result<Config>`, `Database::connect(&Path) -> Result<Database>`, `CredentialVault::{seal,open}`, `SessionStore`, and `build_router(AppState) -> Router`.
- Produces API: `GET /api/v1/health`, `GET /api/v1/session`, `POST /api/v1/auth/login`, `POST /api/v1/auth/logout`.

- [ ] Write tests proving a missing/short PIN fails startup validation, vault ciphertext round-trips but rejects a different PIN, unauthenticated sessions return 401, login creates an HttpOnly SameSite cookie, invalid PIN attempts are throttled, and mutation endpoints reject a missing CSRF header.
- [ ] Implement strict configuration parsing with bounded PIN length, safe default bind/data paths, redacted debug output, and no secret logging.
- [ ] Create SQLite migrations for accounts, messages, preferences, and notification settings; enable WAL, foreign keys, and a busy timeout.
- [ ] Implement XChaCha20-Poly1305 encryption using an Argon2id-derived key and a persistent random vault salt stored with owner-only permissions where supported.
- [ ] Implement in-memory expiring sessions and per-address login throttling. Return a CSRF token only after successful PIN verification.
- [ ] Add security headers including CSP, frame denial, nosniff, referrer policy, and permissions policy. Serve SPA assets only for non-API routes.
- [ ] Run `cargo test --test auth_api --test security --test embedded_web` and ensure all tests pass.

### Task 2: Multi-mail-account persistence and per-account proxies

**Files:**
- Create: `src/accounts/mod.rs`, `src/accounts/model.rs`, `src/accounts/repository.rs`, `src/accounts/api.rs`
- Create: `src/mail/mod.rs`, `src/mail/proxy.rs`, `src/mail/tls.rs`
- Test: `tests/account_api.rs`, `tests/proxy_protocol.rs`

**Interfaces:**
- Consumes: authenticated/CSRF-protected API state, `Database`, and `CredentialVault` from Task 1.
- Produces: `MailAccount`, `AccountSecrets`, `ProxyConfig`, `connect_via_proxy(target, proxy)`, and account CRUD/test routes.
- Produces API: `GET/POST /api/v1/accounts`, `PATCH/DELETE /api/v1/accounts/:id`, `POST /api/v1/accounts/test`, `POST /api/v1/accounts/:id/test`.

- [ ] Write API tests showing multiple mail accounts can coexist, deleting one does not affect another, secret fields never round-trip, and invalid ports/hosts/security/proxy combinations receive validation errors.
- [ ] Implement account DTOs for display name, email, username, IMAP/SMTP host/port/security, default-account state, and proxy kind/host/port/optional username/password.
- [ ] Encrypt email and proxy passwords before SQL insertion. Preserve an existing secret when an update omits its replacement value.
- [ ] Implement direct TCP, HTTP CONNECT including optional Basic authentication, and SOCKS5 including optional username/password authentication. Add DNS/connection/handshake timeouts and response-size limits.
- [ ] Implement rustls wrapping and protocol-specific STARTTLS negotiation while rejecting plaintext authentication.
- [ ] Implement connection tests that authenticate against both IMAP and SMTP through the selected per-account proxy without persisting a draft account test.
- [ ] Run `cargo test --test account_api --test proxy_protocol` and ensure all tests pass.

### Task 3: Inbox synchronization, local message index, and SMTP sending

**Files:**
- Create: `src/mail/imap.rs`, `src/mail/smtp.rs`, `src/mail/message.rs`, `src/messages/api.rs`, `src/messages/repository.rs`, `src/messages/mod.rs`
- Test: `tests/message_repository.rs`, `tests/mail_parsing.rs`, `tests/smtp_protocol.rs`

**Interfaces:**
- Consumes: `MailAccount`, decrypted `AccountSecrets`, `connect_via_proxy`, and `Database`.
- Produces: `sync_inbox(account) -> SyncResult`, `send_message(account, ComposeRequest)`, message list/detail queries, and local read/star updates.
- Produces API: `POST /api/v1/accounts/:id/sync`, `GET /api/v1/messages`, `GET/PATCH /api/v1/messages/:id`, `POST /api/v1/messages/send`.

- [ ] Write parsing tests with multipart UTF-8 fixtures and repository tests proving account isolation, UID de-duplication, unread/star filtering, search, and newest-first ordering.
- [ ] Authenticate with IMAP, select INBOX, fetch the newest bounded window using `BODY.PEEK[]`, parse RFC 5322/MIME data, sanitize HTML, and upsert by `(account_id, folder, uid)`.
- [ ] Store sender, recipients, subject, preview, sanitized text/HTML bodies, timestamps, attachment count, and read/star flags. Never store raw authentication responses.
- [ ] Add list filters for account, folder, unread, starred, attachment, and search. Keep detail bodies out of list responses.
- [ ] Build RFC-compliant MIME messages and send them via SMTP AUTH PLAIN over implicit TLS or STARTTLS, including dot stuffing and bounded responses.
- [ ] Add a compose endpoint with validated sender account, recipient limits, subject/body limits, and generic external-service errors.
- [ ] Run `cargo test --test message_repository --test mail_parsing --test smtp_protocol` and ensure all tests pass.

### Task 4: Safe command and HTTP notification hooks

**Files:**
- Create: `src/notifications/mod.rs`, `src/notifications/model.rs`, `src/notifications/template.rs`, `src/notifications/runner.rs`, `src/notifications/api.rs`
- Test: `tests/notification_templates.rs`, `tests/notification_runner.rs`, `tests/notification_api.rs`

**Interfaces:**
- Consumes: newly inserted messages from `sync_inbox`, authenticated settings API, and `Database`.
- Produces: `NotificationEvent`, `render_template`, `NotificationRunner::dispatch`, notification settings/test routes.
- Produces API: `GET/PATCH /api/v1/notifications/settings`, `POST /api/v1/notifications/test`.

- [ ] Write tests proving every documented placeholder renders, unknown placeholders fail validation, mail content cannot change the configured executable or add shell syntax, HTTP endpoints are fixed HTTP/HTTPS URLs, and commands/requests time out.
- [ ] Store enabled state, message template, optional command template, and optional fixed webhook URL. Default message template is `[{account}] {sender}: {subject}`.
- [ ] Parse the command template with shell-style tokenization before placeholder substitution, reject placeholders in argv[0], invoke with `tokio::process::Command`, clear sensitive inherited variables where practical, and never use a shell.
- [ ] POST JSON to the configured HTTP/HTTPS URL containing `message`, `account`, `email`, `sender`, `senderEmail`, `subject`, and `preview`; cap request/response sizes and timeouts.
- [ ] Dispatch hooks only for messages newly inserted by synchronization, isolate failures from synchronization success, and record only redacted status text.
- [ ] Run `cargo test --test notification_templates --test notification_runner --test notification_api` and ensure all tests pass.

### Task 5: React application, login, mail workspace, account settings, themes, and locales

**Files:**
- Create: `web/package.json`, `web/package-lock.json`, `web/tsconfig.json`, `web/vite.config.ts`, `web/index.html`
- Create: `web/src/main.tsx`, `web/src/app/App.tsx`, `web/src/app/api.ts`, `web/src/app/types.ts`
- Create: `web/src/i18n/I18nProvider.tsx`, `web/src/i18n/messages.ts`
- Create: `web/src/theme/ThemeProvider.tsx`, `web/src/theme/tokens.css`, `web/src/styles/app.css`
- Create: `web/src/features/auth/LoginPage.tsx`, `web/src/features/mail/MailWorkspace.tsx`, `web/src/features/mail/MessageList.tsx`, `web/src/features/mail/MessageDetail.tsx`, `web/src/features/mail/ComposeDialog.tsx`, `web/src/features/accounts/AccountDialog.tsx`, `web/src/features/settings/SettingsDialog.tsx`
- Create: `web/public/meowmail-logo.png`
- Test: `web/src/**/*.test.tsx`, `web/e2e/app.spec.ts`

**Interfaces:**
- Consumes: all JSON APIs from Tasks 1–4.
- Produces: `/login` PIN flow and authenticated responsive application routes.

- [ ] Copy the supplied transparent cat-envelope logo into the Web public assets and create accessible alt text for both 妙邮 and Meowmail contexts.
- [ ] Implement application bootstrapping: a 401 routes to `/login`, successful PIN login returns to `/mail/inbox`, logout clears the session, and all mutations attach the in-memory CSRF token.
- [ ] Implement Chinese and English dictionaries with browser-language default and a persisted locale selector.
- [ ] Implement system/light/dark themes with pre-paint bootstrap, design tokens, AA contrast, and no flash of the wrong theme.
- [ ] Build the desktop three-column workspace, tablet two-pane navigation, and mobile list/detail route transitions. Include account switcher, folders, search, filters, sync, message selection, read/star actions, empty/loading/error states, and relative timestamps.
- [ ] Build account add/edit UI with Gmail, Outlook, and custom presets plus per-account direct/HTTP/SOCKS5 proxy fields and password-preservation semantics.
- [ ] Build compose and settings dialogs, including notification command/webhook templates and a visible placeholder reference.
- [ ] Apply Emil design engineering constraints: exact-property transitions, 0.97 active scale, origin-aware popovers, centered modal transforms, under-300-ms enter/exit, reduced-motion handling, and hover effects gated to fine pointers.
- [ ] Apply Apple design constraints: platform system fonts with optical sizing, translucent floating chrome only where it communicates hierarchy, solid content planes, direct press feedback, symmetric enter/exit paths, and `prefers-reduced-transparency` / `prefers-contrast` fallbacks.
- [ ] Run `npm --prefix web run typecheck`, `npm --prefix web run test:ci`, and `npm --prefix web run build` and ensure all pass.

### Task 6: Automated local, container, CI, and release builds

**Files:**
- Create: `Makefile`, `Dockerfile`, `.dockerignore`, `README.md`
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/dependabot.yml`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: production Vite bundle and Rust application from Tasks 1–5.
- Produces: repeatable local commands, a minimal container, CI checks, and GitHub release archives.

- [ ] Add `make dev`, `make web`, `make test`, `make build`, and `make docker` targets with fail-fast shell behavior.
- [ ] Make release Rust builds fail if `web/dist/index.html` is absent and embed the complete built asset tree with immutable caching for hashed files and no-cache for `index.html`.
- [ ] Add a multi-stage Docker build and non-root runtime image with a persistent `/data` volume; document `MEOWMAIL_PIN` startup.
- [ ] Add CI jobs for locked npm install/audit/signatures, TypeScript/Vitest/build, Rust fmt/clippy/test, and a production embedded-Web smoke test.
- [ ] Add tagged release builds for Linux x86_64/aarch64, Windows x86_64, and macOS x86_64/aarch64 with checksums and GitHub Release publishing.
- [ ] Document account setup, app-password expectations, HTTP/SOCKS5 proxies, notification templates, security boundaries, backup guidance, and all build commands.
- [ ] Run the full release verification: `npm --prefix web ci --ignore-scripts`, `npm --prefix web audit --audit-level=high`, `npm --prefix web run typecheck`, `npm --prefix web run test:ci`, `npm --prefix web run build`, `cargo fmt --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked`, and `cargo build --release --locked`.
