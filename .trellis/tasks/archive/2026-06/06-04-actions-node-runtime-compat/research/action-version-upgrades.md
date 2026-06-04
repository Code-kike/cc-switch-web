# Action Version Upgrade Research

## Context

The latest PR CI run succeeded but emitted a GitHub Actions annotation:
`actions/checkout@v4` runs on the deprecated Node.js 20 action runtime.

The repository's existing changelog also documents upstream dependency bumps for
the same class of maintenance:

* `actions/checkout` 4 -> 6
* `pnpm/action-setup` 5 -> 6
* `softprops/action-gh-release` 2 -> 3
* `actions/stale` 9 -> 10

## Verified Tags

The following target tags were verified with `git ls-remote --tags`:

* `actions/checkout@v6`
* `pnpm/action-setup@v6`
* `actions/stale@v10`
* `softprops/action-gh-release@v3`

## Decision

Upgrade only the action major versions that are stale in project workflows.
Keep the workflow commands, app runtime `node-version: "20"`, triggers,
permissions, and release/stale behavior unchanged.

## Validation

* Run a local YAML parse check for edited workflow files.
* Run `pnpm typecheck` and `pnpm test:unit` because CI workflow edits should not
  hide application regressions.
* Push and verify the PR CI run completes without the old Node.js 20 action
  annotation.
