# Replace persistent web service with updated build

## Goal

Build the current repository state and replace the existing always-on local
`cc-switch-web` Web service with the updated binary and static Web assets.

## What I Already Know

- The user already has a persistent local Web service and wants it replaced with
  the updated project.
- The current user-level systemd service is `cc-switch-web.service`.
- The running process is `/home/orion/.local/bin/cc-switch-web`.
- The service listens on `0.0.0.0:3010` and reuses `~/.cc-switch`.
- The repository provides `scripts/install-cc-switch-web-service.sh` for this
  exact deployment path.
- The install script builds Web assets, builds the standalone Web server binary,
  installs files to `~/.local/bin` and `~/.local/share/cc-switch-web`, writes the
  user systemd unit, configures Basic Auth, enables the service, and explicitly
  restarts it.

## Assumptions

- Use the repository-supported installer instead of manually copying individual
  artifacts.
- Preserve the existing data directory at `~/.cc-switch`.
- Do not modify application source code for this operational task.
- Validate the replacement by checking systemd status and the local health route.

## Requirements

- Build the frontend with the Web configuration.
- Build the standalone Rust Web server in release mode with `web-server` feature.
- Replace the installed binary and static assets used by `cc-switch-web.service`.
- Ensure non-loopback Web auth is configured so the updated server can start.
- Restart the persistent user service so the new binary is actually loaded.
- Verify the service is active and responding.

## Acceptance Criteria

- [x] `cc-switch-web.service` is active after replacement.
- [x] The running process uses `/home/orion/.local/bin/cc-switch-web`.
- [x] The installed binary and static assets have current timestamps.
- [x] `GET /api/health` on `127.0.0.1:3010` returns a healthy response.
- [x] Worktree status is understood after build/deploy.

## Definition of Done

- Service replacement completed.
- Verification commands pass or any blocker is documented.
- No unrelated local service is stopped or modified.
- Task notes record the deployment outcome.

## Out of Scope

- Changing source code.
- Changing service port, host, data directory, or auth username unless required
  by the existing installer.
- Migrating or resetting `~/.cc-switch` data.

## Technical Notes

- Existing unit before replacement had custom `ALLOW_HTTP_BASIC_OVER_HTTP=1` and
  `RUST_LOG=info,cc_switch=debug`, while the repository unit uses `RUST_LOG=info`
  and configures Web Basic Auth through a preserved/generated drop-in.
- The updated server refuses a non-loopback bind unless
  `CC_SWITCH_WEB_AUTH_PASSWORD` is configured.
- Relevant files:
  - `scripts/install-cc-switch-web-service.sh`
  - `deploy/systemd/cc-switch-web.service`
  - `src-tauri/examples/server.rs`
  - `src-tauri/src/web_api/middleware/auth.rs`

## Deployment Result (2026-07-09)

- Ran `./scripts/install-cc-switch-web-service.sh`.
- Frontend `pnpm build:web` completed successfully.
- Rust release build completed successfully:
  `cargo build --release --manifest-path src-tauri/Cargo.toml --no-default-features --features web-server --example server`.
- Installed binary matches the build output:
  `~/.local/bin/cc-switch-web` == `src-tauri/target/release/examples/server`.
- Installed static index matches the build output:
  `~/.local/share/cc-switch-web/dist-web/index.html` == `dist-web/index.html`.
- Restarted user service `cc-switch-web.service`; active PID after replacement:
  `277002`.
- Health check passed:
  `GET http://127.0.0.1:3010/api/health` returned HTTP 200 with
  `{"status":"ok","name":"cc-switch","version":"3.16.5"}`.
- The installer generated
  `~/.config/systemd/user/cc-switch-web.service.d/auth.conf` with mode `0600`.
