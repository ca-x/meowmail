# Meowmail 0.3.0 MCP Implementation Plan

1. Add additive SQLite migrations and SeaORM entities for one per-user MCP token and user-owned outgoing drafts.
2. Implement token lifecycle APIs and bearer authentication with hashed storage, constant-time comparison, revocation and permission state.
3. Implement the stateless MCP JSON-RPC endpoint and bounded mail tools over existing account/message/IMAP/SMTP repositories.
4. Add server-and-local deletion with an explicit per-user MCP permission gate.
5. Add concise bilingual MCP controls to Settings and component tests for token lifecycle and delete opt-in.
6. Update version metadata and README MCP client instructions.
7. Add a per-user INBOX fetch scope with all-mail or validated recent-count modes, defaulting to 50 and included in configuration archives.
8. Run security audits, Rust/Web tests, release build and smoke checks.
9. Commit, push, create `v0.3.0`, and monitor GitHub binary/Docker publication. Do not modify the LazyCat application repository.

## Risks and Mitigations

- Bearer-token theft: display once, store only a digest, never accept URL tokens, never log authorization headers.
- Cross-tenant access: every repository lookup includes token-derived `user_id`; tests use two users.
- Prompt injection/excessive agency: permissions are enforced in Rust, arguments are validated, results are bounded, and delete defaults off.
- Accidental deletion: delete is a separately advertised tool, requires current opt-in, and removes local data only after the IMAP delete succeeds.
- Duplicate sending: atomically transition drafts from `draft` to `sending`; SMTP transport errors become `ambiguous`, and only `draft` may be claimed.
- Protocol incompatibility: implement MCP `2025-03-26` initialize, response-free notifications, ping, tools/list and tools/call with strict JSON-RPC envelopes.
- DNS rebinding/browser abuse: authenticate before body parsing and reject HTTP(S) Origin authorities that do not exactly match Host.

## Verification Checkpoints

- Migration/entity compilation and token repository tests pass.
- MCP endpoint contract and permission tests pass.
- Frontend typecheck/component tests pass in Chinese and English.
- Existing databases receive a default fetch limit of 50; all-mail and configured-count sequences are covered by tests.
- Rust fmt/clippy/tests and npm audit/signature/build pass.
- Release binary reports 0.3.0 and serves `/mcp` with bearer authentication.
- GitHub Release and amd64/arm64 GHCR/Docker Hub manifests complete for `v0.3.0`.
