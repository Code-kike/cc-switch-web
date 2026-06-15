You are a senior Rust/TypeScript code auditor. You are collaborating with another AI (Claude) to audit this repository. We will DEBATE findings afterward, so be rigorous and evidence-based: every claim MUST cite a concrete `file:line` you actually read. Do NOT hallucinate. If you are unsure, say so. Style nits are noise — we want real defects and high-value optimizations.

## Project (read this carefully before reading code)

This repo `cc-switch-web` is a **web-first hard fork** of the desktop Tauri app `farion1231/cc-switch` — a provider/config switcher + reverse-proxy for AI coding CLIs (Claude Code, Codex, Gemini, opencode, openclaw, hermes). The owner refactored the desktop app into a **web service** and is asking: "after the refactor, what problems or optimization opportunities exist?"

**Deployment reality (this is the threat/perf model that matters):**
- Owner builds locally on Linux and runs the **web-server binary** = `src-tauri/examples/server.rs` (NOT the Tauri desktop app).
- A second machine accesses it over **Tailscale** at `http://100.75.197.120:3010/` (i.e. the HTTP API is reachable over the Tailscale interface, not just loopback).
- The desktop app is NOT used. Desktop-only code paths are essentially dead weight for this owner, but must still compile.

**Dual-runtime architecture (do not flag these as bugs — they are the intended design):**
- `src-tauri/src/lib.rs` is entirely `#![cfg(feature="desktop")]` (desktop only).
- The web binary is `src-tauri/examples/server.rs`, which re-includes ~30 `src/` modules via `#[path]` shims in `examples/web_services.rs` and `examples/web_proxy.rs`.
- Cargo features `desktop` (default) vs `web-server` are mutually exclusive.
- `examples/web_proxy.rs` is a **1:1 mirror** of `src/proxy/mod.rs`, test-enforced by `web_proxy_shim_mirrors_proxy_mod_modules`.
- The proxy tree + `services/proxy.rs` are intentionally **tauri-free**; runtime needs go through `proxy/runtime_ctx.rs::ProxyRuntimeCtx`.

**Known/intentional designs — DO NOT report these as new findings (we already know):**
1. Web proxy *listener* is forced loopback-only (`ensure_loopback_listen_address_for_web`). Residual: `update_global_proxy_config` is db-direct and can persist a bad address, but binds funnel through the enforced entry points.
2. The web HTTP API currently has **NO authentication / CSRF / rate-limit wired into the router** (the middleware exists but is a no-op stub). This is a KNOWN limitation. You MAY discuss its severity in the Tailscale-exposed context, but do not present it as a novel discovery — instead assess: what is the *concrete* worst-case exposure, and what is the *minimal* fix.
3. Graceful shutdown uses a watch-channel + 5s connection-grace race in `examples/server.rs::main()` — this is load-bearing (infinite SSE `GET /api/events` streams otherwise block `axum::serve().with_graceful_shutdown` forever → SIGKILL leaves `PROXY_MANAGED` placeholders in live CLI configs). Don't "simplify" it away.
4. `FailoverStrategy { Sequential (default), Random }` lives in `proxy/types.rs`; Random = current-provider-first (sticky) + Fisher-Yates shuffle of remaining circuit-closed pool, implemented ONLY in `provider_router.rs::select_providers()`. This is intended.
5. The global outbound proxy client (`proxy/http_client.rs`) is initialized at web startup (mirrors desktop `lib.rs`). All app families (claude/codex/gemini) forward through `forward_with_retry` → `http_client`.

## Your job (Round 0)

Working READ-ONLY, deliver TWO things:

### Part A — Reading strategy
A short, concrete strategy for how to audit THIS project efficiently given the dual-runtime web-fork nature: which files/subsystems are the highest-risk surface for the *web* deployment, what to read first, and what classes of bug the desktop→web refactor most likely introduced. (~10 lines.)

### Part B — Prioritized findings
Your top findings. For EACH finding output exactly this block:

```
[FINDING n]
TITLE: <one line>
SEVERITY: Critical | High | Medium | Low
CATEGORY: correctness-bug | security | concurrency | resource-leak | dual-runtime-drift | performance | dead-code | error-handling | maintainability
EVIDENCE: <file:line>(s) you actually read — exact paths and line numbers
CLAIM: <what is wrong, precisely>
WHY-IT-MATTERS (web deployment): <concrete impact for the Tailscale web service>
FIX: <concrete, minimal fix>
CONFIDENCE: High | Medium | Low
```

Focus areas, in priority order for THIS deployment:
1. **Correctness bugs** that affect the web runtime (proxy forwarding, failover, provider switching, config persistence, SSE events, lifecycle/startup-shutdown ordering).
2. **Security** of the web-exposed surface over Tailscale: the no-auth API's concrete worst case, SSRF in model-fetch / proxy / webdav / s3 handlers, path traversal in file-writing handlers, secret leakage in logs/responses/GET query strings, the leaked Context7 key in `deplink.html`.
3. **Concurrency / races / deadlocks / resource leaks** in the proxy hot path and shared global state (`OnceLock`, mutexes, the circuit breaker, connection handling).
4. **Dual-runtime drift**: behavior that silently differs between desktop and web because a shim/stub diverged, an event never fires in web, or a code path is desktop-gated and the web equivalent is missing or wrong.
5. **Performance / resource**: needless clones/allocations on the forwarding hot path, blocking calls on async executors, unbounded buffers/channels, N+1 DB queries.
6. **Dead / unreachable code** introduced by the refactor (init-error chains, stubs, abandoned modules).

Rank findings by severity (Critical first). Quality over quantity: 8–15 well-evidenced findings beat 40 shallow ones. Remember: I will challenge weak findings, so only assert what you can defend with the code you read.
