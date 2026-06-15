# PRD — Harden residual authenticated web API vectors (post-audit follow-up)

## Context
Follow-up to the archived audit `06-14-audit-and-optimize-cc-switch-web-post-refactor-codex-co-review`
(branch `fix/web-audit-phase1-2`). That audit wired HTTP Basic auth (C2), so the whole `/api` is now
authenticated. A prior broad security review (recorded in memory `cc-switch-web-web-api-auth-unwired`)
flagged 4 residual items that were UNAUTHENTICATED then and are AUTHENTICATED-only now. Deployment =
single-user Linux host + Tailscale + Win10 browser. Continue on branch `fix/web-audit-phase1-2`.

## Scope decision (verified firsthand 2026-06-15)

### R1 — FIX: `get-session-messages?sourcePath=` arbitrary file/SQLite read
- `session_manager/mod.rs::load_messages` (~:93) does `Path::new(source_path)` with NO containment
  check, and the `sqlite:` form opens any SQLite DB. The sibling `delete_session` (~:114) DOES
  validate via `delete_session_with_roots(provider_id, session_id, path, &roots)` (~:127/:141).
- FIX: add a root guard to `load_messages` (and the sqlite path) mirroring `delete_session`'s root
  validation — resolve the provider's legitimate session roots, canonicalize `source_path`, reject if
  it escapes all roots (return an error, do not read). Keep desktop + web behavior identical (shared
  code; the guard is correct for both). The `sqlite:<db>:<id>` form must validate `<db>` is within an
  allowed root too.

### R2 — FIX: debug-logging persists prompts/responses by default
- Shipped `deploy/systemd/cc-switch-web.service` sets `RUST_LOG=info,cc_switch=debug`; full
  request/response bodies log at `log::debug!` (`forwarder.rs`, `response_processor.rs`), so prompts +
  model outputs land in the systemd journal in plaintext.
- FIX: change the shipped unit default to `RUST_LOG=info` (or `info,cc_switch=info`) so bodies are NOT
  logged by default; add a comment that operators can set `cc_switch=debug` temporarily for
  troubleshooting. VERIFY no prompt/response body is logged at `info!`/`warn!` level (if any is, gate
  it behind debug). Privacy-by-default; opt-in to verbose.

### R3 — ACCEPT (no code change): MCP-upsert write→exec
- `POST /api/mcp/upsert-mcp-server` → `McpService::upsert_server` writes an MCP stdio command into live
  CLI config (executes on next CLI launch). This is the INTENDED web-UI feature for managing MCP
  servers; post-C2 it is operator-only (the single authenticated user configuring their own machine).
  No fix — adding friction/confirmation would break the feature. Documented residual: if the tailnet
  ever becomes multi-user, reconsider a confirmation gate or command allowlist.

### R4 — ACCEPT (no code change): OAuth tokens at rest
- `copilot_auth.rs`/`codex_oauth_auth.rs` already write token files `0o600` on Linux (`mode(0o600)` +
  `set_permissions(0o600)`) — standard practice (≈ `~/.ssh`, `~/.aws/credentials`). Encryption-at-rest
  needs key management (keyring/passphrase) — out of scope for a single-user Linux box where 0o600
  already gates other local users. The prior memo's "no Windows ACL" is a desktop-Windows concern,
  N/A to this Linux deployment.

## Acceptance
- R1: a `load_messages` call with a `source_path` outside the provider's session roots is rejected
  (test); legitimate in-root sessions still load; `sqlite:` out-of-root db rejected.
- R2: shipped systemd unit no longer enables body-logging by default; no prompt/response body logs at
  info level.
- R3/R4: documented as accepted in this prd + (R3) a note in the spec's auth scenario if warranted.
- Gates: desktop clippy -D + lib tests; web check + web tests; typecheck; check:web-routes missing:0;
  fmt; diff --check. No regression to Phase 1-4.
