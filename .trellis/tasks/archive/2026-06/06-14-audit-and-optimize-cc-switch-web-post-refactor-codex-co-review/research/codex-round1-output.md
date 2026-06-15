Point-by-point:

- F1 traversal: concur Critical. Your live proof settles it.

- F2 no-auth: concur Critical. Also agree `ALLOW_HTTP_BASIC_OVER_HTTP` is misleading because `require_auth` is not wired and is permissive anyway.

- F3 WebDAV/S3 SSRF: concur High. The key inconsistency is real: `config.rs:459-461` and `system.rs:202` use `validate_outbound_url`, but `webdav.rs:146` and `s3.rs:137` go straight to sync services.

- F4 ZenMux SSRF: concede Medium. I do not have evidence of arbitrary internal-response reflection. It is blind and requires attacker-supplied `api_key`; still real because `subscription.rs:49-54` passes `base_url` through and `coding_plan.rs:418-427` dials it.

- F5 JSON migration gap: concur Medium for this deployment, High for general upgrade path. Evidence remains `server.rs:251` direct `Database::init()` versus desktop `lib.rs:363-434`.

- F6 bootstrap imports: refine, not “desktop-only.”
  - Provider live import/default/official seeding is a real web gap: providers are web-reachable via `routes.rs:55` and `providers.rs:209-216`; desktop does live import/seeding at `lib.rs:510-545`; web startup path `server.rs:251-318` lacks that.
  - MCP/prompts import is also web-reachable but narrower: routes are merged at `routes.rs:48` and `routes.rs:54`; desktop imports empty tables at `lib.rs:641-709`; web omits it. This is first-run/import parity, not total feature breakage.
  - Skill repo seeding and `skills_ssot_migration_pending` are real if web exposes skills: `routes.rs:60`; desktop does `init_default_skill_repos` and consumes pending at `lib.rs:446-490`; schema sets pending at `schema.rs:933-943`; web omits it.
  Consensus severity: Medium if grouped as “web startup parity gaps”; not High.

- F7 single-provider breaker bypass: concur Medium. Narrow but valid.

- F8 start race: validity yes; accept Low. There is an easy trigger under F2 via concurrent `POST /proxy/start-proxy-server` (`proxy.rs:71`, handler `proxy.rs:149-157`), but impact is bounded to bind error or leaked port-0 listener.

- F9 failover reachability: defend as reachable. Main UI renders `FailoverToggle` when `enableFailoverToggle` is set (`App.tsx:1308-1309`); the toggle calls `useSetAutoFailoverEnabled` (`FailoverToggle.tsx:22-30`) and renders a `Switch` (`FailoverToggle.tsx:69-72`). Settings also renders `FailoverQueueManager` (`ProxyTabContent.tsx:190-193`), whose switch calls `handleToggleFailover` (`FailoverQueueManager.tsx:91-94`, `183-186`). No `isWebMode` gate in those paths. Keep Medium.

- F10 auto-sync reachability: defend as reachable. Settings renders `WebdavSyncSection` (`SettingsPage.tsx:423-429`); its props have no web-mode flag (`WebdavSyncSection.tsx:189-194`, `235-240`); WebDAV and S3 auto-sync switches render at `WebdavSyncSection.tsx:1083-1096` and `1370-1386`. Because web stubs are no-op (`webdav_auto_sync_web.rs:27`, `s3_auto_sync_web.rs:34`), keep Medium.

- F11 blocking DNS/fan-out: partially defend, refine wording. The blocking resolver is not in `speedtest.rs`; it is in the web SSRF guard: `system.rs:201-203` calls `validate_outbound_url`, and `common.rs:236` uses `to_socket_addrs()` inside the async request path. The no-cap fan-out remains `TestApiEndpointsRequest.urls` (`system.rs:45-47`) and `join_all` (`speedtest.rs:112`). SSRF itself is guarded here. Keep Medium as “blocking guard DNS + unbounded fan-out.”

Missed findings:

- M1 provider-router DB reads: agree as Medium-Low/Low optimization. It is on the request path via `handler_context.rs:127-132`; failover path reads config/providers/queue at `provider_router.rs:47-63`, and result recording re-reads config at `provider_router.rs:182`. I would rank Low for personal scale unless proxy QPS matters.

- L1 600s streamed timeout: refute as stated for the proxy hot path. `http_client.rs:218` sets a 600s client default, but the reqwest streaming path overrides per request to 24h at `forwarder.rs:1721-1724`; the raw hyper path does not use `http_client` and passes its own timeout at `forwarder.rs:1757-1765`. I need a concrete streamed callsite using `http_client` without override before accepting.

- L2 dead model_fetch router: agree Low. `model_fetch.rs:13-14` returns `Router::new()`, yet it is still merged at `routes.rs:49`.

False positives:

- Confirm I did not rely on unguarded `config.rs` model fetch; it is guarded at `config.rs:459-461`.
- Confirm Context7 live key is out of scope for this fork; I only saw the placeholder in `deplink.html:526-532`.

Unresolved: only L1, unless you have a concrete streamed `http_client` callsite. F11 remains a disagreement only on location/wording, not on the blocking-DNS claim itself.