# Spec: Meowmail 0.2.0 Multi-user Authentication and Migration

## Objective

Meowmail 0.2.0 turns the existing single-user PIN-gated application into a multi-user Web mail client while keeping SQLite and SeaORM. Each application user owns an isolated set of mail accounts, cached messages, preferences, notification hooks, mail retention/cleanup rules, profile data, and an optional personal app-lock PIN.

The general distribution supports local accounts, OIDC, or both. The LazyCat package is single-instance and OIDC-only. The first OIDC user becomes administrator only when no administrator exists. A local administrator can be bootstrapped from environment variables.

User migration uses an encrypted export archive with selectable sections: profile/avatar, mail accounts, notifications, and retention/cleanup rules. Ordinary users can export/import only their own selected sections. Administrators can choose either “my configuration only” or “all users”. An all-user archive additionally carries roles, local password hashes, PIN hashes, and OIDC issuer/subject mappings so a complete instance can be migrated. No archive contains plaintext login passwords/PINs, OIDC tokens, OIDC client secrets, or sessions. Cleanup-rule account references are exported by mail address and remapped during import.

## Assumptions and Decisions

1. Local authentication uses username/password and server-side cookie sessions.
2. OIDC uses Authorization Code Flow with PKCE, state, nonce, issuer/audience/signature/expiry verification, and the identity key `(issuer, subject)`.
3. `MEOWMAIL_AUTH_MODE` accepts `local`, `oidc`, or `hybrid`; the default is `local`.
4. `MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME` and `MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD` create the first local administrator only when that username does not already exist. They do not reset an existing password on restart.
5. `MEOWMAIL_OIDC_FIRST_USER_ADMIN` defaults to `true`, but elevates the first OIDC user only if no administrator exists.
6. The personal PIN is an optional post-login app lock. It does not replace local/OIDC login and is hashed independently per user with Argon2id.
7. Version 0.2.0 uses a clean initial multi-user schema. In-place migration from 0.1.x is intentionally out of scope because 0.1.0 was not deployed.
8. Mail credentials remain encrypted at rest by an installation-level vault key. `MEOWMAIL_VAULT_KEY` may provide a stable deployment secret; otherwise a random key is created under the data directory.
9. Personal import merges selected sections into the current user, restores retention and cleanup rules after remapping account references, and cannot create users or grant roles. All-user import is administrator-only and may restore users/roles/authentication hashes/identities while reporting identity and username conflicts instead of silently replacing them.
10. Avatar uploads accept PNG, JPEG, or WebP up to 512 KiB and are served only to authenticated users.
11. A personal mail-retention option controls whether a local cached copy survives when a message disappears from the IMAP server; the default is to retain the local copy.
12. Cleanup rules combine optional account, sender, age/received-time, subject, and body predicates. Rules default to local-cache cleanup; IMAP deletion must be explicitly enabled per rule.

## Tech Stack

- Rust 1.94, Axum 0.8, SeaORM/SeaORM Migration 1.1, SQLite
- `openidconnect` 4.x for OIDC discovery, authorization, token exchange, and ID token verification
- Argon2id for local passwords, app-lock PINs, and export-passphrase key derivation
- XChaCha20-Poly1305 for mail credentials and encrypted migration archives
- React 19, TypeScript 7, Vite 8
- LazyCat LPK v2, single instance, OIDC redirect path `/api/v1/auth/oidc/callback`

## Configuration

```text
MEOWMAIL_AUTH_MODE=local|oidc|hybrid
MEOWMAIL_BOOTSTRAP_ADMIN_USERNAME=<username>
MEOWMAIL_BOOTSTRAP_ADMIN_PASSWORD=<password>
MEOWMAIL_OIDC_ISSUER=<https issuer>
MEOWMAIL_OIDC_CLIENT_ID=<client id>
MEOWMAIL_OIDC_CLIENT_SECRET=<client secret>
MEOWMAIL_OIDC_REDIRECT_URL=<absolute callback URL>
MEOWMAIL_OIDC_SCOPES="openid profile email"
MEOWMAIL_OIDC_FIRST_USER_ADMIN=true|false
MEOWMAIL_VAULT_KEY=<installation secret, optional>
MEOWMAIL_DATA_DIR=/data
MEOWMAIL_BIND=0.0.0.0:8080
```

For LazyCat, `MEOWMAIL_OIDC_*` values are mapped from `LAZYCAT_AUTH_OIDC_*`, `MEOWMAIL_AUTH_MODE=oidc`, and `application.multi_instance=false`.

## Commands

