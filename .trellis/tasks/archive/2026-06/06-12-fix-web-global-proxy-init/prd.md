# Fix: web runtime missing global outbound proxy init at startup

## Goal

The web server never initializes the global outbound proxy HTTP client from the saved DB setting,
so after every service (re)start the forwarder makes DIRECT outbound connections — relay providers
that require the user's outbound proxy (e.g. local Clash) are unreachable and every failover
candidate fails. Desktop initializes it in `lib.rs:895-922`; `examples/server.rs` must mirror that.

## Diagnosis (confirmed 2026-06-12)

- Reproduction: user enables codex auto-failover (takeover via local proxy) → all calls fail;
  takeover off → CLI direct call (with its own proxy config) succeeds.
- `journalctl` shows `[GlobalProxy] [GP-004] Client not initialized, using fallback` in web runtime.
- `forwarder.rs:1697` `http_client::get_current_proxy_url()` → None (never initialized) → direct
  outbound; `:1719` `http_client::get()` → GP-004 fallback client.
- Desktop init: `lib.rs:895-922` — `db.get_global_proxy_url()` → `http_client::init(url)`; on error
  GP-005/006/007: clear invalid config from DB, re-init direct (GP-008).
- Web `set_global_proxy_url` handler (web_api/handlers/global_proxy.rs:85) already validates +
  persists + `apply_proxy` (hot-apply works) — the ONLY gap is startup init.

## Requirements

- `examples/server.rs` startup: initialize the global outbound proxy client from
  `db.get_global_proxy_url()`, mirroring desktop semantics exactly (invalid-config clearing +
  direct fallback, same GP-00x log codes). Placement: after DB/AppState, BEFORE takeover restore
  (a restored proxy must forward correctly immediately) — i.e. extend the established lifecycle
  order: ctx → **global-proxy init** → crash recovery → snippets → restore → serve → cleanup.
- Extend the existing `main_pins_proxy_lifecycle_ordering` pin test (or add a sibling) to cover the
  init call and its position.

## Acceptance Criteria

- [ ] Web example test pins global-proxy init in main() lifecycle order.
- [ ] Full gates green (desktop untouched expected; web example suite, smoke, integration).
- [ ] Live verification post-deploy: with a saved global proxy URL, service restart →
      `get_current_proxy_url()` reflects it (no GP-004 on forward path) and failover через relay works.

## Out of Scope

- `update_global_proxy_config` db-direct residual (documented in spec; unrelated to outbound client).
- Proxy auth for the web API (separate known issue).

## Technical Notes

- Spec scenario "Web Server Proxy Module Wiring" lifecycle contract gets a one-line amendment.
- Mirror code, do not extract shared helper from desktop-gated lib.rs (same precedent as S3 helpers).
