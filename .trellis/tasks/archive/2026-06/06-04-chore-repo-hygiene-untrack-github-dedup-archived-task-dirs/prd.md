# chore: repo hygiene — untrack .github + dedup archived task dirs

## Goal
Two minor repo-hygiene fixes surfaced during this session's PR work.

## Changes
1. **`.gitignore`: remove the `.github` line (31).** Dormant trap — `.github/` files (workflows, ISSUE_TEMPLATE, FUNDING) are ALREADY tracked (12 files), but the rule means a NEW `.github` file (e.g. a new workflow) would not be auto-added (`git add` skips it, needs `-f`). Removing the line lets `.github` track normally. No currently-tracked file changes status.
2. **Dedup archived task dirs.** `.trellis/tasks/{00-bootstrap-guidelines,05-13-config-usage-query,05-21-sync-upstream-v315,05-30-sync-upstream-v316}/` are tracked in BOTH the active `tasks/` location AND `tasks/archive/2026-06/` (a squash-merge artifact — `task.py archive`'s move didn't net out the deletion). `git rm -r` the active-location copies (keep the `archive/2026-06/` ones). Fixes `get_context --mode record` listing 4 already-done tasks as "active."

## Acceptance Criteria
- [ ] `.gitignore` no longer contains `.github`; `git check-ignore .github/workflows/ci.yml` → not ignored; the 12 tracked `.github` files unchanged.
- [ ] The 4 task dirs exist ONLY under `.trellis/tasks/archive/2026-06/`; `get_context --mode record` no longer lists them as active (only this hygiene task, until it too is archived).
- [ ] No unintended changes; nothing else staged.

## Out of Scope
- `commands.manifest.json` gitignore entry (same class, not flagged this round); dependabot PRs #10/#12 (reviewed separately); deferred security items.

## Technical Notes
- Pure repo-config/bookkeeping; no source/build behavior change. Branch `chore/repo-hygiene-gitignore-task-dedup` off main; small PR.
