# Symbolic-Link Write Policy Research

## Current Behavior

`config::atomic_write` creates a temporary sibling and renames it over the requested path. This preserves complete-file replacement semantics but replaces a final symbolic link with a regular file. The helper is used by application configuration, credentials, MCP files, backup files, prompts, workspace files, and Hermes memory.

The path roles are not equivalent:

- User-managed application configuration commonly uses symlinks for dotfiles, GNU Stow, chezmoi, or NixOS.
- Workspace endpoints expose allowlisted filenames; blindly following a symlink there could escape the intended filesystem boundary.
- Backup and newly created managed files do not need final-symlink following.

Therefore a global "always follow symlinks" change is unsafe.

## Comparable Patterns

- `rtk-ai/rtk`, `kenanpelit/margo`, `Markpad`, and `fallow-rs/fallow` resolve an existing symlink target and create the atomic temporary file beside the resolved target, preserving the link and same-filesystem rename.
- `tirith` takes the opposite security posture and refuses to overwrite a symlink target.
- Plain temp-plus-rename implementations replace the directory entry and therefore break final symlinks.

## Feasible Policies

### A. Context-aware managed-target following (Recommended)

- Add an explicit managed-config write mode that follows an existing, non-dangling final symlink to a regular-file target.
- Create the temporary file in the resolved target's parent, preserve target permissions, flush, and replace the target atomically.
- Reject dangling links, cycles, directory targets, and unsupported reparse targets.
- Keep default/restricted writes from silently following final symlinks. Workspace writes must reject links or resolve them only after verifying the target remains inside the allowed root.

Benefits: preserves dotfiles/NixOS workflows and atomicity without turning every allowlisted path into an arbitrary symlink-following write primitive.

Cost: call sites must declare the intended write policy; Windows reparse-point handling and failure-safe replacement need platform-specific tests.

### B. In-place write through the link

- Open/truncate the requested path and write directly, which naturally follows a normal file symlink.

Benefits: simple and preserves the link.

Cost: loses complete-file atomicity; crashes and partial writes can corrupt credentials/configuration. Not recommended.

### C. Reject all final symlinks

- Detect final symlinks and return an explicit error.

Benefits: clearest security model and retains atomic replacement for regular files.

Cost: intentionally breaks common dotfile-managed configurations and does not meet the upstream issue's compatibility expectation.

## Recommended Contract

Use A with an explicit write-policy API rather than changing the semantics of every `atomic_write` caller. Add Unix tests for relative and absolute symlinks, permission preservation, dangling links, directory targets, and containment-sensitive workspace paths. Add Windows replacement-failure tests separately because current delete-then-rename behavior is independently unsafe.

Deleting an external live configuration is a separate policy decision. This tranche does not reinterpret `delete_file` as “delete the resolved symlink target,” because unlinking a managed link and deleting its target have materially different ownership semantics.
