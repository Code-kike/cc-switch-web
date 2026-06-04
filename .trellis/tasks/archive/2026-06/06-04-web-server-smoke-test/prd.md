# Run Web-Server Smoke Test

## Goal

Run an end-to-end web-server smoke test after the review fixes and scaffold cleanup so the pushed branch is validated in a real standalone server flow, not only unit tests and CI compile checks.

## What I Already Know

* The branch `chore/repo-hygiene-gitignore-task-dedup` has been pushed and GitHub CI passed.
* The repository has a `pnpm smoke:web-server` script.
* The smoke script starts the Rust web-server example with isolated `CC_SWITCH_DATA_DIR`, `CC_SWITCH_TEST_HOME`, and `CC_SWITCH_WEB_DIST_DIR`.
* The server entry point is `cargo run --no-default-features --features web-server --example server`.
* No business-code changes are expected for this task.

## Assumptions

* Existing smoke script coverage is the preferred project-level validation for Web API behavior.
* If the smoke script fails due to environment/test flake, investigate and report clearly before changing code.
* If a real product defect is found, create a separate fix task rather than mixing repairs into this smoke-test task.

## Requirements

* Confirm working tree starts clean except this task's Trellis files.
* Run the project web-server smoke script.
* Verify the standalone server starts and completes API smoke coverage.
* Confirm working tree remains clean of business-code changes.
* Archive this task and record the session after validation.

## Acceptance Criteria

* [ ] `pnpm smoke:web-server` completes successfully, or failures are clearly categorized.
* [ ] GitHub CI status from the previous push remains successful.
* [ ] No business-code files are modified by the smoke run.
* [ ] Current Trellis task is archived and journaled.

## Out of Scope

* Fixing newly discovered defects.
* Changing CI workflows.
* Manual browser UX inspection beyond basic availability checks unless the scripted smoke leaves a gap.

## Technical Notes

* Smoke script: `scripts/smoke-web-server.mjs`
* Server entry: `src-tauri/examples/server.rs`
* Package script: `pnpm smoke:web-server`
