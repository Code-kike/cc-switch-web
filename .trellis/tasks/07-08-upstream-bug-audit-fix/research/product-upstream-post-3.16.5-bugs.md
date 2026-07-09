# Product upstream post-v3.16.5 bug snapshot

Source: `farion1231/cc-switch` product upstream, fetched into `refs/remotes/product-upstream/main` on 2026-07-08.

## Scope

The first audit slice follows the user's selected source: product-upstream inherited bugs. The useful comparison point is this fork's current `v3.16.5` sync baseline versus product-upstream commits after tag `v3.16.5`.

## Upstream refs

- `v3.16.5` tag target: `8d1b3306d09a27b9d8fc29694791d8421aba5f93`
- `product-upstream/main` at fetch time: `d271d60cf960e3366ddacae0285aa705603b3598`
- Current fork branch at research time: `fix/web-audit-phase1-2`

## Candidate fixes selected for first implementation batch

### Volcano GLM 5.2 image fallback

- Upstream commit: `52534618 fix(proxy): close media fallback gaps for Volcano GLM 5.2 image 400s`
- Upstream issue: `#5025`
- Reason to port: small backend-only proxy media-sanitizer fix; directly addresses image blocks being forwarded to text-only GLM 5.2 paths and reactive fallback missing `"Model only support text input"` errors.
- Expected local impact: improves Claude/Codex proxy robustness without weakening Web security hardening.

### Codex free-plan 30-day quota window

- Upstream commit: `7a7d41c8 fix(subscription): display Codex free-plan 30-day quota window (#3651) (#4886)`
- Upstream issue: `#3651`
- Reason to port: small backend/frontend i18n fix; current fork maps unknown 30-day windows dynamically but frontend `TIER_I18N_KEYS` does not whitelist `"30_day"`, so a free Codex account can render no visible quota.
- Expected local impact: subscription quota footer shows free-plan quota and reset window correctly.

### OpenCode session resume command

- Upstream commit: `0cda8d46 fix: 更新 OpenCode 会话恢复命令 (#2359)`
- Upstream PR: `#2359`
- Reason to port: tiny session-manager correctness fix; current fork still emits `opencode session resume <id>`, while product upstream switched to `opencode -s <id>`.
- Expected local impact: copied/resumed OpenCode sessions use the current CLI command.

## Candidate fixes selected for second implementation batch

### Usage transient-failure keep-last-good

- Upstream commit: `2df2212c fix(usage): reject transient transport failures so retry and keep-last-good work`
- Reason to port after the first batch: high-value cross-layer correctness fix
  touching balance, coding plan, subscription, provider usage, query cache
  behavior, command emit semantics, and tests.
- Expected local impact: transient transport failures reject and retry without
  poisoning the Web cache bridge or hiding last-good usage/quota data.

## Deferred candidates

### Codex renamed session titles

- Upstream commit: `e606adfa fix(codex): display renamed session titles (#4927)`
- Reason to defer from first batch: valuable but larger change that introduces shared Codex state DB resolution and extra SQLite title lookup paths. It should be ported deliberately with session-manager tests.
- Follow-up recommendation: next batch if the first batch passes cleanly.

### Project profiles

- Upstream commits: `8f018a2d` through `9f7642e2`, plus related profile fixes.
- Reason to defer: this is a substantial new feature area, not a narrow inherited-bug fix. It changes database schema, frontend UI, proxy lifecycle, tray behavior, and provider switching.

## Non-actionable for this batch

- Release supply-chain/workflow changes and documentation-only Kimi routing guides are out of scope for the Web-first bug-fix batch.
- Desktop/platform packaging issues are out of scope unless they affect the standalone Web server or shared backend behavior.
