# Fix Frontend Checks CI: split web-server integration suites out of test:unit

## Goal
Make the CI **Frontend Checks** job green. It currently fails — and has failed on `main` + every branch (PRE-EXISTING, unrelated to the just-merged PR #11) — because `pnpm test:unit` (`vitest run`) includes the 20 `tests/integration/*.web-server.test.tsx` E2E suites, which boot a REAL web server. The Node-only Frontend Checks job has neither a built `dist-web` nor a Rust toolchain, so every such suite's `beforeAll` throws and the 20 suites fail.

## Diagnosis (verified)
- `.github/workflows/ci.yml` `frontend` job: Node-only (node 20, pnpm), runs typecheck / format:check / `test:unit`; NO `dist-web` build, NO Rust.
- `tests/helpers/web-server.ts::startTestWebServer`: `ensureDistWeb()` (`fs.access(dist-web/index.html)` → throws if missing) then `spawn("cargo", [… "--example","server"])` (needs Rust).
- 20 suites, uniform `beforeAll(async () => { webServer = await startTestWebServer(); })`; NO skip guard → beforeAll throws → suite FAILS with `dist-web/index.html` ENOENT.
- The `backend` job (Rust) is green and unaffected.

## Approach (core — both options)
These are INTEGRATION tests (need a real server) mis-grouped into the unit run. Separate them:
- Exclude `tests/integration/**/*.web-server.test.tsx` from the default `vitest run` so `test:unit` is pure unit/component/hook (jsdom, no server) → Frontend Checks green.
- Add `pnpm test:integration` (a vitest config including ONLY the web-server suites) to run them where a real server can boot (local dev / a Rust-equipped CI job).
- Preserve ALL non-server unit/component/hook/integration coverage in `test:unit`.

## Decision: A (chosen 2026-06-04)
Web-server suites become **local-only** via `pnpm test:integration` (run where cargo + dist-web exist). NO CI integration job. Frontend Checks goes green by `test:unit` excluding them. (Option C — a Rust-equipped CI integration job — explicitly NOT done; can be added later if CI E2E coverage is wanted.)

## Acceptance Criteria
- [ ] `pnpm test:unit` green WITHOUT server/dist-web/Rust (excludes web-server suites), preserving all unit/component/hook coverage.
- [ ] `pnpm test:integration` runs the 20 web-server suites (verified locally where cargo + dist-web available).
- [ ] CI `Frontend Checks` passes.
- [ ] (If C) the integration CI job builds dist-web + Rust and runs the E2E suites green.
- [ ] `pnpm typecheck` / `format:check` / `build:web` unaffected.

## Out of Scope
- Rewriting the web-server harness or the suites' assertions.
- Deferred security items (C1 auth, SSRF hardening).

## Technical Notes
- Likely: `vitest.config.ts` add `test.exclude` (`**/*.web-server.test.tsx` + vitest defaults); new `vitest.integration.config.ts` including only `tests/integration/**/*.web-server.test.tsx`; `package.json` add `test:integration` (+ maybe `test:integration:watch`). (If C) add a job to `.github/workflows/ci.yml`.
- Branch off `main`; this is a separate small PR.
