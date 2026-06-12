# Research: claude-code-hub account selection + auto-failover

- **Query**: How does claude-code-hub's account selection + auto-failover work, and how does it map onto cc-switch-web's proxy machinery? Goal: replicate "random account; on failure, randomly pick another available account".
- **Scope**: mixed (local installs + local source checkouts + fork code)
- **Date**: 2026-06-12

## 1. Tools found on this machine

| Tool | Where | Version | Running? | Relevance |
|---|---|---|---|---|
| **claude-code-hub** (ding113/claude-code-hub) | Docker `claude-code-hub-app-1` (`ghcr.io/ding113/claude-code-hub:latest`, image built 2026-06-06) on **port 23000**, + `postgres:18` + `redis:7-alpine`. Full source checkout at `/home/orion/Workspace/github/claude-code-hub` | repo VERSION **0.8.4** (package.json 0.8.0) | Yes, healthy, up with host | **This is the daily driver** the maintainer described. Port 23000 matches the gateway agents use. |
| CLIProxyAPI (router-for-me / CLIProxyAPI, Go) | `/home/orion/cliproxyapi/` (`cli-proxy-api` binary, `config.yaml`), systemd `cliproxyapi.service`, port 8317 | 6.10.9 | Yes (systemd, active) | Different tool, also running. Its routing is **round-robin** (`routing.strategy: "round-robin"`) with `session-affinity: true` (TTL 1h), retry on 403/408/500/502/503/504, `request-retry: 1`, `max-retry-credentials: 0` (= try all). It does NOT match the "random" behavior described. |

Conclusion: the maintainer did not conflate names — both exist, but the behavior to replicate ("随机调用某一个账号，失败后随机换一个可用账号") is **claude-code-hub's**, and CCH is the container actually serving Claude Code / Codex traffic on :23000.

## 2. claude-code-hub algorithm (source-verified, repo @ v0.8.4)

Key files (all under `/home/orion/Workspace/github/claude-code-hub/`):

| File | Role |
|---|---|
| `src/app/v1/_lib/proxy/provider-selector.ts` | Selection: session reuse → filters → priority tier → weighted random (`ProxyProviderResolver`) |
| `src/app/v1/_lib/proxy/forwarder.ts` | Failover loops, error classification handling, request replay (5036 lines) |
| `src/app/v1/_lib/proxy/errors.ts:551` | `ErrorCategory` enum (failure classification) |
| `src/lib/circuit-breaker.ts` | Per-provider breaker (consecutive failures, open/half-open) |
| `src/lib/redis/circuit-breaker-config.ts` | Per-provider breaker config, defaults |
| `src/lib/session-manager.ts:748` | `updateSessionBindingSmart` (sticky sessions, Redis) |
| `src/lib/vendor-type-circuit-breaker.ts` | Temp breaker per (vendor, providerType) on all-endpoints-timeout |

### 2.1 Selection (per request)

`ProxyProviderResolver.ensure()` → `pickRandomProvider()` (provider-selector.ts:716):

1. **Session reuse first** (sticky): if request has >1 messages (`session.shouldReuseProvider()`, session.ts:577) and Redis holds `session:{id}:provider`, reuse that provider — after re-validating: enabled, schedule window, circuit not open, vendor-type circuit not open, format compatibility, allowedModels, client allow/block lists, group match, cost limits. Any check fails → binding cleared/skipped → fresh pick.
2. **Filter pipeline**: group pre-filter → client restrictions → `isEnabled` + not in `excludeIds` → schedule window → format↔providerType compatibility (claude→claude|claude-auth, response→codex, …) → allowedModels → health (`filterByLimits`: vendor-type circuit, per-provider circuit (`isCircuitOpen`), 5h/daily/weekly/monthly/total cost limits).
3. **Priority tiers**: keep only providers at the minimum `priority` value (group overrides supported).
4. **Weighted random within tier** (`selectOptimal` → `weightedRandom`, lines 1155–1197): sort by costMultiplier, then `Math.random() * totalWeight` cumulative scan. With all weights equal ⇒ **uniform random**. `totalWeight === 0` ⇒ plain `Math.floor(Math.random() * n)`.
5. **Atomic concurrency check** (`limitConcurrentSessions`, 0 = unlimited); on failure: exclude provider, re-pick (loop inside `ensure()`).
6. Nothing available ⇒ **503 "No available providers"** (this is exactly the 503 seen locally under heavy agent fan-out).

