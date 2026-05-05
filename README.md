<div align="center">

# cc-switch-web

### Web-First Remote Manager for Claude Code, Codex, Gemini CLI, OpenCode & OpenClaw

[![Platform](https://img.shields.io/badge/platform-Linux%20Server%20%7C%20Browser-lightgrey.svg)](#current-deployment-model)
[![Built with Tauri](https://img.shields.io/badge/backend-Tauri%202%20Web%20Server-orange.svg)](https://tauri.app/)
[![Frontend](https://img.shields.io/badge/frontend-React%20%2B%20Vite-646cff.svg)](#development)



</div>

## Overview

`cc-switch-web` is a web-first deployment of the `cc-switch` ecosystem, focused on **remote access**, **always-on server deployment**, and **browser-based management** of local AI CLI tool configurations.

If you manage Claude Code, Codex, Gemini CLI, OpenCode, or OpenClaw on a machine you often access remotely, this project gives you a Web UI instead of requiring a local desktop app.

## Acknowledgement

This project directly benefits from two upstream projects:

- [`farion1231/cc-switch`](https://github.com/farion1231/cc-switch): provides the mature product foundation, data model, provider management logic, multi-tool integration, and core backend capabilities.
- [`Laliet/CC-Switch-Web`](https://github.com/Laliet/CC-Switch-Web): demonstrates the browser-based direction and validates the value of remote Web management for the `cc-switch` workflow.

`cc-switch-web` is built as a practical convergence of those two lines of work: keeping up with newer `cc-switch` capabilities while making them available through a remotely accessible Web deployment model.

## Why This Project

The original `cc-switch` desktop app is strong for local usage, but it is not ideal when:

- your main machine is accessed over SSH or remote desktop
- you want the service to stay online after reboot
- you need browser access from another device on your LAN
- you want to manage providers, prompts, MCP, skills, and sessions without launching a desktop GUI

This repository focuses on solving that gap.

## What It Provides

- **Web-first management UI** for Claude Code, Codex, Gemini CLI, OpenCode, and OpenClaw
- **Remote browser access** on a self-hosted machine
- **Systemd service deployment** for always-on use and reboot auto-start
- **Reuse of existing `~/.cc-switch` data** instead of forcing a separate data silo
- **Modern `cc-switch` feature base** rather than staying limited to early web prototypes
- **Standalone web server runtime** for Linux server or workstation deployment

## Current Deployment Model

This project is currently optimized for self-hosted Linux usage.

Typical deployment:

1. Build the Web frontend with `pnpm build:web`
2. Build the standalone server with Cargo
3. Install the binary and static assets
4. Run it as a `systemd --user` service
5. Access it from `http://<host>:3010`

In this repository, the service deployment already supports:

- bind address `0.0.0.0`
- default port `3010`
- `systemd --user` auto-start
- static asset installation to `~/.local/share/cc-switch-web/dist-web`
- data reuse from `~/.cc-switch`

## Repository Layout

- `src/`: React + Vite frontend
- `src-tauri/`: shared backend logic and standalone web server
- `deploy/systemd/`: user service unit
- `scripts/install-cc-switch-web-service.sh`: build and install script for persistent service deployment
- `dist-web/`: generated Web frontend build output

## Development

### Frontend Development

```bash
pnpm install
pnpm dev:web
```

### Web Build

```bash
pnpm build:web
```

### Standalone Web Server

```bash
cargo run --manifest-path src-tauri/Cargo.toml \
  --no-default-features \
  --features web-server \
  --example server
```

### Service Installation

```bash
./scripts/install-cc-switch-web-service.sh
```

### Service Management

```bash
systemctl --user status cc-switch-web.service --no-pager
systemctl --user restart cc-switch-web.service
journalctl --user -u cc-switch-web.service -f
```

## Data Directory

The current service deployment is configured to reuse:

```bash
~/.cc-switch
```

That means existing providers, prompts, skills, backups, and related data can continue to be used by the Web service, rather than being reset on reboot or split into a second database by default.

## Project Positioning

This repository is not trying to replace the upstream projects conceptually.

Its role is narrower and more practical:

- track newer `cc-switch` functionality
- expose it through a remotely usable Web UI
- support long-running self-hosted deployment
- reduce the gap between desktop-oriented `cc-switch` and earlier web-oriented prototypes

## Status

The project already has a working standalone Web runtime and persistent service deployment path, but feature parity work is still ongoing in some areas. The main direction is to keep synchronizing newer `cc-switch` capabilities into the Web experience and close remaining management gaps.

## Upstream Projects

- `cc-switch`: https://github.com/farion1231/cc-switch
- `CC-Switch-Web`: https://github.com/Laliet/CC-Switch-Web

## License

This repository currently follows the license terms included in this project tree. Review the `LICENSE` file before redistribution or derivative use.
