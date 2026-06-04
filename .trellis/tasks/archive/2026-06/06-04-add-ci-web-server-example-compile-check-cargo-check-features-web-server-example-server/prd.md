# Add CI web-server example compile check

## Goal
Close the CI coverage gap that let the PR #14 regression hide: the `backend` job runs `cargo test` / `cargo clippy` under DEFAULT (desktop) features, which **skips the `server` example** (it carries `required-features = ["web-server"]`). So the web-server feature tree (the deployable web binary) is **never compiled in CI**. Add a step that compiles it, so a future "added a `crate::proxy::X` dep to a shared service but didn't wire it into `examples/web_proxy.rs`" regression fails CI directly.

## Change (`.github/workflows/ci.yml` ONLY)
Add a step to the existing `backend` job, after `Run tests` (the job already has Rust + Linux system deps + `mkdir -p dist`):
```yaml
      - name: Web-server example compile check
        run: cargo check --no-default-features --features web-server --example server --manifest-path src-tauri/Cargo.toml
```
- Use `cargo check` (NOT `clippy -- -D warnings`): the web-server example assembly emits ~185 PRE-EXISTING dead-code warnings (the `#[path]`-included module trees have unused items); `-D warnings` would fail on those. `cargo check` catches compile breaks (E0433 etc.) without failing on benign warnings.
- Reuse the existing `backend` job (Rust toolchain, cargo cache, system deps, dist placeholder already set up) — do NOT add a new job.

## Acceptance Criteria
- [ ] `.github/workflows/ci.yml` `backend` job has the new "Web-server example compile check" step after "Run tests"; existing steps unchanged; YAML valid.
- [ ] The step's command passes locally: `cargo check --no-default-features --features web-server --example server --manifest-path src-tauri/Cargo.toml` → exit 0 (already true on `main` after PR #14).
- [ ] On the PR, CI Backend Checks (incl. the new step) is green; Frontend Checks green.

## Out of Scope
- Adding clippy/`-D warnings` on the web-server feature (would fail on pre-existing example dead-code warnings).
- A separate CI job; building web_proxy/web_services (no longer example targets).
- The deferred security items; the 4 niche pricing ids.

## Technical Notes
- This is the web half of memory trap #1/#1b (dual-runtime compile-coverage). The desktop half is already covered by the backend job's `cargo test`/`clippy`.
- Branch `ci/web-server-example-compile-check` off main; small PR.
