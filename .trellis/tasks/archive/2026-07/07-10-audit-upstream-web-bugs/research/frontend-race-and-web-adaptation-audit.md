# Frontend Race and Web Adaptation Audit

## Scope

Read-only review of frontend state transitions, asynchronous operations, and desktop-to-Web behavior. Findings below are limited to high-confidence behavioral defects rather than style concerns.

## Critical Findings

### F1. Workspace file editor can save file A content into file B

- Location: `src/components/workspace/WorkspaceFileEditor.tsx:39-60`.
- Trigger: start loading file A, close it and open file B, then let A's slower request resolve last.
- Root cause: asynchronous reads have no cancellation, generation token, or filename check; an obsolete response can overwrite the content state while the visible filename/save target is already B.
- Impact: direct cross-file content corruption.
- Fix contract: only the latest `filename + isOpen` request may update content/loading; save must remain disabled until the currently selected file has completed its own load.
- Regression test: resolve B before A and assert the editor and `writeFile` call remain bound to B.

### F2. Daily Memory editor has the same cross-file race

- Location: `src/components/workspace/DailyMemoryPanel.tsx:207-223,243-258`.
- Trigger: open memory file A, return to the list and open B before A finishes; A resolves or rejects after B is selected.
- Root cause: concurrent reads share `content`, `loadingContent`, and `editingFile` without request identity checks. An obsolete request can overwrite B, close B, or enable saving before B loads.
- Impact: one day's memory can be written into another day's file.
- Fix contract: success, failure, and finalization must all be latest-request-only; saving must bind to the successfully loaded file identity.
- Regression tests: inverse-order success and obsolete A failure while B remains pending.

### F3. Web "Restart now" is a no-op presented as a real restart

- Locations: `src/components/settings/SettingsPage.tsx:150-167,516-542`; `src-tauri/src/web_api/handlers/system.rs:123-125`.
- Trigger: change an application config directory in Web mode and accept the restart prompt.
- Root cause: the frontend reuses desktop restart semantics while the Web endpoint only returns `true` and never restarts the process.
- Impact: users believe restart-required settings are active when the service has not restarted.
- Fix contract: Web mode must show an explicit service-restart instruction unless a real supervisor-controlled restart mechanism exists.

## High Findings

### F4. Managed Device Code authentication leaves zombie timers

- Location: `src/components/providers/forms/hooks/useManagedAuth.ts:81-127`.
- Trigger: the first poll succeeds before interval/expiry handles are stored.
- Root cause: the first asynchronous poll starts before timers are assigned, so its success cannot cancel timers created immediately afterward; async interval callbacks can also overlap.
- Impact: polling continues after success and the expiry timer can later replace success with an error.
- Fix contract: use generation-scoped recursive timeout polling; schedule the next poll only after the current one completes and remains pending.

### F5. Provider query failures are converted into successful empty data

- Location: `src/lib/query/queries.ts:62-88`.
- Trigger: provider list/current lookup fails during a transient network error or Web-service restart.
- Root cause: the query catches errors and returns an empty provider map/current ID, preventing React Query error, retry, and last-good behavior.
- Impact: existing providers disappear and the UI can show a false empty state.
- Fix contract: provider-list transport failures must reject; previous data should remain visible while retry/error state is surfaced.

### F6. Concurrent whole-settings autosaves can apply stale snapshots last

- Locations: `src/components/settings/SettingsPage.tsx:170-188`; `src/hooks/useSettings.ts:187-227`; `src/lib/query/mutations.ts:344-353`.
- Trigger: rapidly modify two settings while two full Settings payloads are in flight and the older snapshot completes last.
- Root cause: no serialized writer, revision/CAS, latest-only guard, or merged patch queue.
- Impact: later user changes can be overwritten by an older request or intermediate refetch.
- Fix contract: use a single-writer queue with merged pending changes, or versioned/field-level persistence whose cache accepts only the latest revision.

## Medium Findings

### F7. Daily Memory search can display stale results

- Location: `src/components/workspace/DailyMemoryPanel.tsx:95-130`.
- Trigger: a search for `foo` resolves after a newer search for `foobar`.
- Root cause: debounce cancels only unsent timers, not requests already in flight.
- Impact: the query field and displayed results disagree; stale finalization can also clear the loading state early.
- Fix contract: use abortable or generation-scoped searches.

### F8. Web Device Code verification page is opened outside the user gesture

- Locations: `src/components/providers/forms/hooks/useManagedAuth.ts:62-79`; `src/lib/api/settings.ts:251-253`.
- Trigger: Web Device Code login waits for API and clipboard operations before calling `window.open`.
- Root cause: the browser no longer associates the popup with the original click and can block it; a `null` return is ignored.
- Impact: verification-page opening fails silently.
- Fix contract: provide an explicit link/button, or synchronously pre-open a window and report popup blocking.
