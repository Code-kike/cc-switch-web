# Decide Trellis Scaffold Tracking Policy

## Goal

Make the repository's Trellis/agent scaffold state intentional so routine development starts from a clean working tree and future AI sessions can use the same project workflow files.

## What I Already Know

* The user asked to proceed with the recommended engineering cleanup.
* Recent code-review fixes have already been committed and the previous task was archived.
* The remaining dirty files are untracked Trellis/agent scaffold files, not business code.
* Existing tracked Trellis history already includes archived tasks, `.trellis/workspace/orion/`, and `.trellis/spec/frontend/quality-guidelines.md`.
* `.trellis/.gitignore` already ignores local developer identity and runtime state, but does not currently ignore `.trellis/.template-hashes.json`.
* Root `.gitignore` comments out `AGENTS.md`, which indicates project-level agent instructions are intended to be tracked.

## Assumptions

* This project will continue using Trellis in this repository, so reusable scaffold should be committed.
* Local runtime/cache files should stay untracked.
* No business-code behavior should change in this task.

## Requirements

* Track reusable Trellis scaffold:
  * `.agents/skills/`
  * `.trellis/scripts/`
  * `.trellis/spec/`
  * `.trellis/workflow.md`
  * `.trellis/config.yaml`
  * `.trellis/.version`
  * `.trellis/.gitignore`
  * `.trellis/workspace/index.md`
  * `AGENTS.md`
* Keep local/generated state out of git:
  * `.trellis/.developer`
  * `.trellis/.runtime/`
  * `.trellis/.template-hashes.json`
  * Python caches and temp/backup files already covered by `.trellis/.gitignore`
* Archive this task and record the session after the scaffold commit.

## Acceptance Criteria

* [ ] `git status --short` no longer shows reusable Trellis/agent scaffold as untracked after commit.
* [ ] `.trellis/.template-hashes.json` is ignored by git.
* [ ] No business-code files are modified.
* [ ] Trellis context commands still run.

## Out of Scope

* Changing Trellis workflow semantics.
* Editing generated skill/script content beyond ignore-policy hygiene.
* Pushing to remote.

## Technical Notes

* Relevant files inspected:
  * `.gitignore`
  * `.trellis/.gitignore`
  * `.trellis/config.yaml`
  * `.trellis/workspace/index.md`
  * `AGENTS.md`
