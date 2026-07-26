# Meowmail 0.2.0 Tasks

- [ ] Add users, identities, ownership, profile/avatar, and per-user settings schema.
  - Acceptance: a fresh 0.2.0 database initializes with all user-owned tables and constraints.
  - Verify: migration and repository tests.
  - Files: `src/db/migration.rs`, `src/db/entities/*`, `src/db.rs`.

- [ ] Implement local users, bootstrap administrator, roles, and user-aware sessions.
  - Acceptance: passwords are Argon2id hashed; cookies/CSRF remain protected; existing passwords are not reset.
  - Verify: `tests/auth_api.rs`.
  - Files: `src/auth.rs`, `src/users/*`, `src/config.rs`, `src/lib.rs`.

- [ ] Implement OIDC discovery and Authorization Code + PKCE login.
  - Acceptance: issuer/state/nonce/signature/audience/expiry are verified and identity uses issuer+subject.
  - Verify: config/state/provisioning tests plus local provider smoke where available.
  - Files: `src/auth.rs`, `src/config.rs`, `Cargo.toml`, `Cargo.lock`.

- [ ] Add personal PIN lock/unlock.
  - Acceptance: PIN is optional, per-user Argon2id hash, and never accepted as primary login.
  - Verify: lock/unlock integration tests.
  - Files: `src/auth.rs`, `src/users/*`, `tests/auth_api.rs`.

- [ ] Scope mail accounts/messages/notifications by user.
  - Acceptance: two users cannot access each other's resources; notification hooks use the account owner settings.
  - Verify: two-user integration tests.
  - Files: `src/accounts/*`, `src/messages/*`, `src/notifications/*`.

- [ ] Add per-user mail retention and automatic cleanup rules.
  - Acceptance: server-deleted mail can remain locally; rules match optional account/sender/age/subject/body criteria; server deletion is explicit.
  - Verify: rule matching, local reconciliation, and owned-account enforcement tests.
  - Files: `src/cleanup/*`, `src/messages/*`, `src/db/*`, `tests/*`.

- [ ] Add nickname/avatar and encrypted export/import migration.
  - Acceptance: profile edits persist; avatar type/size is validated; selectable personal sections round trip; administrators can choose all users or only their own configuration; all-user conflicts are reported safely.
  - Verify: profile/archive tests.
  - Files: `src/users/*`, `src/security.rs`, `tests/*`.

- [ ] Update React authentication and settings UX in Chinese and English.
  - Acceptance: local/OIDC/hybrid login, locked screen, profile, PIN, avatar, export/import, retention, and cleanup-rule controls are usable and localized.
  - Verify: typecheck and component tests.
  - Files: `web/src/app/*`, `web/src/features/auth/*`, `web/src/features/settings/*`, `web/src/i18n/messages.ts`, CSS.

- [ ] Update version, docs, Docker, and CI smoke tests.
  - Acceptance: all displayed/package versions are 0.2.0 and docs describe new environment variables and migration behavior.
  - Verify: release build and container smoke test.
  - Files: `Cargo.toml`, `web/package.json`, `.env.example`, `README.md`, workflows.

- [ ] Update and verify the LazyCat package.
  - Acceptance: single instance, OIDC-only, no PIN injection, file chooser interception, 0.2.0 package, dual-store workflow valid.
  - Verify: `actionlint`, `lzc-cli project release`, `lzc-cli lpk info`.
  - Files: `/home/czyt/code/rust/meowmail-lazycat/*`.

- [ ] Commit, push, tag, and publish 0.2.0.
  - Acceptance: both repositories pushed; GitHub Release, GHCR, Docker Hub, binary assets, LPK asset, and enabled stores report the expected version or an explicit external failure.
  - Verify: GitHub workflow/release/store status and asset SHA256.
  - Files: Git history and release metadata.
