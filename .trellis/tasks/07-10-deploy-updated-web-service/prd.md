# Deploy updated unauthenticated Web service

## Goal

Build the current repository state and replace the existing persistent
`cc-switch-web.service` deployment so the running Web API uses the newly
committed unauthenticated implementation.

## What I Already Know

- The persistent user service is `cc-switch-web.service`.
- It currently runs `/home/orion/.local/bin/cc-switch-web`, listens on
  `0.0.0.0:3010`, and uses `~/.cc-switch` for data.
- The current process started on 2026-07-09 and still loads the legacy
  `~/.config/systemd/user/cc-switch-web.service.d/auth.conf` drop-in.
- `scripts/install-cc-switch-web-service.sh` builds the Web assets and release
  server, installs them, removes the legacy auth drop-in, reloads systemd, and
  explicitly restarts the existing service.
- The source worktree was clean before the Trellis task was created.

## Requirements

- Deploy the current checked-out repository state using the supported install
  script.
- Preserve `cc-switch-web.service`, `HOST=0.0.0.0`, `PORT=3010`,
  `CC_SWITCH_DATA_DIR=%h/.cc-switch`, and `RUST_LOG=info`.
- Replace the installed release binary and `dist-web` assets.
- Remove the legacy Basic Auth systemd drop-in and reload the user manager.
- Restart the service so the new binary is loaded.
- Verify the service remains enabled, active, and listening on `0.0.0.0:3010`.
- Verify `/api/health` returns HTTP 200.
- Verify an authentication challenge is no longer required for a non-health API
  route and no `WWW-Authenticate: Basic` header is returned.
- Verify a cross-site mutating `/api/*` request is still rejected with HTTP 403.

## Acceptance Criteria

- [x] `pnpm build:web` succeeds.
- [x] The release `web-server` example build succeeds.
- [x] Installed binary matches `src-tauri/target/release/examples/server`.
- [x] Installed `dist-web/index.html` matches the freshly built asset.
- [x] The installed unit matches `deploy/systemd/cc-switch-web.service`.
- [x] Legacy `cc-switch-web.service.d/auth.conf` is absent after install.
- [x] The service has a new main PID/start timestamp and is active/running.
- [x] Port `3010` listens on `0.0.0.0` through the new process.
- [x] `GET /api/health` returns HTTP 200 with the expected health payload.
- [x] `GET /api/providers/get-providers?app=claude` without credentials returns
  HTTP 200 and no Basic Auth challenge.
- [x] Cross-site `POST /api/nonexistent` returns HTTP 403.
- [x] Direct-client `POST /api/nonexistent` without browser-origin headers
  reaches routing and returns HTTP 404 rather than an auth response.
- [x] No startup failure or panic is present in the post-restart service log.

## Definition of Done

- Deployment and verification commands complete successfully.
- Existing application data is preserved.
- No unrelated service or source file is changed.
- Deployment results are recorded in this PRD.
- Trellis task is committed, archived, and journaled after verification.

## Technical Approach

- Record the pre-deploy PID/start time and legacy drop-in state.
- Run `./scripts/install-cc-switch-web-service.sh` from the repository root.
- Compare installed files with build outputs and tracked systemd unit.
- Probe systemd, listener, health, unauthenticated API behavior, same-origin
  intent behavior, and recent logs.

## Decision (ADR-lite)

**Context**: The persistent service is healthy but still runs the pre-removal
binary with a Basic Auth drop-in.

**Decision**: Use the repository-supported installer to perform an in-place
replacement and explicit restart while preserving the existing service and data
paths.

**Consequences**: The service will be briefly unavailable during restart. After
deployment, every host able to reach port 3010 can operate the full Web API;
same-origin intent checks remain only for browser-originated mutating requests.

## Out of Scope

- Source-code changes.
- Changing port, host, data directory, logging level, or systemd service name.
- Resetting or migrating `~/.cc-switch` data.
- Adding replacement application-layer authentication.
- Changing firewall, reverse proxy, or network access controls.

## Technical Notes

- Source commit at planning time: `73b6f854`.
- Previous deployment task:
  `.trellis/tasks/archive/2026-07/07-09-replace-persistent-web-service/prd.md`.
- Applicable contract:
  `.trellis/spec/frontend/quality-guidelines.md` scenario
  `Unauthenticated Web API + Same-Origin Intent Guard`.

## Deployment Result (2026-07-10)

- Ran `./scripts/install-cc-switch-web-service.sh` successfully.
- `pnpm build:web` completed; Vite reported only existing browser-data,
  mixed-import, and large-chunk warnings.
- The release Web server build completed successfully in 4m 47s.
- Installer removed
  `~/.config/systemd/user/cc-switch-web.service.d/auth.conf` and systemd now
  reports an empty `DropInPaths` value.
- Installed binary, `dist-web/index.html`, and systemd unit match their
  repository build/source counterparts byte-for-byte.
- Service restarted from PID `277002` to PID `1627869` at
  `2026-07-10 15:09:39 CST`; it is enabled, active, and running.
- `ss` confirms PID `1627869` listens on `0.0.0.0:3010`.
- `GET /api/health` returned HTTP 200 with
  `{"status":"ok","name":"cc-switch","version":"3.16.5"}`.
- `GET /api/providers/get-providers?app=claude` returned HTTP 200 without
  credentials and without a Basic Auth challenge.
- Cross-site `POST /api/nonexistent` returned HTTP 403; the same direct-client
  request without browser-origin headers returned HTTP 404.
- The old service logged a bounded graceful-shutdown warning for long-lived SSE
  clients, then stopped cleanly. The new service log contains no
  panic/fatal/error/failed lines and confirms `Using data directory:
  /home/orion/.cc-switch` plus `listening on http://0.0.0.0:3010`.
- Updated `.trellis/spec/frontend/quality-guidelines.md` to use the real
  provider-list route (`/api/providers/get-providers?app=claude`) instead of the
  nonexistent `/api/providers` example discovered during deployment checking.
