# cc-switch-web Context

Vocabulary for the cc-switch-web product context and upstream synchronization work.

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
