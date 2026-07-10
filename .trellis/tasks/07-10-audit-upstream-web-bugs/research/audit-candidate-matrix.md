# Audit Candidate Matrix

## Baseline

- Product upstream: `farion1231/cc-switch`.
- Previous local audit snapshot: product-upstream commit `d271d60c` on 2026-07-08.
- Current product-upstream main: `98ccde00` on 2026-07-09.
- Current fork baseline: cc-switch 3.16.5 on branch `fix/web-audit-phase1-2`.
- Previous local audit already ported media fallback, Codex 30-day quota, OpenCode resume command, and transient usage keep-last-good behavior.

## Confirmed High-Risk Candidates

### C1. Codex common-config extraction leaks provider-scoped artifacts

- Evidence: product-upstream commit `473c2aaa`; open upstream reports include #5174 and the configuration-loss/MCP reports #5149 and #4697.
- Local proof: `ProviderService::extract_codex_common_config` removes only `model`, `model_provider`, top-level `base_url`, and `model_providers`. It currently preserves top-level `experimental_bearer_token`, `model_catalog_json`, `wire_api`, `[mcp_servers]`, and legacy `[mcp.servers]`.
- Existing local tests explicitly preserve some of these fields, so the current regression contract encodes behavior product upstream has now classified as unsafe.
- Impact: API credentials, provider routing choices, model-catalog pointers, and MCP projections can bleed across providers; deleted MCP entries can be resurrected.
- Initial priority: critical configuration-integrity repair.

### C2. Codex single-MCP sync can wipe an unparseable config

- Evidence: product-upstream commit `8b1ce764`.
- Local proof: `sync_single_server_to_codex` catches a TOML parse failure, creates a new empty `DocumentMut`, inserts one MCP server, and writes it over the existing file.
- Impact: malformed or temporarily edited `config.toml` can lose provider, model, plugin, desktop, feature, and comment sections.
- Initial priority: critical data-loss prevention; small, high-confidence fix.

### C3. Codex common-config editor reserializes the entire TOML document

- Evidence: product-upstream commit `88d5ffba`.
- Local proof: `updateTomlCommonConfigSnippet` uses `smol-toml` parse/deep-merge/stringify. Local comments explicitly acknowledge that this drops comments and layout, while treating the behavior as acceptable.
- Upstream proof: the fix moves merge/remove to backend `toml_edit` and adds last-operation-wins plus stale-baseline guards for async form races.
- Impact: user-authored comments/order are destroyed; rapid toggle/save or concurrent hand-editing can apply stale results.
- Initial priority: high, but broader than C2 because it crosses frontend, backend, API adapters, and tests.

### C4. Shared atomic writes replace symbolic links

- Evidence: upstream issue #5129; especially relevant to Linux/NixOS/dotfiles-managed deployments.
- Local proof: `config::atomic_write` creates a sibling temporary file and renames it over the path. When the path is a symbolic link, rename replaces the link instead of updating its target.
- Impact: Web-first Linux users can silently lose dotfiles/NixOS ownership of OpenCode, Codex, Claude, Gemini, or other managed configuration paths.
- Initial priority: high for the Web-first target audience, but requires an explicit atomicity/security design because blindly following symlinks introduces target and race considerations.

### C5. Provider endpoint updates are ignored on existing providers

- Evidence: upstream issue #5099.
- Local proof: `Database::save_provider` removes `meta.custom_endpoints` before serializing metadata, inserts endpoint rows only for new providers, and performs no endpoint reconciliation in the update branch.
- Impact: edited `base_url` or endpoint lists can leave `provider_endpoints` stale, causing model discovery, tests, health checks, or endpoint selection to call an old URL.
- Initial priority: high correctness issue with a transactional database fix and focused DAO/service tests.

## Confirmed Medium-Risk Candidates

### M1. MCP projection failures are cross-app coupled

- Evidence: product-upstream commit `11c173c7`.
- Local proof: provider save/switch paths call `McpService::sync_all_enabled`; the loop returns on the first application error. A corrupt unrelated app config can report a target provider switch as failed after the DB/live mutation already happened and can block later applications from projection.
- Impact: false failure messages, partial state, and unrelated-app coupling.

### M2. Partial MCP import failures are hidden

- Evidence: product-upstream commit `94fc1cc0`.
- Local proof: `import_from_all_apps` logs per-app errors but returns success whenever at least one server was imported; `useImportMcpFromApps` refreshes only on success.
- Impact: users see an incomplete import as success and may not see successfully persisted partial results when the mutation rejects.

### M3. Unified Codex session-history toggle can drop MCP projection

- Evidence: product-upstream commit `6d2ee247` and upstream issue #5131.
- Local outcome: not directly applicable. This fork does not contain the upstream unified Codex session-history feature, so adding that feature solely to port the fix would expand product scope.
- Reused principle: existing provider-save/switch and per-application sync paths now re-project the complete enabled Codex MCP set, including removal of live orphan entries.
- Impact avoided: the first tranche gains MCP self-healing without introducing an unrelated session-history feature.

### M4. Live provider imports skip updates to existing entries

- Evidence: product-upstream commits `e191af4a` (OpenCode) and `e78aa8a` (OpenClaw/Hermes).
- Local proof: all three import functions compute existing IDs and skip them rather than updating their stored configuration.
- Impact: live config edits are never reflected back into existing database providers, leaving stale Web UI and later write-back risk.

### M5. Codex renamed session titles are not displayed

- Evidence: product-upstream commit/PR `e606adfa` / #4927; explicitly deferred by the previous local audit.
- Local proof: the Codex session provider derives titles from JSONL content and has no `state_5.sqlite` renamed-title lookup or shared state-DB resolver.
- Impact: session organization displays stale/generated titles instead of user-renamed titles.

## Confirmed Lower-Risk Candidate

### L1. Usage dashboard refresh interval is not persisted

- Evidence: product-upstream commit/PR `98ccde00` / #5057, fixing issue #4939.
- Local proof: `UsageDashboard` initializes `refreshIntervalMs` to 30 seconds in component state; there is no settings field or save/rollback path.
- Impact: user preference resets whenever the dashboard remounts.

## Open Reports Requiring More Validation

### O1. Codex model catalog casing for Codex 0.144+

- Evidence: upstream issue #5182 reports that generated snake_case model catalog fields are ignored by Codex Desktop 0.144+, which expects camelCase names.
- Local proof: generated entries currently use snake_case fields such as `display_name`, `context_window`, and template-level reasoning fields.
- Remaining uncertainty: compatibility expectations across Codex CLI/Desktop versions and whether a dual-schema or version-gated writer is safe.

### O2. Cross-provider API key corruption report

- Evidence: upstream issue #5174 has multiple independent confirmations on 3.16.5.
- Strong local hypothesis: C1 preserves top-level `experimental_bearer_token` in the shared common-config snippet, which can propagate one provider's credential into another provider.
- Remaining work: create a minimal local regression that proves the exact save/switch path before claiming #5174 is fully explained.

## Recommended Repair Strategy

Use severity-first, evidence-gated tranches instead of bulk upstream merging:

1. Configuration integrity and data-loss prevention: C1, C2, C5, plus the accepted context-aware managed/restricted write contract for C4.
2. Cross-app consistency and self-healing: C3, M1, M2, M3, M4.
3. User-visible correctness: M5, L1, and O1 after compatibility research.

Each accepted item should have a focused regression test and should be adapted manually to preserve Web-server behavior and prior security hardening.
