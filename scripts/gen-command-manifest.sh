#!/usr/bin/env bash
# Generate `commands.manifest.json` at the repo root by parsing the v3.14.1
# Tauri command surface (`#[tauri::command]` + `tauri::generate_handler!`).
#
# Run from the project root:
#   bash scripts/gen-command-manifest.sh [--check]
#
# Use `--check` in CI to fail when the manifest is stale.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${PROJECT_ROOT}/src-tauri"

cargo run \
    --release \
    --bin gen-command-manifest \
    --no-default-features \
    -- "$@"