### 2.2 Failure classification (errors.ts:551, decides failover vs pass-through)

| Category | What | Circuit breaker? | Action |
|---|---|---|---|
| `PROVIDER_ERROR` | All upstream 4xx/5xx (except below) + empty-response | **Yes** (after per-provider retries exhausted) | retry same provider (100 ms delay) up to maxRetryAttempts, then **switch provider** |
| `SYSTEM_ERROR` | transport errors (ECONNREFUSED, ECONNRESET, ETIMEDOUT, UND_ERR_*, DNS) | No (unless `ENABLE_CIRCUIT_BREAKER_ON_NETWORK_ERRORS=true`; default false) | retry once on next endpoint, then switch provider |
| `RESOURCE_NOT_FOUND` | upstream 404 | No | retry, then switch provider |
| `NON_RETRYABLE_CLIENT_ERROR` | DB-configurable error rules (prompt too long, content filter, PDF limit, thinking format, bad params) | No | **throw immediately** — no retry, no switch |
| `CLIENT_ABORT` | AbortError / local 499 | No | stop immediately, **clears session binding** |

### 2.3 Failover loop (forwarder.ts `send()`, lines ~1048–2185)

- **Outer loop**: provider switches, hard cap `MAX_PROVIDER_SWITCHES = 20` (forwarder.ts:172). Alternative = `selectAlternative()` → `pickRandomProviderWithExclusion(session, failedProviderIds)` — i.e. **the next provider is again weighted-random over the remaining healthy set** (not a precomputed queue).
- **Inner loop**: per provider, `maxRetryAttempts` attempts (per-provider DB column; default env `MAX_RETRY_ATTEMPTS_DEFAULT`, fallback `PROVIDER_DEFAULTS.MAX_RETRY_ATTEMPTS = 2`), 100 ms sleep between attempts. Endpoint candidates (vendor endpoint pool, latency-sorted) advance only on SYSTEM_ERROR/524 ("endpoint stickiness").
- **Replay semantics**: full request body is held in `session.request.message`; each attempt re-serializes and re-sends it. Failover only happens for errors raised **before the response is handed to the client** (non-2xx, transport error, empty body, first-byte timeout). Once a 200 stream starts flushing to the client, a mid-stream drop is NOT replayed.
- **Streaming hedge** (`sendStreamingWithHedge`): only when `provider.firstByteTimeoutStreamingMs > 0` — races a second random provider if the first byte is slow. **User has 0 on all 36 providers ⇒ disabled locally.**
- **Success path** (forwarder.ts:~1492): `recordSuccess(provider)` + smart session binding: first success binds with Redis `SET NX`; success after a failover **unconditionally rebinds** to the new provider (session-manager.ts:748).

### 2.4 Circuit breaker (cooldown/recovery)

- Per provider; config columns `circuit_breaker_failure_threshold` / `_open_duration` / `_half_open_success_threshold`. Code defaults: **5 failures / 30 min open / 2 half-open successes** (circuit-breaker-config.ts:23).
- Counts **consecutive** failures (any success in closed state resets count to 0).
- Open → after `openDuration` elapses, first `isCircuitOpen()` check flips to **half-open** (allows traffic; no probe-permit limiting — unlike the fork). 2 successes close it; any failure while count ≥ threshold re-opens with a fresh window.
- State persisted to Redis (multi-instance + restart recovery).
- Separate **vendor-type breaker** trips when all endpoints of a (vendor, type) hit 524 timeouts.
- Recovery is **time-based + live-traffic probing**; the endpoint probe-scheduler (TCP, 30 s) exists but `ENABLE_ENDPOINT_CIRCUIT_BREAKER=false` in the local deployment.

### 2.5 Stickiness & concurrency

- Selection is **per-request random**, softened by **per-session pinning**: Redis `session:{sessionId}:provider`, TTL `SESSION_TTL` env, **default 300 s** (session-manager.ts:193), refreshed on success. Session IDs from `metadata.user_id` (Claude Code), `previous_response_id`/headers (Codex), etc.
- So in practice: a conversation stays on one account while it keeps succeeding; first request of a new conversation (or after 5 min idle, or after failure) rolls the dice again.

## 3. User's actual local CCH settings (sanitized; from container env + postgres)