```bash
# Backend checks
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --release --locked

# Frontend checks
npm --prefix web ci --ignore-scripts
npm --prefix web audit --audit-level=high
npm --prefix web audit signatures
npm --prefix web run typecheck
npm --prefix web run test:ci
npm --prefix web run build

# Container
docker build --tag meowmail:0.2.0 .

# LazyCat
lzc-cli project release -o .lazycat-build/meowmail.lpk
lzc-cli lpk info .lazycat-build/meowmail.lpk
actionlint
```

## Project Structure

```text
src/auth.rs                 authentication, sessions, OIDC flow, app lock
src/users/                  user/profile/admin/export-import APIs and repositories
src/db/entities/            SeaORM entities
src/db/migration.rs         clean 0.2.0 initial schema
src/accounts/               user-scoped mail accounts
src/messages/               user-scoped cached messages
src/notifications/          user-scoped hook settings and dispatch
src/cleanup/                user mail retention and automatic cleanup rules
web/src/features/auth/      local/OIDC login and PIN unlock views
web/src/features/settings/  profile, security, migration, notification settings
tests/                      auth, isolation, migration, archive tests
docs/                       configuration and release documentation
```

## Code Style

Rust validates external values before repository calls and always carries the authenticated user ID into data access:

```rust
async fn list(
    State(state): State<AppState>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<MailAccount>>, AppError> {
    Ok(Json(repository(&state).list(session.user_id).await?))
}
```

TypeScript uses typed API responses and localized copy; authentication secrets never enter browser storage.

## Threat Model

- Spoofing: local password verification, OIDC signature/issuer/audience/nonce/state/PKCE checks, HttpOnly session cookie.
- Tampering: CSRF on state-changing requests, authenticated export/import, AEAD-protected archives.
- Information disclosure: tenant filters on every mail/profile/notification query, generic auth errors, no token/secret logging.
- Denial of service: bounded login attempts, OIDC-flow TTL/count limits, request body limits, avatar/archive size limits, HTTP timeouts.
- Elevation of privilege: server-side role checks and transactional first-admin assignment; personal import never changes user/role/auth credentials.
- Migration authorization: personal archives cannot carry roles/auth identities; all-user export/import requires a live administrator session and uses conflict-safe merges.
- Destructive mail actions: server deletion is opt-in per rule, rule previews are bounded, and IMAP deletion applies only to mail accounts owned by the current user.

## Testing Strategy

- Unit tests for configuration validation, password/PIN hashing, encrypted archive round trips, and OIDC flow-state expiry.
- Integration tests for local login, session locking/unlocking, first-admin provisioning, CSRF, and cross-user resource isolation.
- Frontend component tests for local/OIDC login variants, locked state, localized profile/security/migration controls.
- Full Rust and frontend checks, release build, container smoke test, LazyCat lint/build, and Action workflow validation before tagging.

## Boundaries

- Always: hash passwords/PINs, validate OIDC and uploaded content, scope repositories by user ID, use transactions for role assignment and legacy claims, keep secrets out of logs and Git.
- Ask first: changing the public package ID, enabling role import, adding token persistence, or deleting historical release artifacts.
- Never: trust email as the OIDC identity key, accept unsigned ID tokens, store passwords/PINs/tokens in plaintext, expose another user's data, or let import grant administrator privileges.

## Success Criteria

- Two authenticated users cannot list, read, edit, sync, send through, or receive notifications for each other's mail accounts/messages/settings.
- Local bootstrap admin login works without resetting the password on restart.
- OIDC login validates the complete flow and auto-provisions users; the first OIDC user becomes admin only when no admin exists.
- A user can set a PIN, lock the current session, and unlock it with that PIN; PIN is not accepted as primary login.
- A user can update nickname/avatar and round-trip selected encrypted migration sections, including retention and cleanup rules, into their own account.
- An administrator can export/import all users or explicitly choose “my configuration only”; ordinary users cannot invoke all-user migration.
- A user can retain local copies after server deletion, or disable retention and reconcile local cache during sync.
- A user can create sender/time/subject/body cleanup rules; local-only and explicit server-delete actions run only against owned accounts.
- LazyCat manifest is single-instance, OIDC-only, contains no browser password injection, and supports avatar/import/export file selection.
- Version fields are `0.2.0`; all tests/builds/audits pass before `v0.2.0` is pushed and released.

## Open Questions

None blocking. Full mailbox cache export, OIDC logout propagation, multi-provider OIDC, and administrator UI for bulk user lifecycle management are deferred beyond 0.2.0.
