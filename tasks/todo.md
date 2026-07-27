# Meowmail 0.3.0 MCP Tasks

- [x] Add MCP token and outgoing draft schema/entities.
  - Acceptance: existing 0.2.0 SQLite databases migrate additively; ownership cascades on user/account deletion.
  - Verify: migration/repository tests and `cargo test --locked`.
  - Files: `src/db/migration.rs`, `src/db/entities/*`.

- [x] Add personal token lifecycle API.
  - Acceptance: status, generate/rotate, permission update and revoke are session+CSRF protected; plaintext is returned once; DB stores only digest.
  - Verify: token rotation, revocation and two-user isolation tests.
  - Files: `src/mcp/api.rs`, `src/mcp/repository.rs`, `src/mcp/model.rs`.

- [x] Add MCP Streamable HTTP/JSON-RPC endpoint.
  - Acceptance: bearer auth and same-origin checks precede body parsing; initialize, ping, response-free notifications, tools/list and tools/call follow the documented contract.
  - Verify: router integration tests at `POST /mcp`.
  - Files: `src/mcp/protocol.rs`, `src/mcp/mod.rs`, `src/lib.rs`.

- [x] Implement bounded mail tools and drafts.
  - Acceptance: owned accounts/messages can be listed/read with hard result limits; replies honor Reply-To/References; atomically claimed drafts send via existing SMTP/proxy settings and uncertain delivery cannot be blindly retried.
  - Verify: repository/tool unit tests plus existing mail validation tests.
  - Files: `src/mcp/tools.rs`, `src/messages/repository.rs`, `src/accounts/repository.rs`.

- [x] Implement opt-in MCP deletion.
  - Acceptance: default denial; opt-in checked per call; server IMAP deletion succeeds before local cache deletion.
  - Verify: permission denial and owned-message deletion tests.
  - Files: `src/mcp/tools.rs`, `src/messages/repository.rs`.

- [x] Add concise bilingual Settings controls.
  - Acceptance: users can generate/copy/revoke token and toggle deletion; token is shown only for the generation response; Chinese and English text are complete.
  - Verify: `npm --prefix web run typecheck` and component tests.
  - Files: `web/src/features/settings/SettingsDialog.tsx`, `web/src/app/*`, `web/src/i18n/messages.ts`, CSS.

- [x] Update docs and version metadata.
  - Acceptance: Cargo/Web versions are 0.3.0; README explains MCP endpoint, bearer setup, tools and security behavior.
  - Verify: version searches and release workflow tag check.
  - Files: `Cargo.toml`, `Cargo.lock`, `web/package*.json`, `README.md`.

- [x] Add per-user sync fetch scope.
  - Acceptance: users can fetch all INBOX mail or the most recent 1–10000 messages; default and legacy value is 50; export/import includes the choice.
  - Verify: additive migration, sequence unit tests, repository validation, archive assertion, and bilingual component test.
  - Files: `src/db/migration.rs`, `src/cleanup/*`, `src/messages/api.rs`, `web/src/features/settings/SettingsDialog.tsx`.

- [ ] Verify and publish `v0.3.0`.
  - Acceptance: all release gates pass; main is pushed; GitHub Release, GHCR and Docker Hub publish successfully. LazyCat repository remains untouched.
  - Verify: local commands plus GitHub Actions/release/manifests.
  - Files: repository history and release metadata.
