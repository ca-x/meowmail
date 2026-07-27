# Spec: Per-user MCP access (0.3.0)

## Objective

Add a standards-compatible MCP endpoint that lets each Meowmail user connect an AI client to only that user's mail data. Users can generate or revoke one personal MCP bearer token, read mail, prepare replies and new messages as drafts, send drafts, and optionally allow destructive message deletion.

Acceptance contract:

- Each user owns at most one MCP token. Regeneration revokes the previous token immediately.
- The plaintext token is returned only by the generation response; SQLite stores only a SHA-256 digest of a 256-bit random token.
- MCP authentication uses `Authorization: Bearer …`; query-string tokens and browser sessions are not accepted on `/mcp`.
- Authentication and same-origin checks run before the JSON-RPC body is parsed. Authentication is repeated after the bounded, timed body read so rotation/revocation takes effect before dispatch. Non-browser clients may omit `Origin`; browser requests must use an HTTP(S) Origin whose authority exactly matches `Host`.
- Every account, message, draft and delete operation is scoped by the authenticated token's user ID.
- Reading, draft creation, reply-draft creation and sending are enabled when a token exists.
- MCP deletion is disabled by default and checked server-side for every delete call.
- Deletion requires matching cached/current IMAP UIDVALIDITY, confirms the UID still exists, rechecks current token permission immediately before `UID STORE`, removes the server message, and only then removes its local cache row. A mismatch or mail-server failure leaves the local row intact.
- MCP tokens are never included in configuration export/import.
- The settings UI is concise, keyboard accessible and fully localized in Chinese and English.

## MCP Contract

Endpoint: `POST /mcp`

Protocol support:

- JSON-RPC 2.0
- MCP protocol version `2025-03-26`
- `initialize`, `ping`, `notifications/initialized`, `tools/list`, `tools/call`
- Stateless Streamable HTTP responses using `application/json`
- Only `notifications/initialized` is accepted without an id; it is processed without a response body and returns HTTP 202. Request methods such as `tools/call` require an id.
- A supplied `MCP-Protocol-Version` header must equal `2025-03-26`; the documented missing-header compatibility default remains supported.

Tools:

| Tool | Purpose | Destructive |
| --- | --- | --- |
| `list_mail_accounts` | List the user's configured mail accounts | No |
| `search_emails` | Search cached messages with bounded result count | No |
| `read_email` | Read one cached message as plain text | No |
| `create_email_draft` | Persist a new outgoing draft | No |
| `create_reply_draft` | Persist a reply draft using Reply-To plus In-Reply-To/References threading | No |
| `list_email_drafts` | List up to 20 user-owned MCP drafts and their delivery state | No |
| `send_email_draft` | Atomically claim and send one owned draft through its account | External side effect |
| `delete_email` | Permanently delete one owned message from server and local cache | Yes; requires explicit user opt-in |

All tool arguments and results have size/count limits, including values reloaded from SQLite. Tool failures use MCP `isError: true` content without exposing internal error details. Draft delivery states are `draft`, `sending`, `ambiguous`, and `sent`; only `draft` can be claimed for sending. A mail transport error becomes `ambiguous` so an AI cannot blindly retry and duplicate a message. Startup reconciles interrupted `sending` claims to `ambiguous`.

## Tech Stack

- Rust 1.94, Axum 0.8, SeaORM 1.1, SQLite
- React 19, TypeScript 7, Vite 8
- Existing SMTP, IMAP, proxy and credential-vault modules
- No additional runtime dependency is required for the JSON-RPC surface

## Commands

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
npm --prefix web audit --audit-level=high
npm --prefix web audit signatures
npm --prefix web run typecheck
npm --prefix web run test:ci
npm --prefix web run build
cargo build --release --locked
```

## Project Structure

```text
src/mcp/                  MCP HTTP, token API, repository and tool execution
src/db/entities/          MCP token and email draft SeaORM entities
src/db/migration.rs       0.3.0 additive migration
web/src/features/settings MCP token controls inside the existing settings dialog
web/src/app/              MCP settings API types/client methods
web/src/i18n/             Chinese and English copy
tests/                    Rust public-boundary integration tests where practical
```

## Code Style

External input is deserialized into explicit types, validated once at the route/tool boundary, and passed to user-scoped repository methods:

```rust
let token = repository.authenticate(&bearer).await?;
let message = messages.get(token.user_id, input.message_id).await?;
```

Use snake_case MCP argument names, camelCase browser API responses, UUID resource IDs, parameterized SeaORM queries, and generic external-service errors.

## Testing Strategy

Public seams under test:

- MCP JSON-RPC endpoint: auth-before-body parsing, post-body token refresh, timed body reads, parse/invalid-request errors, restricted notification semantics, protocol-version and Origin rejection, initialize/tool listing, unknown methods and deletion permission denial.
- MCP repository: token rotation/revocation and cross-user isolation.
- Tool input helpers: recipient/subject/body limits and reply subject generation.
- Settings UI: token status, one-time token display, deletion toggle and bilingual copy.
- Existing full Rust/Web build and dependency-audit commands remain release gates.

## Threat Model and Boundaries

Trust boundaries are the bearer header, JSON-RPC payload, AI-generated tool arguments, stored email content, IMAP/SMTP responses and SQLite rows. Assets are mail content, account credentials, user identity and destructive mail actions.

- Always: authenticate before and after reading the JSON-RPC body; enforce Streamable HTTP Origin/version protection; scope every query by user ID; bound list/body/result sizes; compare token digests in constant time; validate UIDVALIDITY before permanent deletion; return plain text; audit token lifecycle and destructive calls without logging token or mail body.
- Ask first: expanding MCP permissions beyond the tools in this spec or allowing permanent deletion by default.
- Never: store/log plaintext MCP tokens, accept tokens in URLs, expose account passwords, trust prompt instructions as authorization, or export MCP credentials.

## Success Criteria

- A user can generate, copy once, rotate and revoke a token from Settings.
- Two users' tokens cannot access each other's accounts, messages or drafts.
- A conforming MCP client can initialize, list tools and invoke all non-destructive tools.
- `delete_email` returns a permission error until the user explicitly enables MCP deletion.
- Sending uses the selected owned account's existing SMTP/proxy configuration.
- Each user can sync the entire INBOX or a validated recent count from 1 to 10000; legacy and new settings default to 50.
- Concurrent calls cannot claim the same draft twice; uncertain SMTP outcomes remain blocked from automatic retry.
- Interrupted sends become `ambiguous` on restart; id-less request methods cannot trigger side effects; token rotation during body upload prevents dispatch.
- README documents endpoint setup, tool list and the deletion/security model.
- Version `0.3.0` is tested, committed, pushed, tagged and published through the existing GitHub binary and Docker workflows.

## Open Questions

None. The requested feature and destructive-action default are explicit; the implementation choices above favor least privilege and the existing Meowmail architecture.
