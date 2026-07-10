# cc-switch-web Context

Vocabulary for the cc-switch-web product context and upstream synchronization work.

## Language

**Web-first fork**:
This repository: a browser/server-focused derivative of CC Switch that preserves remote management and long-running self-hosted deployment.
_Avoid_: desktop app, upstream

**Product upstream**:
The `farion1231/cc-switch` project, used as the source of versioned CC Switch product capabilities and release deltas.
_Avoid_: upstream, origin

**Web prototype upstream**:
The `Laliet/CC-Switch-Web` project, used as a reference for browser-based management direction rather than the default source for product-version synchronization.
_Avoid_: upstream

**Product upstream sync**:
A scoped effort to port selected product-upstream release deltas into the Web-first fork while preserving the Web deployment model.
_Avoid_: pull, merge, update

**Product-upstream inherited bug**:
A defect from product-upstream behavior that is present in the Web-first fork after synchronization or Web adaptation.
_Avoid_: upstream bug, original bug

**Web adaptation defect**:
A defect introduced when shared or desktop-oriented product behavior is exposed through the standalone Web runtime, browser UI, or headless deployment model.
_Avoid_: upstream bug, parity gap, Web-only quirk

**Constrained database restore**:
A restore that accepts only backup content belonging to declared CC Switch product state and cannot change state outside that boundary.
_Avoid_: SQL execution, trusted dump, unrestricted import

**Managed configuration target**:
A configuration location for an external application that cc-switch-web is explicitly authorized to maintain and keep consistent with the application's managed state.
_Avoid_: arbitrary path, file to overwrite

**Common configuration**:
Settings intentionally shared among multiple providers for the same application.
_Avoid_: provider defaults, global provider config

**Provider-scoped configuration**:
Settings belonging to one provider only, including credentials, routing choices, and provider-specific catalogs.
_Avoid_: common configuration, shared snippet

**Provider endpoint**:
A service address associated with one provider and application.
_Avoid_: base URL field, endpoint row

**MCP application assignment**:
The declared association stating which supported applications should enable a managed MCP server.
_Avoid_: MCP copy, live config entry

**Functional catalog delta**:
A product-upstream provider or preset catalog change that affects model availability, defaults, routing behavior, or usage/pricing correctness.
_Avoid_: catalog churn, provider update, preset polish

**Session organization**:
The product capability for browsing, grouping, and recovering saved coding sessions as part of session history.
_Avoid_: session category/group management, history folders, session UI

**Release-operation delta**:
A product-upstream change whose primary purpose is desktop updating, release packaging, or platform-specific launch behavior rather than Web-first product behavior.
_Avoid_: updater change, platform change, packaging update

**Web security hardening**:
The Web-first fork's strengthened safety boundary for remote and self-hosted operation.
_Avoid_: local patch, audit fix, security tweak

**Unauthenticated Web API**:
A deployment posture where cc-switch-web exposes its browser API without an application-layer access challenge.
_Avoid_: no login, no security, remove password

**Same-origin intent check**:
A browser-request guard that rejects cross-site mutating Web API calls without identifying the caller.
_Avoid_: authentication, login, user session
