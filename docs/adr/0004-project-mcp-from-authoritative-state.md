---
status: accepted
---

# Project MCP configuration from authoritative state

For cc-switch-web-managed operations, stored MCP definitions and their application assignments are authoritative; external application configuration files are derived projections that must be fully reconciled, including removal of orphan entries. Because the database and multiple external files cannot share one transaction, a projection failure restores the previous database state and compensates every application already touched, while invalid live configuration fails closed. This chooses stronger consistency and recovery over incremental best-effort writes, at the cost of explicit projection and compensation logic.
