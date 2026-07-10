---
status: accepted
---

# Use context-aware symbolic-link writes

Managed external-application configuration files may follow an existing valid file symlink and atomically replace its resolved target so dotfiles and NixOS ownership survive updates. Other write surfaces must not inherit that behavior implicitly: workspace paths must resolve inside their allowed root and reject a final symlink, because globally following one would turn an allowlisted filename into a filesystem-boundary escape. This decision covers complete-file writes only; deleting a managed path does not imply deleting its resolved target.
