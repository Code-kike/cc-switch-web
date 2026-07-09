# Remove Web Basic Auth for unauthenticated Web API

## Goal

Remove cc-switch-web's application-layer Web Basic Auth while keeping the
standalone Web server bound to `0.0.0.0` and preserving the browser same-origin
intent check for mutating requests.

## What I Already Know

- The user explicitly wants Web Basic Auth removed and does not want security
  authentication for the Web deployment.
- Through the `grill-with-docs` session, the chosen posture is:
  - no application-layer authentication;
  - bind the Web server to `0.0.0.0`;
  - keep the full Web API capability surface open;
  - retain same-origin intent checks for browser-initiated mutating requests;
  - remove/disable old systemd Basic Auth drop-ins during install;
  - delete Basic Auth code paths rather than leaving an optional dormant mode;
  - default future non-blocking design choices to the recommended option.
- Existing domain docs were created/updated before this task:
  - `CONTEXT.md` defines `Unauthenticated Web API` and `Same-origin intent check`.
  - `docs/adr/0001-unauthenticated-web-api.md` records the proposed boundary.
- Current code couples Basic Auth and same-origin intent in
  `src-tauri/src/web_api/middleware/auth.rs`.
- `src-tauri/examples/server.rs` currently refuses non-loopback binds unless
  `CC_SWITCH_WEB_AUTH_PASSWORD` is configured.
- `scripts/install-cc-switch-web-service.sh` currently generates/preserves
  `~/.config/systemd/user/cc-switch-web.service.d/auth.conf`.
- `deploy/systemd/cc-switch-web.service`, `README.md`, `README_ZH.md`, and
  `.trellis/spec/frontend/quality-guidelines.md` all document the old Basic Auth
  contract.

## Requirements

- Remove Web Basic Auth credential parsing, challenge responses, and auth
  configuration semantics.
- Remove `CC_SWITCH_WEB_AUTH_PASSWORD` and `CC_SWITCH_WEB_AUTH_USER` from the
  product-facing Web server contract.
- Allow non-loopback binds such as `HOST=0.0.0.0` without requiring any
  application-layer credential.
- Keep same-origin intent checks for state-changing `/api/*` browser requests:
  - `Sec-Fetch-Site` of `same-origin` or `none` passes;
  - cross-site fetch metadata rejects;
  - matching `Origin` and `Host` passes;
  - mismatched or opaque `Origin` rejects;
  - no `Origin` and no fetch metadata passes for direct clients such as curl.
- Keep CORS preflight behavior intact: `OPTIONS` with `Origin` reaches the
  inner CORS layer instead of being rejected by the intent guard.
- Keep `/api/health` and static SPA assets public.
- Keep the full Web API capability surface open to unauthenticated network
  clients; do not introduce read-only mode or selective high-risk route blocking.
- Update installer behavior so an existing Basic Auth systemd drop-in is removed
  or disabled during install.
- Update systemd unit, README files, code comments, and Trellis code-spec docs to
  reflect the new unauthenticated Web API posture.
- Preserve current privacy-by-default logging (`RUST_LOG=info`) and existing data
  directory behavior.

## Acceptance Criteria

- [x] `HOST=0.0.0.0` Web server startup no longer requires
  `CC_SWITCH_WEB_AUTH_PASSWORD`.
- [x] `/api/*` routes no longer return Basic Auth `401` challenges based on
  missing credentials.
- [x] State-changing cross-site browser requests still return `403`.
- [x] Same-origin/browser-none/direct-client mutating requests still pass the
  same-origin intent guard.
- [x] Installer removes or disables
  `~/.config/systemd/user/cc-switch-web.service.d/auth.conf`.
- [x] Docs no longer instruct users to configure Web Basic Auth.
- [x] Code-spec docs describe the new unauthenticated Web API + same-origin
  intent check contract.
- [x] Web server cargo check and relevant Rust middleware tests pass.
- [x] Frontend route coverage/type checks still pass where relevant.

## Definition of Done

- Tests updated for the new middleware behavior.
- Rust formatting/checks pass.
- Web route/type checks pass when relevant.
- Docs/spec/domain/ADR are consistent.
- Current persistent service can be rebuilt/reinstalled after the code change if
  requested separately.
- Work is committed in a coherent Trellis work commit before task wrap-up.

## Out of Scope

- Adding a replacement login system, session cookies, OAuth, tokens, or API keys.
- Adding read-only mode or selectively disabling high-risk routes.
- Changing service port, data directory, proxy behavior, or static asset serving.
- Changing CORS allow-list semantics beyond preserving existing preflight flow.
- Changing firewall, Tailscale, reverse proxy, or OS-level access controls.

## Technical Approach

- Refactor `web_api/middleware/auth.rs` into a same-origin intent middleware:
  remove Basic Auth credential state and challenge logic, keep/rename the
  state-changing method classifier and `check_same_origin_intent`.
- Update `web_api/routes.rs` to layer the renamed intent guard instead of an auth
  middleware, with comments that describe unauthenticated API behavior.
- Remove non-loopback auth gating and auth log messages from
  `src-tauri/examples/server.rs`.
- Update `scripts/install-cc-switch-web-service.sh` to delete or disable the old
  `auth.conf` drop-in before daemon reload/restart.
- Remove auth environment variables and comments from
  `deploy/systemd/cc-switch-web.service`.
- Update README English/Chinese deployment docs.
- Update `.trellis/spec/frontend/quality-guidelines.md` scenarios that currently
  describe Basic Auth as the Web API boundary.
- Keep `CONTEXT.md` and `docs/adr/0001-unauthenticated-web-api.md` as task docs,
  updating them if implementation reveals a better term.

## Decision (ADR-lite)

**Context**: Current Web security hardening requires Basic Auth for non-loopback
binds. The user wants no application-layer authentication and explicitly accepts
binding the service to `0.0.0.0` with full API capabilities exposed to reachable
network clients.

**Decision**: Remove Web Basic Auth completely, keep the Web server
unauthenticated on `0.0.0.0`, and retain same-origin intent checks only as a
browser-request guard rather than an identity/authentication mechanism.

**Consequences**: Any host that can reach the listening port becomes an operator
of the cc-switch-web instance. The implementation must avoid misleading
half-auth states by removing old auth environment variables and installer
drop-ins.

## Technical Notes

- Relevant code:
  - `src-tauri/src/web_api/middleware/auth.rs`
  - `src-tauri/src/web_api/routes.rs`
  - `src-tauri/examples/server.rs`
  - `deploy/systemd/cc-switch-web.service`
  - `scripts/install-cc-switch-web-service.sh`
- Relevant docs/spec:
  - `README.md`
  - `README_ZH.md`
  - `.trellis/spec/frontend/quality-guidelines.md`
  - `CONTEXT.md`
  - `docs/adr/0001-unauthenticated-web-api.md`

## Implementation Update

- Replaced the old Web Basic Auth middleware with
  `web_api/middleware/intent.rs`, which keeps only the same-origin intent guard
  for state-changing `/api/*` browser requests.
- Removed the non-loopback auth gate from the standalone web server so
  `HOST=0.0.0.0` starts without `CC_SWITCH_WEB_AUTH_PASSWORD`.
- Updated installer/systemd/docs/spec/ADR language to describe the
  unauthenticated Web API posture and remove product-facing Basic Auth
  configuration.
- Installer now removes the legacy user-service `auth.conf` drop-in before
  restarting the service.

## Verification Log

- `cargo fmt --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo check --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server web_api::middleware::intent::tests`
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`
- `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- `pnpm typecheck`
- `pnpm format:check`
- `pnpm check:web-routes`
- `git diff --check`