- 36 providers; **24 enabled** (4 claude-type, 18 codex, rest openai-compatible/gemini). Mostly free/community relay accounts (anyrouter ×N, community codex relays, etc.).
- **Every provider: `weight = 1`, `priority = 0`** ⇒ selection is pure **uniform random** over healthy same-format providers. This is exactly the "随机调用" the maintainer experiences.
- Breaker per provider: threshold **5** (claude rows) or **10** (some anyrouter codex rows); `open_duration` **60 000 ms = 60 s** on 35/36 rows (one 30 min). Much shorter cooldown than CCH's 30-min default.
- `max_retry_attempts` NULL on most rows ⇒ env default ⇒ **2 attempts per provider**.
- `limit_concurrent_sessions = 0` (unlimited) everywhere; `first_byte_timeout_streaming_ms = 0` everywhere (hedge off).
- Container env (behavior-relevant): `ENABLE_RATE_LIMIT=true`, `ENABLE_ENDPOINT_CIRCUIT_BREAKER=false`, `ENABLE_SMART_PROBING=false`, probes TCP @30 s, `STORE_SESSION_MESSAGES=false`, fetch timeouts 600 s. `SESSION_TTL` not set ⇒ 300 s sticky TTL. `ENABLE_CIRCUIT_BREAKER_ON_NETWORK_ERRORS` not set ⇒ false.
- (CLIProxyAPI config for contrast: round-robin strategy, session-affinity 1h, retry=1 on 403/408/500/502/503/504, cooling enabled, `max-retry-interval: 30`.)

## 4. Mapping onto cc-switch-web (fork) modules

All paths under `/home/orion/Workspace/github/cc-switch-web/src-tauri/src/proxy/`.

| CCH behavior | Fork module | Status in fork |
|---|---|---|
| Candidate set = enabled ∩ circuit-available | `provider_router.rs::select_providers()` (lines 37–109) | EXISTS — failover ON: failover queue order (`get_failover_queue`, sort_index), filtered by `breaker.is_available()` (read-only check; Open allowed once cooldown elapsed). Failover OFF: current provider only, breaker bypassed. |
| **Random selection / random next-on-failure** | — | **MISSING** — order is strictly the queue order; no RNG anywhere in the router. |
| Weighted random / priority tiers | — | MISSING (no weight/priority columns on fork providers; `sort_index` is the only ordering). |
| Per-provider breaker: consecutive failures → open → cooldown → half-open probe → close | `circuit_breaker.rs` | EXISTS and **stronger than CCH**: consecutive-failure threshold (default 4) + sliding-window error-rate trip (0.6 over last 10), 60 s cooldown, half-open with **limited probe permits** (`AllowResult.used_half_open_permit`, must be released), success_threshold 2. Per `app_type:provider_id`, config per app from `proxy_config`. |
| Failure classification (provider vs client errors) | `forwarder.rs::categorize_proxy_error()` (line 1968) | EXISTS — `Retryable` (timeouts, transport, most 4xx/5xx) vs `NonRetryable` (400/405/406/413/414/415/422/501, internal) vs `ClientAbort`. Equivalent to CCH's PROVIDER_ERROR vs NON_RETRYABLE_CLIENT_ERROR split (fork lacks the DB-configurable error-rule engine; fine). |
| Retry-the-request on next account (replay) | `forwarder.rs::forward_with_retry_inner()` (line 352) | EXISTS — iterates pre-selected provider list, `body.clone()` per provider, full replay; cap = `max_retries + 1` attempts (`AppProxyConfig.max_retries`; 0 when failover off — `handler_context.rs:212`). Each provider tried once (plus rectifier retries). |
| Don't count success until stream actually produces data | `forwarder.rs::prepare_success_response_for_failover()` (line 1796) | EXISTS — non-streaming: reads full body under timeout; streaming: waits for first chunk and re-chains it. Mid-stream-after-first-byte failures not failed over (same as CCH). |
| Switch "current provider" after failover success | `failover_switch.rs::FailoverSwitchManager::try_switch()` | EXISTS (dedup + hot switch + tray/event). Orthogonal to selection strategy. |
| Sticky per-session binding (300 s) | `session.rs` (extracts session IDs: Claude `metadata.user_id`, Codex `previous_response_id`) | PARTIAL — session-ID extraction exists, but there is **no session→provider binding store**. |
| Vendor-type breaker, cost limits, schedule windows, groups | — | Not present; not needed for the requested behavior. |

### What "random" concretely needs that the fork lacks

