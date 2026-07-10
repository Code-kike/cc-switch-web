# Backend State-Integrity Audit

## Scope

Read-only review of database restore, provider transitions, Web runtime directory semantics, proxy restart, cloud snapshot application, and persistence helpers.

## Critical Finding

### B1. Uploaded SQL backups execute arbitrary SQLite statements

- Locations: `src-tauri/src/database/backup.rs:94-166`; `src-tauri/src/web_api/handlers/config.rs:683`.
- Current validation: the input is accepted when it begins with `-- CC Switch SQLite 导出`, then the full payload is passed to `Connection::execute_batch` on a temporary database.
- Exploit surface: a forged file can include `ATTACH`/`DETACH`, path-writing SQLite operations, malicious triggers/views, virtual tables, or other statements outside the declared backup contract.
- Boundary conflict: the accepted unauthenticated Web API makes reachable clients operators of cc-switch-web data, but it does not intentionally grant an unrestricted SQLite program or service-user filesystem write primitive.
- Required repair: preserve only the declared CC Switch restore capability. Reject external database attachment, temp objects, triggers/views/virtual tables, unexpected schemas/tables/indexes, and unapproved pragmas/functions; validate the resulting database before copying it into the live connection.
- Regression tests: a valid-header payload containing `ATTACH` and a payload containing a malicious trigger must be rejected without creating an external file or mutating the main database.

## High Findings

### B2. Provider DB/settings/live-file transitions are not atomic

- Locations: `src-tauri/src/services/provider/mod.rs:1471,1671,1957`.
- Trigger: adding, updating, or switching a provider when the live directory is read-only or a later MCP/live write fails.
- Root cause: DB current/settings mutations can commit before live configuration succeeds, with incomplete rollback.
- Impact: UI/DB and the CLI's actual live configuration permanently disagree.
- Repair contract: prepare and validate the live write before committing state where possible; otherwise capture and restore DB/settings/live snapshots on every post-mutation failure.

### B3. Web app-config-directory changes create a mixed runtime

- Locations: `src-tauri/examples/server.rs:114,229`; `src-tauri/src/web_api/handlers/config.rs:245`.
- Trigger: call `set-app-config-dir-override` while the Web server is running.
- Root cause: the global path cache changes immediately, but the open `Database` remains bound to the old database; startup environment later overrides the persisted choice again.
- Impact: settings/backups/files and DB operations split across two directories, and the selected path does not survive restart.
- Repair contract: Web mode must either reject hot directory changes and instruct a deployment restart, or perform a locked full migration plus AppState/database rebuild.

### B4. Proxy port reconfiguration destroys the working server before the new bind succeeds

- Location: `src-tauri/src/services/proxy.rs:2663`.
- Trigger: change a running proxy to an occupied/unbindable port.
- Root cause: the new config is persisted and the old server is stopped before the replacement listener is known to be viable.
- Impact: the proxy is down, DB contains the unusable port, and takeover live files may still point to the stopped address.
- Repair contract: prepare/bind first, then commit; or restore the old config/server/live takeover state on failure.

### B5. WebDAV and S3 can concurrently apply different snapshots

- Locations: `src-tauri/src/services/webdav_sync.rs:33`; `s3_sync.rs:26`; `sync_protocol.rs:309`; `webdav_sync/archive.rs:143`.
- Root cause: each transport has its own lock although both replace the same DB and Skills state.
- Impact: DB can come from snapshot A while Skills come from snapshot B; shared backup paths can also collide.
- Repair contract: one global snapshot-apply mutex and unique staging/rollback paths across every transport.

## Medium Findings

### B6. Windows replacement deletes the original before rename

- Location: `src-tauri/src/config.rs:260`.
- Trigger: antivirus, file sharing, permissions, disk error, or process interruption after `remove_file` but before successful rename.
- Impact: configuration or credential file disappears.
- Repair contract: use an OS replacement primitive that retains the original on failure, such as `ReplaceFileW`/appropriate replace-existing semantics.

### B7. Endpoint `last_used` never persists

- Locations: `src-tauri/src/services/provider/endpoints.rs:70`; `src-tauri/src/database/dao/providers.rs:75,180`; `src-tauri/src/database/schema.rs:48`.
- Root cause: the endpoint table has no `last_used` column; reads hard-code `None`, and the provider-update path ignores the detached endpoint map.
- Impact: endpoint ordering/selection cannot retain its usage timestamp across reloads.
- Repair contract: migrate a dedicated column and update it with a dedicated DAO operation.

## First-Tranche Implication

B1 joins the first tranche because it is a proven data-integrity and capability-boundary defect. B2 is also severity-eligible but spans several state machines; it should be decomposed into explicit add/update/switch scenarios rather than addressed through a generic refactor. B3-B7 remain documented for later tranche decisions unless implementation research proves they are prerequisites.
