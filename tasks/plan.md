# Meowmail 0.2.0 Implementation Plan

1. Define a clean multi-user 0.2.0 schema; 0.1.x database migration is out of scope because 0.1.0 was not deployed.
2. Replace global PIN authentication with local/OIDC authentication, user-aware sessions, roles, and optional app-lock PIN.
3. Scope mail accounts, messages, and notification settings by authenticated user in the clean first-release schema.
4. Add per-user server-deletion retention and sender/time/subject/body cleanup rules executed during sync.
5. Add profile/avatar APIs plus encrypted user migration export/import.
6. Update React login, locked state, profile/security/migration/mail-cleanup settings, and complete Chinese/English copy.
7. Update environment docs, smoke tests, version numbers, Docker usage, and release notes.
8. Convert LazyCat package to single-instance OIDC-only, add file chooser interception, update automation for 0.2.0, and build the LPK.
9. Run security/dependency/build verification, commit and push both repositories, create `v0.2.0`, and monitor GitHub/LazyCat publishing.

## Risks and Mitigations

- OIDC provider incompatibility: use standards-compliant discovery and document exact required environment variables.
- Cross-tenant leakage: require user ID in every repository method and add two-user integration tests.
- Vault durability: generate a protected installation key by default, with an optional stable `MEOWMAIL_VAULT_KEY` for controlled deployments.
- Export secret leakage: encrypt archives with Argon2id + XChaCha20-Poly1305 and exclude authentication/role data.
- Destructive cleanup mistakes: default to local-only cleanup, require explicit server deletion, and always scope rules to owned accounts.
- LazyCat SSO mismatch: use the documented OIDC redirect path and injected environment variables; remove all DOM credential injection.

## Verification Checkpoints

- Schema/entities compile and migration tests pass.
- Auth and isolation integration tests pass.
- Frontend typecheck/component tests pass in both locales.
- Rust fmt/clippy/tests and npm audit/build pass.
- Docker image starts with local auth and reports 0.2.0.
- LazyCat actionlint, LPK build/info, package/version/asset SHA checks pass.
