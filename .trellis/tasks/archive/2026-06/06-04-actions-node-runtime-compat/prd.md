# Update GitHub Actions Node Runtime Compatibility

## Goal

Remove GitHub Actions Node.js 20 runtime deprecation annotations by upgrading
outdated workflow actions to their Node.js 24 compatible major versions, while
preserving the current CI/release/stale workflow behavior.

## What I Already Know

* The latest PR CI passed, but GitHub Actions emitted a Node.js 20 deprecation
  annotation for `actions/checkout@v4`.
* `.github/workflows/ci.yml` uses `actions/checkout@v4` in frontend and backend
  jobs, plus `pnpm/action-setup@v5`.
* `.github/workflows/release.yml` uses `actions/checkout@v4`,
  `pnpm/action-setup@v5`, and `softprops/action-gh-release@v2`.
* `.github/workflows/stale.yml` uses `actions/stale@v9`.
* Existing release notes in this repo already reference upstream dependency
  bumps: `actions/checkout` 4 -> 6, `pnpm/action-setup` 5 -> 6,
  `softprops/action-gh-release` 2 -> 3, and `actions/stale` 9 -> 10.

## Research References

* [`research/action-version-upgrades.md`](research/action-version-upgrades.md)
  records the target tags verified with `git ls-remote`.

## Assumptions

* Runtime compatibility should be handled by upgrading action major versions,
  not by suppressing the annotation with environment variables.
* The project should keep Node.js `20` as the application build/test runtime for
  now; the warning is about action runtime, not app runtime.
* No product code should change in this task.

## Requirements

* Update outdated GitHub Actions action versions in CI/release/stale workflows.
* Preserve existing workflow triggers, permissions, job structure, and command
  steps.
* Do not change release artifact behavior or stale issue policy.
* Verify the edited YAML and run relevant local checks.
* Push the branch and confirm GitHub CI passes after the workflow update.

## Acceptance Criteria

* [ ] `.github/workflows/ci.yml` no longer uses `actions/checkout@v4` or
  `pnpm/action-setup@v5`.
* [ ] `.github/workflows/release.yml` no longer uses outdated checkout, pnpm
  setup, or GitHub release action versions.
* [ ] `.github/workflows/stale.yml` no longer uses `actions/stale@v9`.
* [ ] Local verification passes.
* [ ] GitHub CI completes successfully after push.
* [ ] No business-code files are modified.

## Out of Scope

* Changing the app's Node.js version from 20.
* Rewriting the CI pipeline or release packaging logic.
* Changing branch triggers, schedule cadence, stale labels, release body, or
  artifact upload/download behavior.

## Technical Notes

* Likely files:
  * `.github/workflows/ci.yml`
  * `.github/workflows/release.yml`
  * `.github/workflows/stale.yml`
* Expected version bumps:
  * `actions/checkout@v4` -> `actions/checkout@v6`
  * `pnpm/action-setup@v5` -> `pnpm/action-setup@v6`
  * `softprops/action-gh-release@v2` -> `softprops/action-gh-release@v3`
  * `actions/stale@v9` -> `actions/stale@v10`
