# Stop cargo treating web_proxy/web_services as standalone examples (autoexamples=false)

## Goal
`src-tauri/examples/web_proxy.rs` and `web_services.rs` are NOT standalone examples — they are module-definition files (`#[path] pub mod ...` lists, **no `fn main`**) that `examples/server.rs` pulls in via `#[path]` (`server.rs:124 mod proxy;` and `:165 mod services;`). But cargo auto-discovers every `examples/*.rs` as an example target, so `cargo build --example web_proxy` (and `cargo test` before Round-1's band-aid) tries to compile them STANDALONE → fails (E0601 no `main` + E0433 unwired modules). Round-1 masked this by adding `[[example]] required-features=["web-server"]` so `cargo test` skips them — but `cargo build/check --features web-server --example web_proxy` still fails. Fix it properly: stop cargo treating them as examples at all.

## Change (Cargo.toml ONLY)
- `[package]`: add `autoexamples = false`.
- Keep ONLY the real example, explicitly: `[[example]] name = "server"`, `path = "examples/server.rs"`, `required-features = ["web-server"]`.
- REMOVE the `[[example]]` blocks for `web_proxy` and `web_services` (~lines 59–65). They remain plain files included by `server.rs` via `#[path]`.

## Change 2 (REGRESSION FIX — `examples/web_proxy.rs`)
Verifying the above surfaced a real regression already in `main`: `cargo check --no-default-features --features web-server --example server` **FAILS** with `E0433: cannot find model_mapper in proxy` (`usage_stats.rs:1661`). Round-2 Fix A added `crate::proxy::model_mapper::strip_one_m_suffix_for_upstream` to `usage_stats.rs`, but the `server` example's `mod proxy` (= `web_proxy.rs`) never declared `model_mapper`. The web-server binary has not compiled since PR #11 — missed because CI `cargo test` skips the server example (required-features) and the final-validation `--example server` gate gave a FALSE PASS. Fix: add to `examples/web_proxy.rs`
```rust
#[path = "../src/proxy/model_mapper.rs"]
pub mod model_mapper;
```
(`model_mapper` only depends on `crate::provider::Provider`, already wired in server.rs → no cascade; verified `--example server` then compiles cleanly.)

## Acceptance Criteria
- [ ] `cargo build --example web_proxy` (and web_services) → "no example target named ..." (they are no longer example targets).
- [ ] `cargo check --no-default-features --features web-server --example server` passes (server still assembles web_proxy.rs + web_services.rs via `#[path]`).
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` (desktop) passes — server skipped via required-features; NO standalone compile of web_proxy/web_services.
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` (no --all-targets) green; `cargo fmt --check` green.

## Out of Scope
- Rewriting web_proxy.rs/web_services.rs internals; deleting them (server.rs needs them); deferred security items.

## Technical Notes
- `autoexamples = false` disables ONLY example auto-discovery; `autobins`/`autotests`/`autobenches` unchanged (the `cc-switch` / `gen-command-manifest` bins + `src/bin` discovery unaffected).
- With autoexamples=false, set the server example's `path` explicitly to be safe.
- Branch `fix/cargo-autoexamples-web-server-includes` off main; separate small PR. CI Frontend Checks unaffected (Rust-only change → Backend Checks).