1. **Selection RNG**: `rand = "0.8"` is already a dependency but **only under the `web-server` feature** (Cargo.toml:39,132). Any use in `provider_router.rs` (compiled in BOTH desktop and web runtimes — see memory re dual-runtime compile coverage) requires either promoting `rand` to non-optional, or a tiny self-contained PRNG (e.g. seed from `SystemTime`/`std::collections::hash_map::RandomState`) to avoid feature-gate breakage of desktop clippy/test CI gates.
2. **Availability set definition**: already exists — failover queue ∩ `breaker.is_available()`. Random strategy only changes the ORDER of that list, nothing else.
3. **Cooldown source**: reuse the existing `CircuitBreaker` unchanged — it already implements CCH's open/half-open/cooldown semantics (60 s default matches the user's CCH tuning) plus permit-limited probing that CCH doesn't even have.
4. **Pinning (optional)**: CCH pins per conversation for 300 s. The fork is currently per-request. Without pinning, uniform random per request still satisfies the maintainer's stated requirement; pinning is only needed if account-side prompt caching matters.

## 5. Design options

### Option A — `FailoverStrategy::Random`: shuffle the candidate list in `select_providers` (recommended)

- Add a strategy field (e.g. `failover_strategy: "ordered" | "random"`, default `ordered`) to `proxy_config` per app; in `select_providers()`, when `random` and failover ON, build the same availability-filtered list, then Fisher-Yates shuffle it before returning.
- Forwarder/breaker/failover_switch need **zero changes**: `forward_with_retry_inner` already walks the list in order, replays the body, records failures into the breaker, and stops on NonRetryable — a shuffled list IS "random pick, then random next available on failure", exactly CCH-with-equal-weights.
- Trade-offs: + minimal surface (one DAO field, ~15 lines in router, FE toggle); + preserves `max_retries` cap and half-open permit accounting untouched; − no health-weighting (a flaky-but-not-yet-open provider is as likely as a healthy one); − RNG dependency caveat (point 4.1).

### Option B — Weighted/health-aware random

- Same shuffle point, but weight each provider by recent health from the breaker's sliding window (`get_stats()`: total/failed in last 10) or DB `provider_health`; pick via cumulative-weight roll per slot (CCH's `weightedRandom`, generalized to ordering).
- Trade-offs: + degrades gracefully for semi-broken providers below the breaker threshold; + closest to CCH's full algorithm if per-provider `weight`/`priority` columns are added later; − more state plumbed across modules, needs tie-breaking/zero-weight rules, harder to test deterministically; − the user's actual CCH config doesn't use weights at all, so this is speculative value.

### Option C — Random + per-session pinning (full CCH affinity emulation)

- Option A + an in-memory `HashMap<String /*session_id*/, (provider_id, Instant)>` (TTL ~300 s) keyed by `session.rs`'s extracted session ID; on success bind/rebind (CCH's `updateSessionBindingSmart` semantics: bind on first success, force-rebind after failover success, clear on client abort); reuse only if the pinned provider passes `breaker.is_available()`.
- Trade-offs: + keeps a conversation on one account (prompt-cache hits, fewer context re-uploads on relays that cache); + matches the maintainer's daily-experienced behavior most precisely; − new shared state + eviction logic, web runtime is multi-worker-safe today only because state sits in `ProviderRouter`-style `Arc`s — pinning map must live there too; − more tests (TTL expiry, rebind-after-failover, abort-clears-binding).

**Recommendation**: Option A now (it is literally the described requirement; the user's CCH config has all weights=1 and priority=0, so uniform random is behavior-identical), with Option C as a follow-up if account-side caching turns out to matter.

## Caveats / Not Found

- CCH provider rows contain account emails in names; intentionally aggregated here, and the postgres DSN credential seen in container env was excluded. Do not copy raw `docker inspect` output into specs.
- `halfOpenSuccessThreshold` per-row values weren't queried (defaults to 2); irrelevant to the port.
- CCH's DB-driven error-rule engine (NON_RETRYABLE overrides) was not ported-relevance-checked rule-by-rule; the fork's static status-code bucket list covers the same intent.
- Mid-stream (post-first-chunk) failover: neither tool replays; no need to invent it.
- `rand` feature-gating caveat (Section 4.1) is load-bearing for the desktop CI gates — verify with `cargo check` for BOTH runtimes per the gate-suite memory.
