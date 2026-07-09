---
status: proposed
---

# Unauthenticated Web API

The proposed direction is to remove cc-switch-web's application-layer Web API authentication and bind the server to `0.0.0.0`. This is a deliberate reversal of the current Web security hardening: the browser API can manage provider secrets, OAuth accounts, MCP command configuration, SQLite import/export, and proxy takeover, so any host that can reach the listening port becomes an operator of the cc-switch-web instance.

The proposed unauthenticated posture keeps the Web API capability surface fully open. Provider secret management, OAuth account management, MCP command configuration, database import/export, proxy takeover, and other mutating operations remain available to unauthenticated network clients rather than being downgraded to read-only or selectively disabled.

The proposed posture still retains same-origin intent checks for mutating browser requests. This is not an authentication layer: it does not identify an operator and does not block direct clients such as curl, scripts, or same-origin pages, but it does reject cross-site browser-initiated writes that could otherwise be triggered by an unrelated web page.

The installer should remove or disable the existing systemd Basic Auth drop-in instead of preserving it. Leaving `CC_SWITCH_WEB_AUTH_PASSWORD` in the user service environment would make an unauthenticated deployment look configured but behave inconsistently across fresh installs and upgrades.

Implementation should delete the Basic Auth challenge and credential code path rather than leaving it as a dormant option. `CC_SWITCH_WEB_AUTH_PASSWORD` and `CC_SWITCH_WEB_AUTH_USER` should stop being product configuration, while the same-origin intent check should remain as a separate non-authentication browser-request guard.

For remaining non-blocking design choices in this change, prefer the recommended option. Only pause for user input when the choice would create a new safety, data-loss, or irreversible product boundary that is not already covered by this ADR.
