#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_NAME="cc-switch-web.service"

BIN_DIR="${HOME}/.local/bin"
SHARE_DIR="${HOME}/.local/share/cc-switch-web"
DIST_DIR="${SHARE_DIR}/dist-web"
DATA_DIR="${HOME}/.cc-switch"
SYSTEMD_DIR="${HOME}/.config/systemd/user"
SERVICE_PATH="${SYSTEMD_DIR}/${SERVICE_NAME}"

echo "[cc-switch-web] building web assets"
cd "${REPO_ROOT}"
pnpm build:web

echo "[cc-switch-web] building web server binary"
RUSTFLAGS="${RUSTFLAGS:--Awarnings}" cargo build --release \
  --manifest-path src-tauri/Cargo.toml \
  --no-default-features \
  --features web-server \
  --example server

echo "[cc-switch-web] installing files"
mkdir -p "${BIN_DIR}" "${DIST_DIR}" "${DATA_DIR}" "${SYSTEMD_DIR}"
install -m 0755 "${REPO_ROOT}/src-tauri/target/release/examples/server" "${BIN_DIR}/cc-switch-web"
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}"
cp -a "${REPO_ROOT}/dist-web/." "${DIST_DIR}/"
install -m 0644 "${REPO_ROOT}/deploy/systemd/${SERVICE_NAME}" "${SERVICE_PATH}"

DROPIN_DIR="${SYSTEMD_DIR}/${SERVICE_NAME}.d"
AUTH_CONF="${DROPIN_DIR}/auth.conf"
echo "[cc-switch-web] removing legacy web auth drop-in"
if [ -f "${AUTH_CONF}" ]; then
  rm -f "${AUTH_CONF}"
  rmdir "${DROPIN_DIR}" 2>/dev/null || true
  echo "[cc-switch-web] removed ${AUTH_CONF}"
fi

echo "[cc-switch-web] enabling user service"
systemctl --user daemon-reload
systemctl --user enable "${SERVICE_NAME}"
# `enable --now` does NOT restart an already-running service, so a re-install would
# keep the OLD binary running. Restart explicitly to load the freshly-built binary.
systemctl --user restart "${SERVICE_NAME}"

echo "[cc-switch-web] service status"
systemctl --user --no-pager --full status "${SERVICE_NAME}" || true

echo
echo "Installed ${SERVICE_NAME}"
echo "URL: http://127.0.0.1:3010"
echo "LAN: http://$(hostname -I | awk '{print $1}'):3010"
echo "Auth: none (unauthenticated Web API)"
echo "Logs: journalctl --user -u ${SERVICE_NAME} -f"
