# Redeploy Web Server as a Persistent System Service

## Goal

Build the current `fix/web-audit-phase1-2` repository state, replace the
installed cc-switch Web binary and static assets, and verify the existing
systemd user service remains continuously available across process failure,
logout, and machine restart.

## What I Already Know

- The supported deployment entry point is
  `scripts/install-cc-switch-web-service.sh`.
- The existing service is the user unit `cc-switch-web.service` and is already
  `enabled` and `active (running)`.
- It runs as user `orion` from `/home/orion/.local/bin/cc-switch-web`, listens on
  `0.0.0.0:3010`, reads static assets from
  `/home/orion/.local/share/cc-switch-web/dist-web`, and preserves application
  data under `/home/orion/.cc-switch`.
- `loginctl show-user orion` reports `Linger=yes`, so the user manager and unit
  can start at boot and remain active without an interactive login.
- The system scope has no separate `cc-switch-web.service`; creating a second
  system unit would duplicate the listener and conflict on port 3010.
- The current deployed process predates the completed integrity repair commits,
  so a rebuild/restart is required to load them.
- The Web API remains intentionally unauthenticated and bound to `0.0.0.0`; the
  deployment relies on the existing network boundary for access control.

## Assumptions

- Preserve the existing service name, user-service scope, host, port, data
  directory, static asset directory, logging level, and unauthenticated posture.
- Use the repository-supported installer rather than introducing another
  deployment mechanism.
- Use the recommended rollback-safe path without asking further non-blocking
  questions.

## Open Questions

- None. Current service parameters and the supported deployment path are
  discoverable and already match the user's persistent-service requirement.

## Requirements

- Record the current source commit, service PID/start time, installed file
  checksums, unit configuration, and health state.
- Back up the installed binary, static assets, and systemd unit to a timestamped
  directory before replacement. Do not copy or mutate `/home/orion/.cc-switch`.
- Run `./scripts/install-cc-switch-web-service.sh` from the repository root.
- Preserve `HOST=0.0.0.0`, `PORT=3010`, `CC_SWITCH_DATA_DIR=%h/.cc-switch`,
  `CC_SWITCH_WEB_DIST_DIR=%h/.local/share/cc-switch-web/dist-web`, and
  `RUST_LOG=info`.
- Keep the service as a `systemd --user` unit with `Restart=on-failure`,
  `RestartSec=3`, `TimeoutStopSec=30`, and `WantedBy=default.target`.
- Confirm the user unit is enabled and `Linger=yes`; do not create a competing
  system-scope unit.
- Verify the installed binary, frontend entry point, and unit match the fresh
  build/source copies.
- Verify a new PID/start timestamp, an active listener on `0.0.0.0:3010`, a
  healthy API response, frontend HTTP success, and no startup panic/fatal error.
- Verify the unauthenticated API and browser same-origin intent behavior remain
  consistent with the accepted deployment contract.
- If installation or post-restart health verification fails, restore the
  timestamped backup and restart the previous unit.

## Acceptance Criteria

- [x] Pre-deploy state and rollback backup are recorded.
- [x] `pnpm build:web` succeeds through the installer.
- [x] The release Web server example build succeeds through the installer.
- [x] The installed binary matches the newly built server binary.
- [x] The installed `dist-web/index.html` matches the fresh frontend build.
- [x] The installed unit matches `deploy/systemd/cc-switch-web.service`.
- [x] `systemctl --user is-enabled cc-switch-web.service` returns `enabled`.
- [x] `systemctl --user is-active cc-switch-web.service` returns `active`.
- [x] `loginctl show-user orion -p Linger` remains `Linger=yes`.
- [x] The service has a new PID/start timestamp and listens on `0.0.0.0:3010`.
- [x] `GET /api/health` returns HTTP 200 with the expected payload.
- [x] The frontend root returns HTTP 200.
- [x] An unauthenticated provider-list request returns HTTP 200 with no Basic
  Auth challenge.
- [x] A cross-site mutating API request is rejected with HTTP 403, while a
  direct client without browser-origin headers reaches routing.
- [x] Post-restart logs contain no startup panic, fatal error, or systemd failure.
- [x] Existing data remains available from `/home/orion/.cc-switch`.

## Definition of Done

- Current source is built and deployed through the supported installer.
- The persistent user service is enabled, active, healthy, and boot-capable via
  linger.
- Rollback artifacts and verification evidence are recorded.
- No unrelated source, system service, firewall, or application data is changed.

## Technical Approach

1. Capture pre-deploy service/file evidence and create a timestamped rollback
   directory under `~/.local/share/cc-switch-web/deploy-backups/`.
