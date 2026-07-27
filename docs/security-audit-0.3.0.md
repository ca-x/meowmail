# Security audit notes for 0.3.0

Audit date: 2026-07-27

`npm audit --audit-level=high` reports no vulnerabilities, and all 372 audited npm packages have verified registry signatures; 78 also have verified attestations.

`cargo audit --no-fetch --stale` reports `RUSTSEC-2023-0071` for `rsa 0.9.10` with medium severity and no fixed release. The dependency path is:

```text
rsa 0.9.10 -> openidconnect 4.0.1 -> meowmail 0.3.0
```

The advisory concerns timing leakage in RSA private-key decryption/signing operations. Meowmail uses `openidconnect` only to validate OIDC provider signatures with public keys; it neither loads an RSA private key nor calls RSA signing/decryption. The affected operation is therefore not reachable in Meowmail's runtime paths. This is accepted for 0.3.0 and must be reviewed again when `openidconnect` or `rsa` publishes a compatible fix.

The MCP release review additionally verified and hardened irreversible/external side effects:

- permanent deletion requires a known cached IMAP UIDVALIDITY that matches the selected mailbox and confirms the UID still exists before server deletion;
- token permission is refreshed after request-body upload and immediately before SMTP/IMAP side effects;
- only `notifications/initialized` may omit a JSON-RPC id, so id-less `tools/call` cannot silently send or delete;
- body upload is limited to 2 MiB and 10 seconds, and unsupported supplied MCP protocol versions are rejected;
- interrupted `sending` drafts are marked `ambiguous` on startup to prevent blind duplicate delivery;
- one-time token responses use `Cache-Control: no-store, private` and `Pragma: no-cache`.