2. Run the supported installer, which builds frontend and Rust release assets,
   installs the binary/assets/unit, reloads the user manager, enables the unit,
   and explicitly restarts it.
3. Compare installed artifacts, verify systemd/linger/listener state, probe the
   frontend and API security behavior, and inspect only the new invocation logs.
4. Restore the backup if any required health check fails.

## Decision (ADR-lite)

**Context**: A healthy persistent user service already exists, but it runs a
binary built before the latest repair commits. A separate system unit would
conflict with the existing listener and duplicate state ownership.

**Decision**: Perform a rollback-backed in-place update of the existing
`systemd --user` service and retain `Linger=yes` for boot/logout persistence.

**Consequences**: Port 3010 is briefly unavailable during restart. The Web API
continues to be reachable without application-layer authentication from every
host allowed by the current network boundary.

## Out of Scope

- Changing source code, host, port, data paths, log verbosity, or service name.
- Creating a system-scope service in parallel with the existing user unit.
- Changing firewall, reverse proxy, TLS, authentication, or network exposure.
- Deleting, migrating, or resetting `/home/orion/.cc-switch`.

## Technical Notes

- Current source HEAD at planning time: `2663f0c0`.
- Existing pre-deploy PID: `1627869`; service start:
  `2026-07-10 15:09:39 CST`.
- Existing service is enabled/active, `NRestarts=0`, and currently responds to
  `/api/health` with version `3.16.5`.
- Previous deployment record:
  `.trellis/tasks/archive/2026-07/07-10-deploy-updated-web-service/prd.md`.
- Deployment contract:
  `.trellis/spec/frontend/quality-guidelines.md`, scenarios
  `Unauthenticated Web API + Same-Origin Intent Guard` and
  `Standalone Web-Server Smoke Validation`.

## Deployment Result — 2026-07-10

- Source deployed: `2663f0c0216e31eb65977c8b2b097871f773aa9d`.
- Rollback backup:
  `/home/orion/.local/share/cc-switch-web/deploy-backups/20260710-192308`.
  The backup contains the prior executable, complete `dist-web`, and service
  unit. `/home/orion/.cc-switch` was not copied, replaced, or removed.
- Pre-deploy service: PID `1627869`, active since
  `2026-07-10 15:09:39 CST`, binary SHA-256
  `ac82641374f931fe214f863f15cc917e573ae1e0162b3f66ef0d2867e305ba6e`.
- Ran `./scripts/install-cc-switch-web-service.sh` successfully.
  - Vite production build completed in 23.75 seconds with only existing
    browser-data, mixed-import, and large-chunk warnings.
  - Rust release Web server build completed in 4 minutes 46 seconds.
- Installed artifact verification:
  - Binary matches `src-tauri/target/release/examples/server`; SHA-256
    `ebbc8411417e94a24862d10542041bf1a09ae16616b8e9f9d3ffd3d07f29896c`.
  - Installed `dist-web/index.html` matches the fresh build; SHA-256
    `62ed421860dc8959cc8c6e9b74f00a231aec08c0e4dcbb9e8a4bc5bf8f0b692f`.
  - Installed user unit matches `deploy/systemd/cc-switch-web.service` and has
    no drop-ins.
- Post-deploy service: PID `2075668`, active since
  `2026-07-10 19:28:36 CST`, `enabled`, `active`, `Result=success`,
  `NRestarts=0`.
- Persistence: `Linger=yes`; the service remains a single user unit with
  `WantedBy=default.target` and no competing system-scope unit.
- Listener: PID `2075668` owns `0.0.0.0:3010`.
- HTTP verification:
  - `GET /api/health` -> `200`,
    `{"status":"ok","name":"cc-switch","version":"3.16.5"}`.
  - `GET /` -> `200`.
  - unauthenticated `GET /api/providers/get-providers?app=claude` -> `200`,
    no `WWW-Authenticate` header.
  - cross-site `POST /api/nonexistent` -> `403`.
  - direct-client `POST /api/nonexistent` without browser-origin headers ->
    `404`, proving it reached routing rather than authentication middleware.
- Current invocation ID: `ae24e89111cf4a91b36e988acc97b375`.
  Its logs confirm the existing data directory, normal bootstrap, and listener;
  no panic, fatal, failed-start, thread panic, or segmentation-fault marker was
  found.
- Quality checks after deployment:
  - `pnpm typecheck` -> PASS.
  - `pnpm check:web-routes` -> PASS, 267 commands, missing 0.
  - `git diff --check` -> PASS.
